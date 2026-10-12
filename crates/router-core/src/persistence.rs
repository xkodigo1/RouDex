use crate::domain::{EventRecord, RoutePromptInput, RouteReceipt, RouteStatus, StoredRun};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct RouterStore {
    connection: Connection,
    path: PathBuf,
}

impl RouterStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let connection = Connection::open(&path)?;
        let store = Self { connection, path };
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self, StoreError> {
        let connection = Connection::open_in_memory()?;
        let store = Self {
            connection,
            path: PathBuf::from(":memory:"),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn default_path() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("RouDex")
            .join("router.sqlite3")
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn migrate(&self) -> Result<(), StoreError> {
        self.connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS runs (
                 run_id TEXT PRIMARY KEY,
                 route_id TEXT NOT NULL,
                 session_id TEXT NOT NULL,
                 turn_id TEXT NOT NULL,
                 cwd TEXT NOT NULL,
                 prompt TEXT NOT NULL,
                 profile TEXT NOT NULL,
                 model TEXT NOT NULL,
                 effort TEXT NOT NULL,
                 thread_id TEXT,
                 status TEXT NOT NULL,
                 created_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS events (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 run_id TEXT NOT NULL REFERENCES runs(run_id),
                 event_type TEXT NOT NULL,
                 payload TEXT NOT NULL,
                 created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_events_run_id ON events(run_id);
             CREATE INDEX IF NOT EXISTS idx_runs_session_id ON runs(session_id);",
        )?;
        Ok(())
    }

    pub fn record_route(
        &self,
        input: &RoutePromptInput,
        receipt: &RouteReceipt,
    ) -> Result<(), StoreError> {
        let status = match receipt.status {
            RouteStatus::Accepted => "accepted",
            RouteStatus::Blocked => "blocked",
            RouteStatus::Failed => "failed",
        };
        self.connection.execute(
            "INSERT INTO runs (
                run_id, route_id, session_id, turn_id, cwd, prompt,
                profile, model, effort, status, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                receipt.run_id,
                receipt.route_id,
                input.session_id,
                input.turn_id,
                input.cwd,
                input.prompt,
                receipt.profile,
                receipt.model,
                receipt.effort,
                status,
                now_string(),
            ],
        )?;
        Ok(())
    }

    pub fn record_event(&self, event: &EventRecord) -> Result<(), StoreError> {
        self.connection.execute(
            "INSERT INTO events (run_id, event_type, payload, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                event.run_id,
                event.event_type,
                serde_json::to_string(&event.payload)?,
                now_string(),
            ],
        )?;
        Ok(())
    }

    pub fn last_thread_id(&self, run_id: &str) -> Result<Option<String>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT payload FROM events
             WHERE run_id = ?1 AND event_type = 'thread.started'
             ORDER BY id DESC LIMIT 1",
        )?;
        let payload = statement
            .query_row(params![run_id], |row| row.get::<_, String>(0))
            .optional()?;
        payload
            .map(|value| {
                serde_json::from_str::<serde_json::Value>(&value).map(|json| {
                    json.get("threadId")
                        .and_then(|id| id.as_str())
                        .map(ToOwned::to_owned)
                })
            })
            .transpose()
            .map(|value| value.flatten())
            .map_err(StoreError::from)
    }

    pub fn get_run(&self, run_id: &str) -> Result<Option<StoredRun>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT run_id, route_id, session_id, turn_id, cwd, prompt,
                    profile, model, effort, status
             FROM runs WHERE run_id = ?1",
        )?;
        Ok(statement
            .query_row(params![run_id], |row| {
                Ok(StoredRun {
                    run_id: row.get(0)?,
                    route_id: row.get(1)?,
                    session_id: row.get(2)?,
                    turn_id: row.get(3)?,
                    cwd: row.get(4)?,
                    prompt: row.get(5)?,
                    profile: row.get(6)?,
                    model: row.get(7)?,
                    effort: row.get(8)?,
                    status: row.get(9)?,
                })
            })
            .optional()?)
    }

    pub fn count_runs(&self) -> Result<i64, StoreError> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM runs", [], |row| row.get(0))?)
    }
}

use rusqlite::OptionalExtension;

fn now_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{RoutePromptInput, RouteReceipt, RouteStatus};

    #[test]
    fn persists_route_and_events() {
        let store = RouterStore::in_memory().expect("store");
        let input = RoutePromptInput {
            prompt: "Implement a task".to_owned(),
            session_id: "session".to_owned(),
            turn_id: "turn".to_owned(),
            cwd: "C:/workspace".to_owned(),
            permission_mode: Some("default".to_owned()),
        };
        let receipt = RouteReceipt {
            route_id: "route".to_owned(),
            run_id: "run".to_owned(),
            profile: "standard".to_owned(),
            model: "gpt-6.1-sol".to_owned(),
            effort: "medium".to_owned(),
            status: RouteStatus::Accepted,
        };
        store.record_route(&input, &receipt).expect("route");
        store
            .record_event(&EventRecord {
                run_id: "run".to_owned(),
                event_type: "accepted".to_owned(),
                payload: serde_json::json!({"ok": true}),
            })
            .expect("event");
        assert_eq!(store.count_runs().expect("count"), 1);
    }
}
