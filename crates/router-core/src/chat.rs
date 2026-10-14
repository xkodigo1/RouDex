//! Chat threads and turns: persistence, event recording and one app-server
//! session per project.

use crate::app_server::AppServerConfig;
use crate::classifier::RouteDecision;
use crate::persistence::{now_string, RouterStore, StoreError};
use crate::session::{ApprovalDecision, EventSink, Session, SessionError, SessionEvent};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub(crate) fn migrate(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS chat_threads (
             thread_id TEXT PRIMARY KEY,
             cwd TEXT NOT NULL,
             title TEXT NOT NULL,
             created_at TEXT NOT NULL,
             updated_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_chat_threads_cwd ON chat_threads(cwd);
         CREATE TABLE IF NOT EXISTS chat_turns (
             turn_id TEXT PRIMARY KEY,
             thread_id TEXT NOT NULL REFERENCES chat_threads(thread_id),
             prompt TEXT NOT NULL,
             tier TEXT NOT NULL,
             model TEXT NOT NULL,
             effort TEXT NOT NULL,
             source TEXT NOT NULL,
             reason TEXT NOT NULL,
             status TEXT NOT NULL,
             response TEXT NOT NULL DEFAULT '',
             error TEXT,
             input_tokens INTEGER NOT NULL DEFAULT 0,
             cached_input_tokens INTEGER NOT NULL DEFAULT 0,
             output_tokens INTEGER NOT NULL DEFAULT 0,
             reasoning_tokens INTEGER NOT NULL DEFAULT 0,
             total_tokens INTEGER NOT NULL DEFAULT 0,
             created_at TEXT NOT NULL,
             completed_at TEXT
         );
         CREATE INDEX IF NOT EXISTS idx_chat_turns_thread ON chat_turns(thread_id);",
    )?;
    // Columns added after the first release of the chat tables.
    let has_archived: bool = connection.query_row(
        "SELECT count(*) > 0 FROM pragma_table_info('chat_threads') WHERE name = 'archived'",
        [],
        |row| row.get(0),
    )?;
    if !has_archived {
        connection.execute(
            "ALTER TABLE chat_threads ADD COLUMN archived INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatThread {
    pub thread_id: String,
    pub cwd: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenCounts {
    pub input: i64,
    pub cached_input: i64,
    pub output: i64,
    pub reasoning: i64,
    pub total: i64,
}

impl TokenCounts {
    /// Parse a `TokenUsageBreakdown` object from the protocol.
    pub fn from_breakdown(value: &Value) -> Self {
        let field = |key: &str| value[key].as_i64().unwrap_or(0);
        Self {
            input: field("inputTokens"),
            cached_input: field("cachedInputTokens"),
            output: field("outputTokens"),
            reasoning: field("reasoningOutputTokens"),
            total: field("totalTokens"),
        }
    }

    fn minus(self, other: Self) -> Option<Self> {
        let delta = Self {
            input: self.input - other.input,
            cached_input: self.cached_input - other.cached_input,
            output: self.output - other.output,
            reasoning: self.reasoning - other.reasoning,
            total: self.total - other.total,
        };
        (delta.total >= 0 && delta.input >= 0 && delta.output >= 0).then_some(delta)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatTurn {
    pub turn_id: String,
    pub thread_id: String,
    pub prompt: String,
    pub tier: String,
    pub model: String,
    pub effort: String,
    pub source: String,
    pub reason: String,
    pub status: String,
    pub response: String,
    pub error: Option<String>,
    pub tokens: TokenCounts,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub turns: i64,
    pub tokens: TokenCounts,
}

fn json_name<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default()
}

impl RouterStore {
    pub fn upsert_chat_thread(
        &self,
        thread_id: &str,
        cwd: &Path,
        title: &str,
    ) -> Result<(), StoreError> {
        let now = now_string();
        self.connection.execute(
            "INSERT INTO chat_threads (thread_id, cwd, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(thread_id) DO UPDATE SET updated_at = excluded.updated_at",
            params![thread_id, cwd.to_string_lossy(), title, now],
        )?;
        Ok(())
    }

    pub fn insert_chat_turn(
        &self,
        turn_id: &str,
        thread_id: &str,
        prompt: &str,
        decision: &RouteDecision,
    ) -> Result<(), StoreError> {
        let now = now_string();
        self.connection.execute(
            "INSERT INTO chat_turns (turn_id, thread_id, prompt, tier, model, effort, source,
                reason, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'running', ?9)",
            params![
                turn_id,
                thread_id,
                prompt,
                decision.tier.as_str(),
                decision.model,
                decision.effort,
                json_name(&decision.source),
                decision.reason,
                now
            ],
        )?;
        self.connection.execute(
            "UPDATE chat_threads SET updated_at = ?2 WHERE thread_id = ?1",
            params![thread_id, now],
        )?;
        Ok(())
    }

    pub fn finish_chat_turn(
        &self,
        turn_id: &str,
        status: &str,
        response: &str,
        error: Option<&str>,
    ) -> Result<(), StoreError> {
        self.connection.execute(
            "UPDATE chat_turns SET status = ?2, response = ?3, error = ?4, completed_at = ?5
             WHERE turn_id = ?1",
            params![turn_id, status, response, error, now_string()],
        )?;
        Ok(())
    }

    pub fn set_chat_turn_tokens(
        &self,
        turn_id: &str,
        tokens: TokenCounts,
    ) -> Result<(), StoreError> {
        self.connection.execute(
            "UPDATE chat_turns SET input_tokens = ?2, cached_input_tokens = ?3,
                output_tokens = ?4, reasoning_tokens = ?5, total_tokens = ?6
             WHERE turn_id = ?1",
            params![
                turn_id,
                tokens.input,
                tokens.cached_input,
                tokens.output,
                tokens.reasoning,
                tokens.total
            ],
        )?;
        Ok(())
    }

    /// Tokens of every other recorded turn in the thread, used as the baseline
    /// for the cumulative thread totals reported by the runtime.
    fn thread_tokens_excluding(
        &self,
        thread_id: &str,
        turn_id: &str,
    ) -> Result<TokenCounts, StoreError> {
        Ok(self.connection.query_row(
            "SELECT coalesce(sum(input_tokens), 0), coalesce(sum(cached_input_tokens), 0),
                coalesce(sum(output_tokens), 0), coalesce(sum(reasoning_tokens), 0),
                coalesce(sum(total_tokens), 0)
             FROM chat_turns WHERE thread_id = ?1 AND turn_id <> ?2",
            params![thread_id, turn_id],
            |row| {
                Ok(TokenCounts {
                    input: row.get(0)?,
                    cached_input: row.get(1)?,
                    output: row.get(2)?,
                    reasoning: row.get(3)?,
                    total: row.get(4)?,
                })
            },
        )?)
    }

    pub fn chat_threads(&self, cwd: Option<&Path>) -> Result<Vec<ChatThread>, StoreError> {
        let cwd = cwd.map(|path| path.to_string_lossy().into_owned());
        let mut statement = self.connection.prepare(
            "SELECT thread_id, cwd, title, created_at, updated_at FROM chat_threads
             WHERE ?1 IS NULL OR cwd = ?1 ORDER BY CAST(updated_at AS INTEGER) DESC, rowid DESC",
        )?;
        let threads = statement
            .query_map(params![cwd], |row| {
                Ok(ChatThread {
                    thread_id: row.get(0)?,
                    cwd: row.get(1)?,
                    title: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(threads)
    }

    pub fn chat_thread(&self, thread_id: &str) -> Result<Option<ChatThread>, StoreError> {
        Ok(self
            .chat_threads(None)?
            .into_iter()
            .find(|t| t.thread_id == thread_id))
    }

    pub fn chat_turns(&self, thread_id: &str) -> Result<Vec<ChatTurn>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT turn_id, thread_id, prompt, tier, model, effort, source, reason, status,
                response, error, input_tokens, cached_input_tokens, output_tokens,
                reasoning_tokens, total_tokens, created_at, completed_at
             FROM chat_turns WHERE thread_id = ?1 ORDER BY rowid",
        )?;
        let turns = statement
            .query_map(params![thread_id], |row| {
                Ok(ChatTurn {
                    turn_id: row.get(0)?,
                    thread_id: row.get(1)?,
                    prompt: row.get(2)?,
                    tier: row.get(3)?,
                    model: row.get(4)?,
                    effort: row.get(5)?,
                    source: row.get(6)?,
                    reason: row.get(7)?,
                    status: row.get(8)?,
                    response: row.get(9)?,
                    error: row.get(10)?,
                    tokens: TokenCounts {
                        input: row.get(11)?,
                        cached_input: row.get(12)?,
                        output: row.get(13)?,
                        reasoning: row.get(14)?,
                        total: row.get(15)?,
                    },
                    created_at: row.get(16)?,
                    completed_at: row.get(17)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(turns)
    }

    pub fn chat_turn_thread(&self, turn_id: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .connection
            .query_row(
                "SELECT thread_id FROM chat_turns WHERE turn_id = ?1",
                params![turn_id],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn usage_by_model(&self) -> Result<Vec<ModelUsage>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT model, count(*), sum(input_tokens), sum(cached_input_tokens),
                sum(output_tokens), sum(reasoning_tokens), sum(total_tokens)
             FROM chat_turns GROUP BY model ORDER BY sum(total_tokens) DESC, model",
        )?;
        let usage = statement
            .query_map([], |row| {
                Ok(ModelUsage {
                    model: row.get(0)?,
                    turns: row.get(1)?,
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
        Ok(usage)
    }
}

/// Persists the streamed outcome of each turn. Events can arrive before the
/// turn row exists (the server streams right after answering `turn/start`), so
/// they are buffered until [`ChatRecorder::register_turn`] runs.
pub struct ChatRecorder {
    state: Mutex<RecorderState>,
}

struct RecorderState {
    store: RouterStore,
    registered: HashSet<String>,
    early: HashMap<String, Vec<SessionEvent>>,
    texts: HashMap<String, String>,
    rate_limits: Option<Value>,
}

impl ChatRecorder {
    pub fn new(store: RouterStore) -> Self {
        Self {
            state: Mutex::new(RecorderState {
                store,
                registered: HashSet::new(),
                early: HashMap::new(),
                texts: HashMap::new(),
                rate_limits: None,
            }),
        }
    }

    pub fn register_turn(
        &self,
        turn_id: &str,
        thread_id: &str,
        prompt: &str,
        decision: &RouteDecision,
    ) -> Result<(), StoreError> {
        let mut state = self.state.lock().expect("recorder");
        state
            .store
            .insert_chat_turn(turn_id, thread_id, prompt, decision)?;
        state.registered.insert(turn_id.to_owned());
        for event in state.early.remove(turn_id).unwrap_or_default() {
            state.apply(&event)?;
        }
        Ok(())
    }

    pub fn record(&self, event: &SessionEvent) -> Result<(), StoreError> {
        let mut state = self.state.lock().expect("recorder");
        if let SessionEvent::RateLimits { snapshot } = event {
            state.rate_limits = Some(snapshot.clone());
            return Ok(());
        }
        let Some(turn_id) = turn_of(event) else {
            return Ok(());
        };
        if state.registered.contains(turn_id) {
            state.apply(event)
        } else {
            state
                .early
                .entry(turn_id.to_owned())
                .or_default()
                .push(event.clone());
            Ok(())
        }
    }

    pub fn last_rate_limits(&self) -> Option<Value> {
        self.state.lock().expect("recorder").rate_limits.clone()
    }

    pub fn with_store<T>(&self, read: impl FnOnce(&RouterStore) -> T) -> T {
        read(&self.state.lock().expect("recorder").store)
    }
}

fn turn_of(event: &SessionEvent) -> Option<&str> {
    match event {
        SessionEvent::AgentDelta { turn_id, .. }
        | SessionEvent::TokenUsage { turn_id, .. }
        | SessionEvent::TurnCompleted { turn_id, .. } => Some(turn_id),
        _ => None,
    }
}

impl RecorderState {
    fn apply(&mut self, event: &SessionEvent) -> Result<(), StoreError> {
        match event {
            SessionEvent::AgentDelta { turn_id, delta, .. } => {
                self.texts
                    .entry(turn_id.clone())
                    .or_default()
                    .push_str(delta);
            }
            SessionEvent::TokenUsage {
                thread_id,
                turn_id,
                usage,
            } => {
                let total = TokenCounts::from_breakdown(&usage["total"]);
                let baseline = self.store.thread_tokens_excluding(thread_id, turn_id)?;
                // Thread totals are cumulative; fall back to the last request
                // when the local history does not cover the whole thread.
                let tokens = total
                    .minus(baseline)
                    .unwrap_or_else(|| TokenCounts::from_breakdown(&usage["last"]));
                self.store.set_chat_turn_tokens(turn_id, tokens)?;
            }
            SessionEvent::TurnCompleted {
                turn_id,
                status,
                error,
                ..
            } => {
                let text = self.texts.remove(turn_id).unwrap_or_default();
                self.store
                    .finish_chat_turn(turn_id, status, &text, error.as_deref())?;
                self.registered.remove(turn_id);
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SentTurn {
    pub thread_id: String,
    pub turn_id: String,
}

pub type ConfigFactory = Box<dyn Fn(&Path) -> AppServerConfig + Send + Sync>;

/// One app-server session per project directory, created on first use and
/// replaced if the process exits.
pub struct ChatService {
    recorder: Arc<ChatRecorder>,
    ui: EventSink,
    config_for: ConfigFactory,
    sessions: Mutex<HashMap<PathBuf, Arc<Session>>>,
}

impl ChatService {
    pub fn new(store: RouterStore, config_for: ConfigFactory, ui: EventSink) -> Self {
        Self {
            recorder: Arc::new(ChatRecorder::new(store)),
            ui,
            config_for,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn recorder(&self) -> &ChatRecorder {
        &self.recorder
    }

    fn session(&self, project: &Path) -> Result<Arc<Session>, SessionError> {
        let mut sessions = self.sessions.lock().expect("sessions");
        if let Some(session) = sessions.get(project) {
            if !session.is_closed() {
                return Ok(session.clone());
            }
        }
        let recorder = self.recorder.clone();
        let ui = self.ui.clone();
        let sink: EventSink = Arc::new(move |event: SessionEvent| {
            if let Err(error) = recorder.record(&event) {
                ui(SessionEvent::RuntimeError {
                    thread_id: None,
                    turn_id: None,
                    message: format!("no se pudo guardar el evento: {error}"),
                    will_retry: false,
                });
            }
            ui(event);
        });
        let session = Arc::new(Session::start(&(self.config_for)(project), sink)?);
        sessions.insert(project.to_path_buf(), session.clone());
        Ok(session)
    }

    /// Send a prompt with an already-reviewed routing decision. Starts a new
    /// thread when `thread_id` is `None`.
    pub fn send(
        &self,
        project: &Path,
        thread_id: Option<&str>,
        prompt: &str,
        decision: &RouteDecision,
    ) -> Result<SentTurn, String> {
        let session = self.session(project).map_err(|e| e.to_string())?;
        let thread_id = match thread_id {
            Some(thread_id) => {
                session
                    .ensure_thread(thread_id)
                    .map_err(|e| e.to_string())?;
                thread_id.to_owned()
            }
            None => {
                let thread_id = session
                    .start_thread(project, &decision.model)
                    .map_err(|e| e.to_string())?;
                let title: String = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
                let title: String = title.chars().take(80).collect();
                self.recorder
                    .with_store(|store| store.upsert_chat_thread(&thread_id, project, &title))
                    .map_err(|e| e.to_string())?;
                thread_id
            }
        };
        let turn_id = session
            .start_turn(
                &thread_id,
                prompt,
                &decision.model,
                &decision.effort,
                project,
            )
            .map_err(|e| e.to_string())?;
        self.recorder
            .register_turn(&turn_id, &thread_id, prompt, decision)
            .map_err(|e| e.to_string())?;
        Ok(SentTurn { thread_id, turn_id })
    }

    pub fn respond_approval(
        &self,
        project: &Path,
        request_id: Value,
        decision: ApprovalDecision,
    ) -> Result<(), String> {
        self.existing(project)?
            .respond_approval(request_id, decision)
            .map_err(|e| e.to_string())
    }

    pub fn interrupt(&self, project: &Path, thread_id: &str, turn_id: &str) -> Result<(), String> {
        self.existing(project)?
            .interrupt(thread_id, turn_id)
            .map_err(|e| e.to_string())
    }

    /// Current rate limits, read through the project's session.
    pub fn rate_limits(&self, project: &Path) -> Result<Value, String> {
        self.session(project)
            .map_err(|e| e.to_string())?
            .rate_limits()
            .map_err(|e| e.to_string())
    }

    fn existing(&self, project: &Path) -> Result<Arc<Session>, String> {
        self.sessions
            .lock()
            .expect("sessions")
            .get(project)
            .filter(|session| !session.is_closed())
            .cloned()
            .ok_or_else(|| "no hay una sesión activa para este proyecto".to_owned())
    }
}

/// Canonical project directory without the Windows verbatim prefix, so the
/// runtime receives a plain path.
pub fn project_dir(path: &Path) -> Result<PathBuf, String> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| format!("carpeta de proyecto inválida: {error}"))?;
    if !canonical.is_dir() {
        return Err("la ruta del proyecto no es una carpeta".into());
    }
    let text = canonical.to_string_lossy();
    Ok(match text.strip_prefix(r"\\?\") {
        Some(plain) if !plain.starts_with("UNC\\") => PathBuf::from(plain),
        _ => canonical,
    })
}
