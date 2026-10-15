//! Codex Desktop usage from its session files, for the savings estimate
//! (spec 005, D5). Optional and off by default: nothing here runs unless
//! `[savings] include_codex_desktop = true`.
//!
//! A rollout file (`$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl`) holds
//! the whole conversation. Only two kinds of line are read: `turn_context`
//! (which model a turn used) and `token_usage_record` (the tokens of each
//! request). Every other line is skipped without being parsed, and nothing
//! but the thread, turn, model, time and token counts is kept.

use crate::chat::TokenCounts;
use crate::persistence::{RouterStore, StoreError};
use crate::savings::{Usage, UsageSource, SAVINGS_DAYS};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// `$CODEX_HOME/sessions`, by default `~/.codex/sessions`.
pub fn sessions_dir() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(|home| PathBuf::from(home).join(".codex"))
        })
        .unwrap_or_else(|| PathBuf::from(".codex"))
        .join("sessions")
}

pub(crate) fn migrate(connection: &rusqlite::Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS codex_session_files (
             path TEXT PRIMARY KEY,
             size INTEGER NOT NULL,
             modified INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS codex_session_turns (
             path TEXT NOT NULL,
             thread_id TEXT NOT NULL,
             turn_id TEXT NOT NULL,
             model TEXT NOT NULL,
             created_at TEXT NOT NULL,
             input_tokens INTEGER NOT NULL,
             cached_input_tokens INTEGER NOT NULL,
             output_tokens INTEGER NOT NULL,
             reasoning_tokens INTEGER NOT NULL,
             total_tokens INTEGER NOT NULL,
             PRIMARY KEY (path, thread_id, turn_id)
         );",
    )
}

/// How many files a refresh read and how many it found unchanged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Refresh {
    pub read: usize,
    pub unchanged: usize,
}

/// One turn's totals while reading a file.
#[derive(Default)]
struct Turn {
    turn_id: String,
    thread_id: String,
    model: String,
    created_at: String,
    tokens: TokenCounts,
}

/// The top-level `timestamp` and `type` of a rollout line, read from its
/// start without parsing the line. The payload, which holds the
/// conversation, always comes after them.
fn header(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("{\"timestamp\":\"")?;
    let timestamp = &rest[..rest.find('"')?];
    let payload = line.find("\"payload\"").unwrap_or(line.len());
    let start = line[..payload].find("\"type\":\"")? + "\"type\":\"".len();
    let kind = &line[start..start + line[start..payload].find('"')?];
    Some((timestamp, kind))
}

fn read_file(path: &Path) -> std::io::Result<Vec<Turn>> {
    let mut models = HashMap::<String, String>::new();
    let mut turns = HashMap::<String, Turn>::new();
    let mut order = Vec::new();
    for line in BufReader::new(std::fs::File::open(path)?).lines() {
        let line = line?;
        let Some((timestamp, kind)) = header(&line) else {
            continue;
        };
        match kind {
            "turn_context" => {
                let parsed: Value = serde_json::from_str(&line).unwrap_or_default();
                let payload = &parsed["payload"];
                if let (Some(turn), Some(model)) =
                    (payload["turn_id"].as_str(), payload["model"].as_str())
                {
                    models.insert(turn.to_owned(), model.to_owned());
                }
            }
            "token_usage_record" => {
                let parsed: Value = serde_json::from_str(&line).unwrap_or_default();
                let payload = &parsed["payload"];
                let (Some(thread), Some(turn)) =
                    (payload["thread_id"].as_str(), payload["turn_id"].as_str())
                else {
                    continue;
                };
                let usage = &payload["usage"];
                let field = |key: &str| usage[key].as_i64().unwrap_or(0);
                let entry = turns.entry(turn.to_owned()).or_insert_with(|| {
                    order.push(turn.to_owned());
                    Turn {
                        turn_id: turn.to_owned(),
                        thread_id: thread.to_owned(),
                        created_at: timestamp.to_owned(),
                        ..Turn::default()
                    }
                });
                entry.tokens.input += field("input_tokens");
                entry.tokens.cached_input += field("cached_input_tokens");
                entry.tokens.output += field("output_tokens");
                entry.tokens.reasoning += field("reasoning_output_tokens");
                entry.tokens.total += field("total_tokens");
            }
            _ => {}
        }
    }
    Ok(order
        .into_iter()
        .filter_map(|turn| {
            let mut found = turns.remove(&turn)?;
            found.model = models.get(&turn)?.clone();
            Some(found)
        })
        .collect())
}

/// Session files of the last [`SAVINGS_DAYS`] days (and one more, for time
/// zones), from their `YYYY/MM/DD` folders.
fn recent_files(dir: &Path, now: SystemTime) -> Vec<PathBuf> {
    let since = now
        .checked_sub(Duration::from_secs(u64::from(SAVINGS_DAYS + 1) * 86_400))
        .unwrap_or(UNIX_EPOCH);
    let since = days_since_epoch(since);
    let mut files = Vec::new();
    let children = |path: &Path| -> Vec<(u32, PathBuf)> {
        std::fs::read_dir(path)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let number = entry.file_name().to_str()?.parse().ok()?;
                Some((number, entry.path()))
            })
            .collect()
    };
    for (year, year_dir) in children(dir) {
        for (month, month_dir) in children(&year_dir) {
            for (day, day_dir) in children(&month_dir) {
                if civil_days(year, month, day) < since {
                    continue;
                }
                files.extend(
                    std::fs::read_dir(&day_dir)
                        .into_iter()
                        .flatten()
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.extension()
                                .is_some_and(|extension| extension == "jsonl")
                        }),
                );
            }
        }
    }
    files
}

fn days_since_epoch(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|elapsed| (elapsed.as_secs() / 86_400) as i64)
        .unwrap_or(0)
}

/// Days from 1970-01-01 to a civil date (Howard Hinnant's algorithm).
fn civil_days(year: u32, month: u32, day: u32) -> i64 {
    let (year, month, day) = (i64::from(year), i64::from(month), i64::from(day));
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

impl RouterStore {
    /// Read the session files that are new or changed since the last refresh.
    pub fn refresh_codex_sessions(
        &self,
        dir: &Path,
        now: SystemTime,
    ) -> Result<Refresh, StoreError> {
        let mut refresh = Refresh::default();
        for path in recent_files(dir, now) {
            let Ok(metadata) = path.metadata() else {
                continue;
            };
            let size = metadata.len() as i64;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |elapsed| elapsed.as_secs() as i64);
            let key = path.to_string_lossy().into_owned();
            let known: Option<(i64, i64)> = self
                .connection
                .query_row(
                    "SELECT size, modified FROM codex_session_files WHERE path = ?1",
                    [&key],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();
            if known == Some((size, modified)) {
                refresh.unchanged += 1;
                continue;
            }
            // A file that cannot be read now is retried on the next refresh.
            let Ok(turns) = read_file(&path) else {
                continue;
            };
            let transaction = self.connection.unchecked_transaction()?;
            transaction.execute("DELETE FROM codex_session_turns WHERE path = ?1", [&key])?;
            for turn in &turns {
                let tokens = &turn.tokens;
                transaction.execute(
                    "INSERT OR REPLACE INTO codex_session_turns (path, thread_id, turn_id, model,
                         created_at, input_tokens, cached_input_tokens, output_tokens,
                         reasoning_tokens, total_tokens)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        key,
                        turn.thread_id,
                        turn.turn_id,
                        turn.model,
                        turn.created_at,
                        tokens.input,
                        tokens.cached_input,
                        tokens.output,
                        tokens.reasoning,
                        tokens.total
                    ],
                )?;
            }
            transaction.execute(
                "INSERT OR REPLACE INTO codex_session_files (path, size, modified) VALUES (?1, ?2, ?3)",
                rusqlite::params![key, size, modified],
            )?;
            transaction.commit()?;
            refresh.read += 1;
        }
        Ok(refresh)
    }

    /// Codex Desktop turns of the last [`SAVINGS_DAYS`] days, leaving out the
    /// threads RouDex runs itself (its chats and task runs), which its own
    /// records already count.
    pub(crate) fn codex_desktop_usage(&self) -> Result<Vec<Usage>, StoreError> {
        let day = "date(created_at, 'localtime')";
        let mut statement = self.connection.prepare(&format!(
            "SELECT {day}, model, input_tokens, cached_input_tokens, output_tokens,
                    reasoning_tokens, total_tokens
             FROM codex_session_turns
             WHERE {day} > date('now', 'localtime', '-{SAVINGS_DAYS} days')
               AND total_tokens > 0
               AND thread_id NOT IN (SELECT thread_id FROM chat_threads)
               AND thread_id NOT IN (SELECT thread_id FROM runs WHERE thread_id IS NOT NULL)"
        ))?;
        let rows = statement
            .query_map([], |row| {
                Ok(Usage {
                    source: UsageSource::CodexDesktop,
                    day: row.get(0)?,
                    model: row.get(1)?,
                    tier: None,
                    tokens: TokenCounts {
                        input: row.get(2)?,
                        cached_input: row.get(3)?,
                        output: row.get(4)?,
                        reasoning: row.get(5)?,
                        total: row.get(6)?,
                    },
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "mi-clave-de-produccion-1234";

    struct Sessions(PathBuf);

    impl Sessions {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("roudex-sessions-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        /// A rollout file for today with one turn per `(thread, turn, model)`.
        fn write(&self, name: &str, turns: &[(&str, &str, &str)]) -> PathBuf {
            let now = SystemTime::now();
            let (year, month, day) = civil_from_days(days_since_epoch(now));
            let dir = self.0.join(format!("{year:04}/{month:02}/{day:02}"));
            std::fs::create_dir_all(&dir).unwrap();
            let stamp = format!("{year:04}-{month:02}-{day:02}T12:00:00.000Z");
            let mut lines = vec![format!(
                r#"{{"timestamp":"{stamp}","ordinal":0,"type":"session_meta","payload":{{"originator":"Codex Desktop"}}}}"#
            )];
            for (index, (thread, turn, model)) in turns.iter().enumerate() {
                lines.push(format!(
                    r#"{{"timestamp":"{stamp}","ordinal":{index},"type":"turn_context","payload":{{"turn_id":"{turn}","model":"{model}","cwd":"C:/secret"}}}}"#
                ));
                // The conversation, which must never be kept.
                lines.push(format!(
                    r#"{{"timestamp":"{stamp}","ordinal":{index},"type":"response_item","payload":{{"type":"message","content":"{SECRET} \"type\":\"token_usage_record\""}}}}"#
                ));
                for _ in 0..2 {
                    lines.push(format!(
                        r#"{{"timestamp":"{stamp}","ordinal":{index},"type":"token_usage_record","payload":{{"thread_id":"{thread}","turn_id":"{turn}","usage":{{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":50,"reasoning_output_tokens":5,"total_tokens":1050}}}}}}"#
                    ));
                }
            }
            let path = dir.join(format!("rollout-{name}.jsonl"));
            std::fs::write(&path, lines.join("\n") + "\n").unwrap();
            path
        }
    }

    impl Drop for Sessions {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Civil date of a day count (inverse of [`civil_days`]).
    fn civil_from_days(days: i64) -> (i64, i64, i64) {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        (yoe + era * 400 + i64::from(month <= 2), month, day)
    }

    #[test]
    fn lines_are_told_apart_by_their_start_only() {
        let line = r#"{"timestamp":"2026-10-14T12:00:00Z","ordinal":3,"type":"turn_context","payload":{"type":"x"}}"#;
        assert_eq!(header(line), Some(("2026-10-14T12:00:00Z", "turn_context")));
        let message = r#"{"timestamp":"t","ordinal":1,"type":"response_item","payload":{"content":"\"type\":\"token_usage_record\""}}"#;
        assert_eq!(header(message).unwrap().1, "response_item");
        assert_eq!(header("not json"), None);
        assert_eq!(civil_days(1970, 1, 1), 0);
        assert_eq!(civil_days(2000, 3, 1), 11_017);
        assert_eq!(civil_from_days(civil_days(2026, 10, 14)), (2026, 10, 14));
    }

    #[test]
    fn desktop_turns_count_once_without_keeping_any_text() {
        let sessions = Sessions::new();
        let file = sessions.write(
            "a",
            &[
                ("desktop-thread", "turn-1", "gpt-6-sol"),
                ("roudex-thread", "turn-2", "gpt-6-luna"),
            ],
        );
        sessions.write("b", &[("other-thread", "turn-3", "gpt-6-luna")]);
        let store = RouterStore::in_memory().unwrap();
        store
            .connection
            .execute(
                "INSERT INTO chat_threads (thread_id, cwd, title, created_at, updated_at)
                 VALUES ('roudex-thread', '.', 't', '0', '0')",
                [],
            )
            .unwrap();

        let refresh = store
            .refresh_codex_sessions(&sessions.0, SystemTime::now())
            .unwrap();
        assert_eq!(
            refresh,
            Refresh {
                read: 2,
                unchanged: 0
            }
        );
        let usage = store.codex_desktop_usage().unwrap();
        assert_eq!(usage.len(), 2, "the RouDex thread is not counted twice");
        let desktop = usage.iter().find(|turn| turn.model == "gpt-6-sol").unwrap();
        assert_eq!(desktop.source, UsageSource::CodexDesktop);
        assert_eq!(
            (
                desktop.tokens.input,
                desktop.tokens.cached_input,
                desktop.tokens.output
            ),
            (2000, 800, 100),
            "every request of the turn adds up"
        );

        let dump: Vec<String> = store
            .connection
            .prepare("SELECT path || thread_id || turn_id || model || created_at FROM codex_session_turns")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(dump
            .iter()
            .all(|row| !row.contains(SECRET) && !row.contains("C:/secret")));

        let again = store
            .refresh_codex_sessions(&sessions.0, SystemTime::now())
            .unwrap();
        assert_eq!(
            again,
            Refresh {
                read: 0,
                unchanged: 2
            },
            "unchanged files are skipped"
        );
        let mut contents = std::fs::read_to_string(&file).unwrap();
        contents.push('\n');
        std::fs::write(&file, contents).unwrap();
        let changed = store
            .refresh_codex_sessions(&sessions.0, SystemTime::now())
            .unwrap();
        assert_eq!(changed.read, 1);
        assert_eq!(
            store.codex_desktop_usage().unwrap().len(),
            2,
            "a re-read replaces its rows"
        );
    }

    #[test]
    fn the_summary_reads_sessions_only_when_asked() {
        let sessions = Sessions::new();
        sessions.write("a", &[("desktop-thread", "turn-1", "gpt-6-sol")]);
        let store = RouterStore::in_memory().unwrap();
        let config = crate::tiers::RouterConfig::default();
        let files = || -> i64 {
            store
                .connection
                .query_row("SELECT count(*) FROM codex_session_files", [], |row| {
                    row.get(0)
                })
                .unwrap()
        };

        let off = store.savings(&config, None).unwrap();
        assert_eq!(files(), 0, "no session file is opened");
        assert!(off.sources.is_empty());

        let on = store.savings(&config, Some(&sessions.0)).unwrap();
        assert_eq!(files(), 1);
        assert_eq!(on.sources[0].source, Some(UsageSource::CodexDesktop));
        assert_eq!(on.sources[0].turns, 1);
    }

    #[test]
    fn old_folders_are_not_read() {
        let sessions = Sessions::new();
        let old = sessions.0.join("2020/01/01");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("rollout-old.jsonl"), "").unwrap();
        assert!(recent_files(&sessions.0, SystemTime::now()).is_empty());
    }
}
