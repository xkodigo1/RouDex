use crate::domain::{
    EventRecord, RoutePromptInput, RouteReceipt, RouteStatus, StoredRun, StoredTask, TaskApproval,
    TaskDefinition, TaskStatus,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("invalid task transition from {from} to {to}")]
    InvalidTaskTransition { from: String, to: String },
    #[error("invalid approval decision: {0}; expected approved or denied")]
    InvalidApproval(String),
    #[error("invalid terminal event: threadId, turnId and terminal status are required")]
    InvalidTerminalEvent,
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
             CREATE INDEX IF NOT EXISTS idx_runs_session_id ON runs(session_id);
             CREATE TABLE IF NOT EXISTS tasks (
                 task_id TEXT PRIMARY KEY,
                 spec_id TEXT NOT NULL,
                 title TEXT NOT NULL,
                 prompt TEXT NOT NULL,
                 status TEXT NOT NULL,
                 profile TEXT NOT NULL,
                 model TEXT NOT NULL,
                 effort TEXT NOT NULL,
                 approval TEXT NOT NULL,
                 scope TEXT NOT NULL,
                 cwd TEXT NOT NULL,
                 sandbox_type TEXT NOT NULL,
                 writable_roots TEXT NOT NULL,
                 acceptance TEXT NOT NULL,
                 verify TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS task_dependencies (
                 task_id TEXT NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
                 depends_on TEXT NOT NULL,
                 PRIMARY KEY (task_id, depends_on)
             );
             CREATE INDEX IF NOT EXISTS idx_tasks_spec_id ON tasks(spec_id);
             CREATE TABLE IF NOT EXISTS task_approvals (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 task_id TEXT NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
                 decision TEXT NOT NULL,
                 reason TEXT,
                 created_at TEXT NOT NULL
             );",
        )?;
        Ok(())
    }

    pub fn upsert_task(
        &self,
        spec_id: &str,
        definition: &TaskDefinition,
    ) -> Result<(), StoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let now = now_string();
        transaction.execute(
            "INSERT INTO tasks (
                task_id, spec_id, title, prompt, status, profile, model, effort,
                approval, scope, cwd, sandbox_type, writable_roots, acceptance,
                verify, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
             ON CONFLICT(task_id) DO UPDATE SET
                spec_id = excluded.spec_id,
                title = excluded.title,
                prompt = excluded.prompt,
                profile = excluded.profile,
                model = excluded.model,
                effort = excluded.effort,
                approval = excluded.approval,
                scope = excluded.scope,
                cwd = excluded.cwd,
                sandbox_type = excluded.sandbox_type,
                writable_roots = excluded.writable_roots,
                acceptance = excluded.acceptance,
                verify = excluded.verify,
                updated_at = excluded.updated_at",
            params![
                definition.id,
                spec_id,
                definition.title,
                definition.prompt,
                task_status_name(&definition.status),
                definition.profile,
                definition.model,
                definition.effort,
                definition.approval,
                serde_json::to_string(&definition.scope)?,
                definition.cwd,
                definition.sandbox.kind,
                serde_json::to_string(&definition.sandbox.writable_roots)?,
                serde_json::to_string(&definition.acceptance)?,
                definition.verify,
                now,
            ],
        )?;
        transaction.execute(
            "DELETE FROM task_dependencies WHERE task_id = ?1",
            params![definition.id],
        )?;
        for dependency in &definition.depends_on {
            transaction.execute(
                "INSERT INTO task_dependencies (task_id, depends_on) VALUES (?1, ?2)",
                params![definition.id, dependency],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn get_task(&self, task_id: &str) -> Result<Option<StoredTask>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT task_id, spec_id, title, prompt, status, profile, model, effort,
                    approval, scope, cwd, sandbox_type, writable_roots, acceptance, verify
             FROM tasks WHERE task_id = ?1",
        )?;
        statement
            .query_row(params![task_id], |row| {
                let id: String = row.get(0)?;
                let spec_id: String = row.get(1)?;
                let depends_on = self.dependencies_for(&id).map_err(to_sql_error)?;
                let status: String = row.get(4)?;
                Ok(StoredTask {
                    spec_id,
                    definition: TaskDefinition {
                        id,
                        title: row.get(2)?,
                        prompt: row.get(3)?,
                        depends_on,
                        status: parse_task_status(&status).map_err(to_sql_error)?,
                        profile: row.get(5)?,
                        model: row.get(6)?,
                        effort: row.get(7)?,
                        approval: row.get(8)?,
                        scope: serde_json::from_str(&row.get::<_, String>(9)?)
                            .map_err(to_sql_error)?,
                        cwd: row.get(10)?,
                        sandbox: crate::domain::TaskSandbox {
                            kind: row.get(11)?,
                            writable_roots: serde_json::from_str(&row.get::<_, String>(12)?)
                                .map_err(to_sql_error)?,
                        },
                        acceptance: serde_json::from_str(&row.get::<_, String>(13)?)
                            .map_err(to_sql_error)?,
                        verify: row.get(14)?,
                    },
                })
            })
            .optional()
            .map_err(StoreError::from)
    }

    pub fn tasks_for_spec(&self, spec_id: &str) -> Result<Vec<TaskDefinition>, StoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT task_id FROM tasks WHERE spec_id = ?1 ORDER BY rowid")?;
        let ids = statement
            .query_map(params![spec_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| {
                self.get_task(&id)?
                    .map(|task| task.definition)
                    .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows.into())
            })
            .collect()
    }

    pub fn set_task_status(&self, task_id: &str, next: TaskStatus) -> Result<(), StoreError> {
        let current = self
            .get_task(task_id)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?
            .definition
            .status;
        if !current.can_transition_to(&next) {
            return Err(StoreError::InvalidTaskTransition {
                from: task_status_name(&current).to_owned(),
                to: task_status_name(&next).to_owned(),
            });
        }
        self.connection.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE task_id = ?3",
            params![task_status_name(&next), now_string(), task_id],
        )?;
        Ok(())
    }

    pub fn record_task_approval(&self, approval: &TaskApproval) -> Result<(), StoreError> {
        if !matches!(approval.decision.as_str(), "approved" | "denied") {
            return Err(StoreError::InvalidApproval(approval.decision.clone()));
        }
        let transaction = self.connection.unchecked_transaction()?;
        self.connection.execute(
            "INSERT INTO task_approvals (task_id, decision, reason, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                approval.task_id,
                approval.decision,
                approval.reason,
                now_string()
            ],
        )?;
        // Keep the decision and its pending-task transition in one transaction.
        // Active/terminal runs retain their state; resume rechecks the decision.
        self.connection.execute(
            "UPDATE tasks SET status = CASE WHEN ?2 = 'denied' THEN 'blocked' ELSE 'ready' END,
                updated_at = ?3 WHERE task_id = ?1
                AND (status = 'awaiting_approval' OR
                    (?2 = 'denied' AND status IN ('planned', 'ready')))",
            params![approval.task_id, approval.decision, now_string()],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn latest_task_approval(&self, task_id: &str) -> Result<Option<TaskApproval>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT task_id, decision, reason FROM task_approvals
             WHERE task_id = ?1 ORDER BY id DESC LIMIT 1",
        )?;
        Ok(statement
            .query_row(params![task_id], |row| {
                Ok(TaskApproval {
                    task_id: row.get(0)?,
                    decision: row.get(1)?,
                    reason: row.get(2)?,
                })
            })
            .optional()?)
    }

    pub fn dependencies_completed(&self, task_id: &str) -> Result<bool, StoreError> {
        let pending: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM task_dependencies d
             LEFT JOIN tasks t ON t.task_id = d.depends_on
             WHERE d.task_id = ?1 AND (t.task_id IS NULL OR t.status <> 'completed')",
            params![task_id],
            |row| row.get(0),
        )?;
        Ok(pending == 0)
    }

    pub fn update_run_status(&self, run_id: &str, status: &str) -> Result<(), StoreError> {
        self.connection.execute(
            "UPDATE runs SET status = ?1 WHERE run_id = ?2",
            params![status, run_id],
        )?;
        Ok(())
    }

    /// Claim and record a dispatch atomically so two coordinators cannot execute it.
    pub fn begin_task_run(
        &self,
        task: &TaskDefinition,
        spec_id: &str,
        receipt: &RouteReceipt,
    ) -> Result<bool, StoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let claimed = self.connection.execute(
            "UPDATE tasks SET status = 'running', updated_at = ?2 WHERE task_id = ?1
             AND status = 'ready'
             AND NOT EXISTS (SELECT 1 FROM task_dependencies d LEFT JOIN tasks t
                 ON t.task_id = d.depends_on WHERE d.task_id = ?1
                 AND (t.task_id IS NULL OR t.status <> 'completed'))
             AND (approval = 'not_required' OR
                 (SELECT decision FROM task_approvals WHERE task_id = ?1 ORDER BY id DESC LIMIT 1) = 'approved')",
            params![task.id, now_string()],
        )?;
        if claimed == 0 {
            return Ok(false);
        }
        self.record_route(
            &RoutePromptInput {
                prompt: task.prompt.clone(),
                session_id: spec_id.into(),
                turn_id: task.id.clone(),
                cwd: task.cwd.clone(),
                permission_mode: Some(task.approval.clone()),
            },
            receipt,
        )?;
        self.update_run_status(&receipt.run_id, "running")?;
        self.record_event(&EventRecord {
            run_id: receipt.run_id.clone(),
            event_type: "task.started".into(),
            payload: serde_json::json!({ "taskId": task.id, "profile": receipt.profile,
                "model": receipt.model, "effort": receipt.effort, "definition": task }),
        })?;
        transaction.commit()?;
        Ok(true)
    }

    pub fn begin_resume(&self, task_id: &str, run_id: &str) -> Result<bool, StoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let claimed = self.connection.execute(
            "UPDATE tasks SET status = 'running', updated_at = ?3 WHERE task_id = ?1
             AND status = 'interrupted' AND EXISTS (SELECT 1 FROM runs WHERE run_id = ?2
                 AND turn_id = ?1 AND status = 'interrupted')
             AND ?2 = (SELECT run_id FROM runs WHERE turn_id = ?1 ORDER BY rowid DESC LIMIT 1)
             AND NOT EXISTS (SELECT 1 FROM task_dependencies d LEFT JOIN tasks t
                 ON t.task_id = d.depends_on WHERE d.task_id = ?1
                 AND (t.task_id IS NULL OR t.status <> 'completed'))
             AND EXISTS (SELECT 1 FROM events WHERE run_id = ?2 AND event_type = 'task.started'
                 AND json_extract(payload, '$.definition.id') = ?1)
             AND ((SELECT json_extract(payload, '$.definition.approval') FROM events
                    WHERE run_id = ?2 AND event_type = 'task.started' ORDER BY id LIMIT 1) = 'not_required'
                  OR (SELECT decision FROM task_approvals WHERE task_id = ?1 ORDER BY id DESC LIMIT 1) = 'approved')",
            params![task_id, run_id, now_string()],
        )?;
        if claimed == 0 {
            return Ok(false);
        }
        self.update_run_status(run_id, "running")?;
        self.record_event(&EventRecord {
            run_id: run_id.into(),
            event_type: "task.resuming".into(),
            payload: serde_json::json!({ "taskId": task_id }),
        })?;
        transaction.commit()?;
        Ok(true)
    }

    /// Commit state and audit evidence together. A crash cannot expose a terminal
    /// run with a still-running task (or lose the reason for a failed transition).
    pub fn finish_task_run(
        &self,
        task_id: &str,
        run_id: &str,
        status: TaskStatus,
        event: Option<&EventRecord>,
    ) -> Result<(), StoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let latest = self.latest_run_for_task(task_id)?;
        if latest.as_ref().map(|run| run.run_id.as_str()) != Some(run_id) {
            return Err(rusqlite::Error::QueryReturnedNoRows.into());
        }
        self.set_task_status(task_id, status.clone())?;
        self.update_run_status(run_id, task_status_name(&status))?;
        if let Some(event) = event {
            if event.run_id != run_id {
                return Err(rusqlite::Error::InvalidParameterName("event run_id".into()).into());
            }
            self.record_event(event)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn latest_run_for_task(&self, task_id: &str) -> Result<Option<StoredRun>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT run_id, route_id, session_id, turn_id, cwd, prompt,
                    profile, model, effort, status
             FROM runs WHERE turn_id = ?1 ORDER BY rowid DESC LIMIT 1",
        )?;
        statement
            .query_row(params![task_id], |row| {
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
            .optional()
            .map_err(StoreError::from)
    }

    fn dependencies_for(&self, task_id: &str) -> Result<Vec<String>, rusqlite::Error> {
        let mut statement = self.connection.prepare(
            "SELECT depends_on FROM task_dependencies WHERE task_id = ?1 ORDER BY depends_on",
        )?;
        let dependencies = statement
            .query_map(params![task_id], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(dependencies)
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
        if matches!(
            event.event_type.as_str(),
            "turn.completed" | "turn/completed"
        ) {
            return self.record_terminal_event(event);
        }
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

    pub fn events_for_run(&self, run_id: &str) -> Result<Vec<EventRecord>, StoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT event_type, payload FROM events WHERE run_id = ?1 ORDER BY id")?;
        let events = statement
            .query_map(params![run_id], |row| {
                let payload: String = row.get(1)?;
                Ok(EventRecord {
                    run_id: run_id.into(),
                    event_type: row.get(0)?,
                    payload: serde_json::from_str(&payload).map_err(to_sql_error)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    pub fn record_terminal_event(&self, event: &EventRecord) -> Result<(), StoreError> {
        let payload = &event.payload;
        let turn_id = payload["turnId"]
            .as_str()
            .or_else(|| payload["turn"]["id"].as_str());
        let status = payload["status"]
            .as_str()
            .or_else(|| payload["turn"]["status"].as_str());
        if !payload["threadId"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
            || !turn_id.is_some_and(|id| !id.is_empty())
            || !status
                .is_some_and(|status| matches!(status, "completed" | "failed" | "interrupted"))
        {
            return Err(StoreError::InvalidTerminalEvent);
        }
        self.connection.execute(
            "INSERT INTO events (run_id, event_type, payload, created_at)
             SELECT ?1, ?2, ?3, ?4 WHERE NOT EXISTS (
                SELECT 1 FROM events WHERE run_id = ?1 AND event_type = ?2
                AND json_extract(payload, '$.threadId') = json_extract(?3, '$.threadId')
                AND COALESCE(json_extract(payload, '$.turnId'), json_extract(payload, '$.turn.id'))
                    = COALESCE(json_extract(?3, '$.turnId'), json_extract(?3, '$.turn.id')))",
            params![
                event.run_id,
                event.event_type,
                serde_json::to_string(&event.payload)?,
                now_string()
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

fn now_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn task_status_name(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Planned => "planned",
        TaskStatus::Ready => "ready",
        TaskStatus::AwaitingApproval => "awaiting_approval",
        TaskStatus::Running => "running",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Interrupted => "interrupted",
    }
}

fn parse_task_status(value: &str) -> Result<TaskStatus, serde_json::Error> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
}

fn to_sql_error<E>(error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        RoutePromptInput, RouteReceipt, RouteStatus, TaskDefinition, TaskSandbox, TaskStatus,
    };

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

    #[test]
    fn persists_tasks_dependencies_and_approvals() {
        let store = RouterStore::in_memory().expect("store");
        let task = TaskDefinition {
            id: "T-002".to_owned(),
            title: "Persist tasks".to_owned(),
            prompt: "Implement persistence".to_owned(),
            depends_on: vec!["T-001".to_owned()],
            status: TaskStatus::Ready,
            profile: "standard".to_owned(),
            model: "gpt-6.1-sol".to_owned(),
            effort: "medium".to_owned(),
            approval: "manual".to_owned(),
            scope: vec!["crates/router-core".to_owned()],
            cwd: ".".to_owned(),
            sandbox: TaskSandbox {
                kind: "workspaceWrite".to_owned(),
                writable_roots: vec!["crates/router-core".to_owned()],
            },
            acceptance: vec!["records persist".to_owned()],
            verify: "cargo test --workspace".to_owned(),
        };
        store
            .upsert_task("001-task-orchestration", &task)
            .expect("task");
        let stored = store
            .get_task("T-002")
            .expect("read task")
            .expect("task exists");
        assert_eq!(stored.definition.depends_on, vec!["T-001"]);
        store
            .record_task_approval(&TaskApproval {
                task_id: "T-002".to_owned(),
                decision: "approved".to_owned(),
                reason: Some("local test".to_owned()),
            })
            .expect("approval");
        store
            .set_task_status("T-002", TaskStatus::Running)
            .expect("running transition");
        assert_eq!(
            store
                .get_task("T-002")
                .expect("read task")
                .expect("task")
                .definition
                .status,
            TaskStatus::Running
        );
    }

    #[test]
    fn rejects_invalid_task_transition() {
        let store = RouterStore::in_memory().expect("store");
        let task = TaskDefinition {
            id: "T-003".to_owned(),
            title: "Transition".to_owned(),
            prompt: "test".to_owned(),
            depends_on: vec![],
            status: TaskStatus::Planned,
            profile: "routine".to_owned(),
            model: "gpt-6-luna".to_owned(),
            effort: "medium".to_owned(),
            approval: "not_required".to_owned(),
            scope: vec![".".to_owned()],
            cwd: ".".to_owned(),
            sandbox: TaskSandbox {
                kind: "readOnly".to_owned(),
                writable_roots: vec![],
            },
            acceptance: vec!["test".to_owned()],
            verify: "cargo test".to_owned(),
        };
        store.upsert_task("spec", &task).expect("task");
        let error = store
            .set_task_status("T-003", TaskStatus::Completed)
            .expect_err("invalid transition");
        assert!(matches!(error, StoreError::InvalidTaskTransition { .. }));
    }
}
