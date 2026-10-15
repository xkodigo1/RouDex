//! Read models for the desktop app: the summary dashboard and thread lists.

use crate::chat::{ModelUsage, TokenCounts};
use crate::persistence::{RouterStore, StoreError};
use crate::session::AccessMode;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Tokens and turns of one local calendar day (`YYYY-MM-DD`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayUsage {
    pub date: String,
    pub tokens: i64,
    pub turns: i64,
}

/// Turns per tier, split by what decided the tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierUsage {
    pub tier: String,
    pub turns: i64,
    pub by_source: Vec<(String, i64)>,
}

/// One thread with the figures the thread list shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub thread_id: String,
    pub cwd: String,
    pub title: String,
    pub access: AccessMode,
    pub created_at: String,
    pub updated_at: String,
    pub archived: bool,
    pub turns: i64,
    pub tokens: i64,
    pub last_status: Option<String>,
    pub last_tier: Option<String>,
    pub last_model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dashboard {
    /// Days with activity in the last 365 days, oldest first; days without
    /// turns are omitted.
    pub days: Vec<DayUsage>,
    pub year: TokenCounts,
    pub today_tokens: i64,
    pub today_turns: i64,
    pub today_threads: i64,
    pub total_turns: i64,
    /// Consecutive days with turns, ending today (or yesterday when today
    /// has none yet).
    pub streak_days: i64,
    pub models: Vec<ModelUsage>,
    pub tiers: Vec<TierUsage>,
    pub recent: Vec<ThreadSummary>,
}

impl RouterStore {
    /// Threads, newest first. Archived ones only when asked for.
    pub fn thread_summaries(
        &self,
        cwd: Option<&Path>,
        include_archived: bool,
        limit: Option<i64>,
    ) -> Result<Vec<ThreadSummary>, StoreError> {
        let cwd = cwd.map(|path| path.to_string_lossy().into_owned());
        let mut statement = self.connection.prepare(
            "SELECT t.thread_id, t.cwd, t.title, t.access, t.created_at, t.updated_at, t.archived,
                count(u.turn_id), coalesce(sum(u.total_tokens), 0),
                (SELECT status FROM chat_turns l WHERE l.thread_id = t.thread_id ORDER BY l.rowid DESC LIMIT 1),
                (SELECT tier FROM chat_turns l WHERE l.thread_id = t.thread_id ORDER BY l.rowid DESC LIMIT 1),
                (SELECT model FROM chat_turns l WHERE l.thread_id = t.thread_id ORDER BY l.rowid DESC LIMIT 1)
             FROM chat_threads t LEFT JOIN chat_turns u ON u.thread_id = t.thread_id
             WHERE (?1 IS NULL OR t.cwd = ?1) AND (?2 OR t.archived = 0)
             GROUP BY t.thread_id
             ORDER BY CAST(t.updated_at AS INTEGER) DESC, t.rowid DESC
             LIMIT ?3",
        )?;
        let rows = statement
            .query_map(params![cwd, include_archived, limit.unwrap_or(-1)], |row| {
                Ok(ThreadSummary {
                    thread_id: row.get(0)?,
                    cwd: row.get(1)?,
                    title: row.get(2)?,
                    access: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    archived: row.get(6)?,
                    turns: row.get(7)?,
                    tokens: row.get(8)?,
                    last_status: row.get(9)?,
                    last_tier: row.get(10)?,
                    last_model: row.get(11)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Distinct project folders that have threads, most recently used first.
    pub fn chat_projects(&self) -> Result<Vec<String>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT cwd FROM chat_threads GROUP BY cwd
             ORDER BY max(CAST(updated_at AS INTEGER)) DESC",
        )?;
        let rows = statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn rename_chat_thread(&self, thread_id: &str, title: &str) -> Result<bool, StoreError> {
        let title = title.trim();
        if title.is_empty() {
            return Ok(false);
        }
        Ok(self.connection.execute(
            "UPDATE chat_threads SET title = ?2 WHERE thread_id = ?1",
            params![thread_id, title],
        )? == 1)
    }

    pub fn archive_chat_thread(&self, thread_id: &str, archived: bool) -> Result<bool, StoreError> {
        Ok(self.connection.execute(
            "UPDATE chat_threads SET archived = ?2 WHERE thread_id = ?1",
            params![thread_id, archived],
        )? == 1)
    }

    /// Everything the summary screen shows. Dates use the local calendar.
    pub fn dashboard(&self) -> Result<Dashboard, StoreError> {
        const DAY: &str = "date(CAST(created_at AS INTEGER), 'unixepoch', 'localtime')";
        let mut statement = self.connection.prepare(&format!(
            "SELECT {DAY} AS day, sum(total_tokens), count(*) FROM chat_turns
             WHERE {DAY} > date('now', 'localtime', '-365 days')
             GROUP BY day ORDER BY day"
        ))?;
        let days: Vec<DayUsage> = statement
            .query_map([], |row| {
                Ok(DayUsage {
                    date: row.get(0)?,
                    tokens: row.get(1)?,
                    turns: row.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        let year = self.connection.query_row(
            &format!(
                "SELECT coalesce(sum(input_tokens), 0), coalesce(sum(cached_input_tokens), 0),
                    coalesce(sum(output_tokens), 0), coalesce(sum(reasoning_tokens), 0),
                    coalesce(sum(total_tokens), 0)
                 FROM chat_turns WHERE {DAY} > date('now', 'localtime', '-365 days')"
            ),
            [],
            |row| {
                Ok(TokenCounts {
                    input: row.get(0)?,
                    cached_input: row.get(1)?,
                    output: row.get(2)?,
                    reasoning: row.get(3)?,
                    total: row.get(4)?,
                })
            },
        )?;

        let (today_tokens, today_turns, today_threads): (i64, i64, i64) =
            self.connection.query_row(
                &format!(
                    "SELECT coalesce(sum(total_tokens), 0), count(*), count(DISTINCT thread_id)
                 FROM chat_turns WHERE {DAY} = date('now', 'localtime')"
                ),
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        let total_turns: i64 =
            self.connection
                .query_row("SELECT count(*) FROM chat_turns", [], |row| row.get(0))?;
        let streak_days = self.streak()?;

        let mut statement = self.connection.prepare(
            "SELECT tier, source, count(*) FROM chat_turns GROUP BY tier, source ORDER BY tier, source",
        )?;
        let mut tiers: Vec<TierUsage> = Vec::new();
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (tier, source, count) = row?;
            match tiers.iter_mut().find(|entry| entry.tier == tier) {
                Some(entry) => {
                    entry.turns += count;
                    entry.by_source.push((source, count));
                }
                None => tiers.push(TierUsage {
                    tier,
                    turns: count,
                    by_source: vec![(source, count)],
                }),
            }
        }

        Ok(Dashboard {
            days,
            year,
            today_tokens,
            today_turns,
            today_threads,
            total_turns,
            streak_days,
            models: self.usage_by_model()?,
            tiers,
            recent: self.thread_summaries(None, false, Some(5))?,
        })
    }

    fn streak(&self) -> Result<i64, StoreError> {
        // Day offsets from today (0 = today) that have turns, nearest first.
        let mut statement = self.connection.prepare(
            "SELECT DISTINCT CAST(julianday(date('now', 'localtime'))
                - julianday(date(CAST(created_at AS INTEGER), 'unixepoch', 'localtime')) AS INTEGER) AS ago
             FROM chat_turns ORDER BY ago",
        )?;
        let offsets: Vec<i64> = statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        let first = match offsets.first() {
            Some(&day @ (0 | 1)) => day,
            _ => return Ok(0),
        };
        // Count offsets that continue first, first + 1, first + 2…
        let streak = offsets
            .iter()
            .zip(first..)
            .take_while(|(offset, expected)| **offset == *expected)
            .count();
        Ok(streak as i64)
    }
}

#[cfg(test)]
mod tests {
    use crate::classifier::{DecisionSource, RouteDecision};
    use crate::tiers::Tier;
    use crate::{AccessMode, RouterStore, TokenCounts};
    use std::path::Path;

    fn decision(tier: Tier, model: &str, source: DecisionSource) -> RouteDecision {
        RouteDecision {
            tier,
            model: model.into(),
            effort: "medium".into(),
            source,
            reason: "test".into(),
            risk: vec![],
            router_usage: None,
        }
    }

    fn turn(
        store: &RouterStore,
        thread: &str,
        turn: &str,
        days_ago: i64,
        decision: &RouteDecision,
        tokens: i64,
    ) {
        store
            .insert_chat_turn(turn, thread, "prompt", decision)
            .unwrap();
        store
            .set_chat_turn_tokens(
                turn,
                TokenCounts {
                    input: tokens - 10,
                    cached_input: tokens / 2,
                    output: 10,
                    reasoning: 0,
                    total: tokens,
                },
            )
            .unwrap();
        store
            .finish_chat_turn(turn, "completed", "ok", None)
            .unwrap();
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - days_ago * 86_400;
        store
            .connection
            .execute(
                "UPDATE chat_turns SET created_at = ?2 WHERE turn_id = ?1",
                rusqlite::params![turn, created.to_string()],
            )
            .unwrap();
    }

    fn seeded() -> RouterStore {
        let store = RouterStore::in_memory().unwrap();
        store
            .upsert_chat_thread("a", Path::new("C:/p/api"), "Hilo A", AccessMode::Write)
            .unwrap();
        store
            .upsert_chat_thread(
                "b",
                Path::new("C:/p/roudex"),
                "Hilo B",
                AccessMode::ReadOnly,
            )
            .unwrap();
        let sol = decision(Tier::Fuerte, "gpt-6.1-sol", DecisionSource::Risk);
        let luna = decision(Tier::Economico, "gpt-6-luna", DecisionSource::Rule);
        let model = decision(Tier::Economico, "gpt-6-luna", DecisionSource::Model);
        turn(&store, "a", "t1", 0, &sol, 1000);
        turn(&store, "a", "t2", 1, &luna, 200);
        turn(&store, "b", "t3", 2, &model, 300);
        turn(&store, "b", "t4", 5, &luna, 400);
        turn(&store, "b", "t5", 400, &luna, 9999);
        store
    }

    #[test]
    fn dashboard_aggregates_days_totals_streak_models_and_tiers() {
        let dashboard = seeded().dashboard().unwrap();
        assert_eq!(
            dashboard.days.len(),
            4,
            "the 400-day-old turn is outside the year"
        );
        assert_eq!(dashboard.days.last().unwrap().tokens, 1000);
        assert_eq!(dashboard.year.total, 1900);
        assert_eq!(
            (
                dashboard.today_tokens,
                dashboard.today_turns,
                dashboard.today_threads
            ),
            (1000, 1, 1)
        );
        assert_eq!(dashboard.total_turns, 5);
        assert_eq!(dashboard.streak_days, 3);
        assert_eq!(dashboard.models[0].model, "gpt-6-luna");
        let economico = dashboard
            .tiers
            .iter()
            .find(|t| t.tier == "economico")
            .unwrap();
        assert_eq!(economico.turns, 4);
        assert!(economico.by_source.contains(&("rule".into(), 3)));
        assert!(economico.by_source.contains(&("model".into(), 1)));
        assert_eq!(dashboard.recent.len(), 2);
    }

    #[test]
    fn thread_summaries_filter_by_project_and_hide_archived() {
        let store = seeded();
        let all = store.thread_summaries(None, false, None).unwrap();
        assert_eq!(all.len(), 2);
        let api = store
            .thread_summaries(Some(Path::new("C:/p/api")), false, None)
            .unwrap();
        assert_eq!(api.len(), 1);
        assert_eq!((api[0].turns, api[0].tokens), (2, 1200));
        assert_eq!(api[0].last_tier.as_deref(), Some("economico"));
        assert_eq!(api[0].access, AccessMode::Write);
        let roudex = store
            .thread_summaries(Some(Path::new("C:/p/roudex")), false, None)
            .unwrap();
        assert_eq!(roudex[0].access, AccessMode::ReadOnly);
        assert!(store.archive_chat_thread("a", true).unwrap());
        assert_eq!(store.thread_summaries(None, false, None).unwrap().len(), 1);
        assert_eq!(store.thread_summaries(None, true, None).unwrap().len(), 2);
        assert!(store.rename_chat_thread("b", "  Nuevo nombre ").unwrap());
        assert!(!store.rename_chat_thread("b", "   ").unwrap());
        let b = store.thread_summaries(None, false, None).unwrap();
        assert_eq!(b[0].title, "Nuevo nombre");
        let mut projects = store.chat_projects().unwrap();
        projects.sort();
        assert_eq!(projects, ["C:/p/api", "C:/p/roudex"]);
    }

    #[test]
    fn streak_is_zero_without_recent_activity() {
        let store = RouterStore::in_memory().unwrap();
        assert_eq!(store.dashboard().unwrap().streak_days, 0);
        store
            .upsert_chat_thread("a", Path::new("C:/p"), "A", AccessMode::Write)
            .unwrap();
        let luna = decision(Tier::Economico, "gpt-6-luna", DecisionSource::Rule);
        turn(&store, "a", "old", 3, &luna, 10);
        assert_eq!(store.dashboard().unwrap().streak_days, 0);
    }
}

#[cfg(test)]
mod migration_tests {
    use crate::{AccessMode, RouterStore};
    use std::path::Path;

    #[test]
    fn opening_an_older_store_adds_the_archived_column() {
        let path =
            std::env::temp_dir().join(format!("roudex-migrate-{}.sqlite3", uuid::Uuid::new_v4()));
        {
            let old = rusqlite::Connection::open(&path).unwrap();
            old.execute_batch(
                "CREATE TABLE chat_threads (thread_id TEXT PRIMARY KEY, cwd TEXT NOT NULL,
                    title TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
                 INSERT INTO chat_threads VALUES ('old', 'C:/p', 'Antiguo', '1', '1');",
            )
            .unwrap();
        }
        let store = RouterStore::open(&path).unwrap();
        let threads = store
            .thread_summaries(Some(Path::new("C:/p")), false, None)
            .unwrap();
        assert_eq!(threads.len(), 1);
        assert!(!threads[0].archived);
        assert_eq!(threads[0].access, AccessMode::Write);
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn opening_a_store_with_archived_threads_adds_write_access() {
        let path =
            std::env::temp_dir().join(format!("roudex-migrate-{}.sqlite3", uuid::Uuid::new_v4()));
        {
            let old = rusqlite::Connection::open(&path).unwrap();
            old.execute_batch(
                "CREATE TABLE chat_threads (thread_id TEXT PRIMARY KEY, cwd TEXT NOT NULL,
                    title TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
                    archived INTEGER NOT NULL DEFAULT 0);
                 INSERT INTO chat_threads VALUES ('old', 'C:/p', 'Antiguo', '1', '1', 1);",
            )
            .unwrap();
        }
        // Opening twice must not try to add the column again.
        drop(RouterStore::open(&path).unwrap());
        let store = RouterStore::open(&path).unwrap();
        let threads = store
            .thread_summaries(Some(Path::new("C:/p")), true, None)
            .unwrap();
        assert_eq!(threads.len(), 1);
        assert!(threads[0].archived);
        assert_eq!(threads[0].access, AccessMode::Write);
        assert_eq!(
            store.chat_thread("old").unwrap().unwrap().access,
            AccessMode::Write
        );
        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
