//! Interactive chat session over `codex app-server`.
//!
//! One actor thread owns the process stdin and multiplexes two inputs through a
//! single channel: commands from the UI (start thread, send turn, answer an
//! approval, interrupt) and messages read from the server. Server
//! notifications become [`SessionEvent`]s delivered to a callback; approval
//! requests stay pending until the UI answers them.

use crate::app_server::AppServerConfig;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use thiserror::Error;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SessionError {
    #[error("could not start the local app server: {0}")]
    Start(String),
    #[error("the app server session is closed: {0}")]
    Closed(String),
    #[error("app server returned an error: {0}")]
    Remote(String),
    #[error("app server protocol error: {0}")]
    Protocol(String),
    #[error("timed out waiting for {0}")]
    Timeout(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalKind {
    Command,
    FileChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalDecision {
    Accept,
    AcceptForSession,
    Decline,
    Cancel,
}

/// What a chat thread may do in its project folder. Chosen when the thread
/// starts and applied to every later turn of that thread.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessMode {
    /// Workspace-write sandbox confined to the project, without network.
    #[default]
    Write,
    /// Read-only sandbox without network: any write needs an approval.
    ReadOnly,
}

impl AccessMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::ReadOnly => "readOnly",
        }
    }
}

/// Events delivered to the UI, in server order.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionEvent {
    TurnStarted {
        thread_id: String,
        turn_id: String,
    },
    AgentDelta {
        thread_id: String,
        turn_id: String,
        delta: String,
    },
    ItemStarted {
        thread_id: String,
        turn_id: String,
        item: Value,
    },
    ItemCompleted {
        thread_id: String,
        turn_id: String,
        item: Value,
    },
    CommandOutput {
        thread_id: String,
        turn_id: String,
        item_id: String,
        delta: String,
    },
    ApprovalRequested {
        request_id: Value,
        kind: ApprovalKind,
        thread_id: String,
        turn_id: String,
        item_id: String,
        command: Option<String>,
        cwd: Option<String>,
        reason: Option<String>,
    },
    TokenUsage {
        thread_id: String,
        turn_id: String,
        usage: Value,
    },
    RateLimits {
        snapshot: Value,
    },
    TurnCompleted {
        thread_id: String,
        turn_id: String,
        status: String,
        error: Option<String>,
    },
    RuntimeError {
        thread_id: Option<String>,
        turn_id: Option<String>,
        message: String,
        will_retry: bool,
    },
    Closed {
        reason: String,
    },
}

pub type EventSink = Arc<dyn Fn(SessionEvent) + Send + Sync>;

/// Every server notification and request, as received, before it becomes a
/// [`SessionEvent`]. Runs on the session actor: it must not call the session.
pub type RawSink = Arc<dyn Fn(&Value) + Send + Sync>;

type Reply = Sender<Result<Value, SessionError>>;

enum Input {
    Server(Value),
    ServerClosed(String),
    Request {
        method: String,
        params: Value,
        reply: Reply,
    },
    Notify {
        method: String,
        params: Value,
    },
    Respond {
        id: Value,
        result: Value,
        reply: Reply,
    },
    Shutdown,
}

pub struct Session {
    inputs: Sender<Input>,
    child: Arc<Mutex<Child>>,
    actor: Option<JoinHandle<()>>,
    loaded_threads: Mutex<HashSet<String>>,
    closed: Arc<AtomicBool>,
}

impl Session {
    pub fn start(config: &AppServerConfig, sink: EventSink) -> Result<Self, SessionError> {
        Self::start_observed(config, sink, None)
    }

    /// Like [`Session::start`], also handing every raw server message to `raw`.
    pub fn start_observed(
        config: &AppServerConfig,
        sink: EventSink,
        raw: Option<RawSink>,
    ) -> Result<Self, SessionError> {
        let mut command = Command::new(&config.program);
        command
            .args(&config.arguments)
            .current_dir(&config.working_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env("ROUDEX_ROUTER_INTERNAL", "1");
        if let Some(task_id) = &config.active_task_id {
            command.env("ROUDEX_ACTIVE_TASK_ID", task_id);
        }
        let mut child = command
            .spawn()
            .map_err(|error| SessionError::Start(error.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| SessionError::Start("stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SessionError::Start("stdout unavailable".into()))?;
        let (inputs, receiver) = mpsc::channel();
        let reader_inputs = inputs.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(error) => {
                        let _ = reader_inputs.send(Input::ServerClosed(error.to_string()));
                        return;
                    }
                };
                if line.trim().is_empty() {
                    continue;
                }
                // A non-JSON line (e.g. a stray log) is not a protocol failure.
                if let Ok(message) = serde_json::from_str(&line) {
                    if reader_inputs.send(Input::Server(message)).is_err() {
                        return;
                    }
                }
            }
            let _ = reader_inputs.send(Input::ServerClosed("app server exited".into()));
        });
        let closed = Arc::new(AtomicBool::new(false));
        let actor_closed = closed.clone();
        let actor =
            std::thread::spawn(move || Actor::new(stdin, sink, raw, actor_closed).run(receiver));
        let session = Self {
            inputs,
            child: Arc::new(Mutex::new(child)),
            actor: Some(actor),
            loaded_threads: Mutex::new(HashSet::new()),
            closed,
        };
        session.request(
            "initialize",
            json!({ "clientInfo": {
                "name": config.service_name, "title": "RouDex", "version": env!("CARGO_PKG_VERSION")
            }}),
        )?;
        session.notify("initialized", json!({}))?;
        Ok(session)
    }

    /// New thread confined to `cwd` under `access`, without network; commands
    /// or file changes outside policy ask for approval.
    pub fn start_thread(
        &self,
        cwd: &Path,
        model: &str,
        access: AccessMode,
    ) -> Result<String, SessionError> {
        let result = self.request("thread/start", thread_start_params(cwd, model, access))?;
        let thread_id = result["thread"]["id"]
            .as_str()
            .ok_or_else(|| SessionError::Protocol("thread/start returned no thread id".into()))?
            .to_owned();
        self.loaded_threads
            .lock()
            .expect("loaded threads")
            .insert(thread_id.clone());
        Ok(thread_id)
    }

    /// Load an existing thread into this process once; later calls are no-ops.
    pub fn ensure_thread(&self, thread_id: &str) -> Result<(), SessionError> {
        if self
            .loaded_threads
            .lock()
            .expect("loaded threads")
            .contains(thread_id)
        {
            return Ok(());
        }
        self.request("thread/resume", json!({ "threadId": thread_id }))?;
        self.loaded_threads
            .lock()
            .expect("loaded threads")
            .insert(thread_id.to_owned());
        Ok(())
    }

    pub fn start_turn(
        &self,
        thread_id: &str,
        prompt: &str,
        model: &str,
        effort: &str,
        cwd: &Path,
        access: AccessMode,
    ) -> Result<String, SessionError> {
        let result = self.request(
            "turn/start",
            turn_start_params(thread_id, prompt, model, effort, cwd, access),
        )?;
        result["turn"]["id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| SessionError::Protocol("turn/start returned no turn id".into()))
    }

    pub fn interrupt(&self, thread_id: &str, turn_id: &str) -> Result<(), SessionError> {
        self.request(
            "turn/interrupt",
            json!({ "threadId": thread_id, "turnId": turn_id }),
        )
        .map(|_| ())
    }

    pub fn respond_approval(
        &self,
        request_id: Value,
        decision: ApprovalDecision,
    ) -> Result<(), SessionError> {
        let (reply, response) = mpsc::channel();
        self.send(Input::Respond {
            id: request_id,
            result: json!({ "decision": decision }),
            reply,
        })?;
        wait(response, "approval response", REQUEST_TIMEOUT).map(|_| ())
    }

    /// True once the app-server process exited or the session was dropped.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn rate_limits(&self) -> Result<Value, SessionError> {
        self.request("account/rateLimits/read", json!({}))
    }

    fn request(&self, method: &str, params: Value) -> Result<Value, SessionError> {
        self.call(method, params, REQUEST_TIMEOUT)
    }

    /// Send a request and wait up to `timeout` for its result.
    pub(crate) fn call(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, SessionError> {
        let (reply, response) = mpsc::channel();
        self.send(Input::Request {
            method: method.into(),
            params,
            reply,
        })?;
        wait(response, method, timeout)
    }

    fn notify(&self, method: &str, params: Value) -> Result<(), SessionError> {
        self.send(Input::Notify {
            method: method.into(),
            params,
        })
    }

    fn send(&self, input: Input) -> Result<(), SessionError> {
        self.inputs
            .send(input)
            .map_err(|_| SessionError::Closed("session actor stopped".into()))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.inputs.send(Input::Shutdown);
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(actor) = self.actor.take() {
            let _ = actor.join();
        }
    }
}

fn thread_start_params(cwd: &Path, model: &str, access: AccessMode) -> Value {
    let mut params = json!({
        "model": model,
        "cwd": cwd,
        "approvalPolicy": "on-request",
        "serviceName": "roudex-chat"
    });
    match access {
        AccessMode::Write => {
            params["sandbox"] = json!("workspace-write");
            params["config"] = json!({
                "sandbox_workspace_write.writable_roots": [cwd],
                "sandbox_workspace_write.network_access": false
            });
        }
        AccessMode::ReadOnly => params["sandbox"] = json!("read-only"),
    }
    params
}

fn turn_start_params(
    thread_id: &str,
    prompt: &str,
    model: &str,
    effort: &str,
    cwd: &Path,
    access: AccessMode,
) -> Value {
    let sandbox_policy = match access {
        AccessMode::Write => json!({
            "type": "workspaceWrite",
            "writableRoots": [cwd],
            "networkAccess": false
        }),
        AccessMode::ReadOnly => json!({ "type": "readOnly", "networkAccess": false }),
    };
    json!({
        "threadId": thread_id,
        "input": [{ "type": "text", "text": prompt }],
        "cwd": cwd,
        "approvalPolicy": "on-request",
        "sandboxPolicy": sandbox_policy,
        "model": model,
        "effort": effort
    })
}

fn wait(
    response: Receiver<Result<Value, SessionError>>,
    what: &str,
    timeout: Duration,
) -> Result<Value, SessionError> {
    response
        .recv_timeout(timeout)
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => SessionError::Timeout(what.into()),
            mpsc::RecvTimeoutError::Disconnected => {
                SessionError::Closed("session actor stopped".into())
            }
        })?
}

struct Actor {
    stdin: ChildStdin,
    sink: EventSink,
    raw: Option<RawSink>,
    next_id: u64,
    pending: HashMap<u64, Reply>,
    closed: Arc<AtomicBool>,
}

impl Actor {
    fn new(
        stdin: ChildStdin,
        sink: EventSink,
        raw: Option<RawSink>,
        closed: Arc<AtomicBool>,
    ) -> Self {
        Self {
            stdin,
            sink,
            raw,
            next_id: 1,
            pending: HashMap::new(),
            closed,
        }
    }

    fn run(mut self, inputs: Receiver<Input>) {
        for input in inputs {
            match input {
                Input::Request {
                    method,
                    params,
                    reply,
                } => {
                    let id = self.next_id;
                    self.next_id += 1;
                    match self.write(&json!({ "method": method, "id": id, "params": params })) {
                        Ok(()) => {
                            self.pending.insert(id, reply);
                        }
                        Err(error) => {
                            let _ = reply.send(Err(error));
                        }
                    }
                }
                Input::Notify { method, params } => {
                    let _ = self.write(&json!({ "method": method, "params": params }));
                }
                Input::Respond { id, result, reply } => {
                    let _ = reply.send(
                        self.write(&json!({ "id": id, "result": result }))
                            .map(|()| Value::Null),
                    );
                }
                Input::Server(message) => self.handle(message),
                Input::ServerClosed(reason) => {
                    self.close(&reason);
                    return;
                }
                Input::Shutdown => {
                    self.close("session closed");
                    return;
                }
            }
        }
    }

    fn close(&mut self, reason: &str) {
        self.closed.store(true, Ordering::SeqCst);
        for (_, reply) in self.pending.drain() {
            let _ = reply.send(Err(SessionError::Closed(reason.into())));
        }
        (self.sink)(SessionEvent::Closed {
            reason: reason.into(),
        });
    }

    fn write(&mut self, message: &Value) -> Result<(), SessionError> {
        let mut line = serde_json::to_vec(message)
            .map_err(|error| SessionError::Protocol(error.to_string()))?;
        line.push(b'\n');
        self.stdin
            .write_all(&line)
            .and_then(|()| self.stdin.flush())
            .map_err(|error| SessionError::Closed(error.to_string()))
    }

    fn handle(&mut self, message: Value) {
        let method = message["method"].as_str().map(ToOwned::to_owned);
        if let (Some(raw), Some(_)) = (&self.raw, &method) {
            raw(&message);
        }
        match (method, message.get("id").cloned()) {
            (Some(method), Some(id)) => self.server_request(&method, id, &message["params"]),
            (Some(method), None) => {
                if let Some(event) = notification_event(&method, &message["params"]) {
                    (self.sink)(event);
                }
            }
            (None, Some(id)) => {
                let Some(reply) = id.as_u64().and_then(|id| self.pending.remove(&id)) else {
                    return;
                };
                let result = match message.get("error") {
                    Some(error) => Err(SessionError::Remote(
                        error["message"]
                            .as_str()
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| error.to_string()),
                    )),
                    None => Ok(message["result"].clone()),
                };
                let _ = reply.send(result);
            }
            (None, None) => {}
        }
    }

    fn server_request(&mut self, method: &str, id: Value, params: &Value) {
        let kind = match method {
            "item/commandExecution/requestApproval" => Some(ApprovalKind::Command),
            "item/fileChange/requestApproval" => Some(ApprovalKind::FileChange),
            _ => None,
        };
        let text = |key: &str| params[key].as_str().map(ToOwned::to_owned);
        match kind {
            Some(kind) => (self.sink)(SessionEvent::ApprovalRequested {
                request_id: id,
                kind,
                thread_id: text("threadId").unwrap_or_default(),
                turn_id: text("turnId").unwrap_or_default(),
                item_id: text("itemId").unwrap_or_default(),
                command: text("command"),
                cwd: text("cwd"),
                reason: text("reason"),
            }),
            None => {
                // Anything RouDex cannot present (user input, elicitation,
                // dynamic tools) is refused so the turn never hangs on it.
                let _ = self.write(&json!({ "id": id, "error": {
                    "code": -32601,
                    "message": format!("RouDex does not support {method}")
                }}));
                (self.sink)(SessionEvent::RuntimeError {
                    thread_id: text("threadId"),
                    turn_id: text("turnId"),
                    message: format!("solicitud no soportada rechazada: {method}"),
                    will_retry: false,
                });
            }
        }
    }
}

fn notification_event(method: &str, params: &Value) -> Option<SessionEvent> {
    let text = |key: &str| params[key].as_str().unwrap_or_default().to_owned();
    let turn_id = || {
        params["turnId"]
            .as_str()
            .or_else(|| params["turn"]["id"].as_str())
            .unwrap_or_default()
            .to_owned()
    };
    Some(match method {
        "turn/started" => SessionEvent::TurnStarted {
            thread_id: text("threadId"),
            turn_id: turn_id(),
        },
        "item/agentMessage/delta" => SessionEvent::AgentDelta {
            thread_id: text("threadId"),
            turn_id: turn_id(),
            delta: text("delta"),
        },
        "item/started" => SessionEvent::ItemStarted {
            thread_id: text("threadId"),
            turn_id: turn_id(),
            item: params["item"].clone(),
        },
        "item/completed" => SessionEvent::ItemCompleted {
            thread_id: text("threadId"),
            turn_id: turn_id(),
            item: params["item"].clone(),
        },
        "item/commandExecution/outputDelta" => SessionEvent::CommandOutput {
            thread_id: text("threadId"),
            turn_id: turn_id(),
            item_id: text("itemId"),
            delta: text("delta"),
        },
        "thread/tokenUsage/updated" => SessionEvent::TokenUsage {
            thread_id: text("threadId"),
            turn_id: turn_id(),
            usage: params["tokenUsage"].clone(),
        },
        "account/rateLimits/updated" => SessionEvent::RateLimits {
            snapshot: params["rateLimits"].clone(),
        },
        "turn/completed" => SessionEvent::TurnCompleted {
            thread_id: text("threadId"),
            turn_id: turn_id(),
            status: params["turn"]["status"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned(),
            error: params["turn"]["error"]["message"]
                .as_str()
                .map(ToOwned::to_owned),
        },
        "error" => SessionEvent::RuntimeError {
            thread_id: params["threadId"].as_str().map(ToOwned::to_owned),
            turn_id: params["turnId"].as_str().map(ToOwned::to_owned),
            message: params["error"]["message"]
                .as_str()
                .unwrap_or("runtime error")
                .to_owned(),
            will_retry: params["willRetry"].as_bool().unwrap_or(false),
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_turn_lifecycle_notifications() {
        let completed = notification_event(
            "turn/completed",
            &json!({ "threadId": "t", "turn": { "id": "u", "status": "failed",
                "error": { "message": "usage limit" } } }),
        );
        assert_eq!(
            completed,
            Some(SessionEvent::TurnCompleted {
                thread_id: "t".into(),
                turn_id: "u".into(),
                status: "failed".into(),
                error: Some("usage limit".into()),
            })
        );
        assert_eq!(
            notification_event(
                "item/agentMessage/delta",
                &json!({ "threadId": "t", "turnId": "u", "delta": "hola" })
            ),
            Some(SessionEvent::AgentDelta {
                thread_id: "t".into(),
                turn_id: "u".into(),
                delta: "hola".into()
            })
        );
        assert_eq!(notification_event("skills/changed", &json!({})), None);
    }

    #[test]
    fn events_serialize_with_camel_case_tags_and_fields() {
        let value = serde_json::to_value(SessionEvent::ApprovalRequested {
            request_id: json!(7),
            kind: ApprovalKind::FileChange,
            thread_id: "t".into(),
            turn_id: "u".into(),
            item_id: "i".into(),
            command: None,
            cwd: None,
            reason: Some("write".into()),
        })
        .unwrap();
        assert_eq!(value["type"], "approvalRequested");
        assert_eq!(value["kind"], "fileChange");
        assert_eq!(value["requestId"], 7);
        assert_eq!(
            serde_json::to_value(ApprovalDecision::AcceptForSession).unwrap(),
            "acceptForSession"
        );
    }

    #[test]
    fn write_threads_get_a_workspace_sandbox_rooted_at_the_project() {
        let cwd = Path::new("C:/p/api");
        let thread = thread_start_params(cwd, "gpt-6.1-sol", AccessMode::Write);
        assert_eq!(thread["approvalPolicy"], "on-request");
        assert_eq!(thread["sandbox"], "workspace-write");
        assert_eq!(
            thread["config"]["sandbox_workspace_write.writable_roots"],
            json!(["C:/p/api"])
        );
        assert_eq!(
            thread["config"]["sandbox_workspace_write.network_access"],
            false
        );
        let turn = turn_start_params("t", "hola", "gpt-6.1-sol", "medium", cwd, AccessMode::Write);
        assert_eq!(turn["approvalPolicy"], "on-request");
        assert_eq!(
            turn["sandboxPolicy"],
            json!({ "type": "workspaceWrite", "writableRoots": ["C:/p/api"], "networkAccess": false })
        );
        assert_eq!(turn["input"][0]["text"], "hola");
        assert_eq!(
            (&turn["model"], &turn["effort"]),
            (&json!("gpt-6.1-sol"), &json!("medium"))
        );
    }

    #[test]
    fn read_only_threads_get_no_writable_roots() {
        let cwd = Path::new("C:/p/api");
        let thread = thread_start_params(cwd, "gpt-6-luna", AccessMode::ReadOnly);
        assert_eq!(thread["approvalPolicy"], "on-request");
        assert_eq!(thread["sandbox"], "read-only");
        assert!(thread.get("config").is_none());
        let turn = turn_start_params(
            "t",
            "explica",
            "gpt-6-luna",
            "high",
            cwd,
            AccessMode::ReadOnly,
        );
        assert_eq!(turn["approvalPolicy"], "on-request");
        assert_eq!(
            turn["sandboxPolicy"],
            json!({ "type": "readOnly", "networkAccess": false })
        );
    }

    #[test]
    fn access_mode_names_match_the_ui() {
        assert_eq!(
            serde_json::to_value(AccessMode::ReadOnly).unwrap(),
            "readOnly"
        );
        assert_eq!(
            serde_json::from_value::<AccessMode>(json!("write")).unwrap(),
            AccessMode::Write
        );
        assert_eq!(AccessMode::default(), AccessMode::Write);
        assert_eq!(AccessMode::ReadOnly.as_str(), "readOnly");
    }
}
