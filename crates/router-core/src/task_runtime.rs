//! Spec task turns over an interactive [`Session`]: the same streaming,
//! approvals and interrupt as the chat, in a process of their own so the chat
//! recorder never sees them.

use crate::app_server::{
    agent_text, resumed_task_turn, task_resume_params, task_thread_params, task_turn_params,
    task_verify_params, AppServerConfig, AppServerTurn,
};
use crate::coordinator::{CoordinatorError, RuntimeTurn, TaskRuntime};
use crate::domain::TaskDefinition;
use crate::session::{ApprovalDecision, EventSink, RawSink, Session, SessionEvent};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Longest silence inside a turn before the run is considered lost.
const TURN_IDLE_TIMEOUT: Duration = Duration::from_secs(1800);
/// The verify command may take its 120 s, plus the round trip.
const VERIFY_TIMEOUT: Duration = Duration::from_secs(150);

/// Who answers Codex's approval requests during a task turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskApprovals {
    /// Declined at once: nobody can answer (the CLI).
    Decline,
    /// Left to the UI, which answers through [`TaskRuns`].
    Interactive,
}

/// Task runs in progress, so the UI can answer approvals and stop a turn.
#[derive(Default)]
pub struct TaskRuns {
    runs: Mutex<HashMap<String, ActiveRun>>,
}

struct ActiveRun {
    session: Arc<Session>,
    turn: Option<(String, String)>,
}

impl TaskRuns {
    pub fn is_running(&self, task_id: &str) -> bool {
        self.runs.lock().expect("task runs").contains_key(task_id)
    }

    /// Answer an approval request of a running task.
    pub fn respond(
        &self,
        task_id: &str,
        request_id: Value,
        decision: ApprovalDecision,
    ) -> Result<(), String> {
        self.session(task_id)?
            .respond_approval(request_id, decision)
            .map_err(|error| error.to_string())
    }

    /// Stop the current turn of a running task; it ends as interrupted.
    pub fn interrupt(&self, task_id: &str) -> Result<(), String> {
        let (session, turn) = {
            let runs = self.runs.lock().expect("task runs");
            let run = runs
                .get(task_id)
                .ok_or_else(|| format!("la tarea {task_id} no se está ejecutando"))?;
            (run.session.clone(), run.turn.clone())
        };
        let (thread_id, turn_id) =
            turn.ok_or_else(|| format!("la tarea {task_id} aún no tiene un turno en curso"))?;
        session
            .interrupt(&thread_id, &turn_id)
            .map_err(|error| error.to_string())
    }

    fn session(&self, task_id: &str) -> Result<Arc<Session>, String> {
        self.runs
            .lock()
            .expect("task runs")
            .get(task_id)
            .map(|run| run.session.clone())
            .ok_or_else(|| format!("la tarea {task_id} no se está ejecutando"))
    }

    fn register(&self, task_id: &str, session: Arc<Session>) {
        self.runs.lock().expect("task runs").insert(
            task_id.to_owned(),
            ActiveRun {
                session,
                turn: None,
            },
        );
    }

    fn set_turn(&self, task_id: &str, thread_id: &str, turn_id: &str) {
        if let Some(run) = self.runs.lock().expect("task runs").get_mut(task_id) {
            run.turn = Some((thread_id.to_owned(), turn_id.to_owned()));
        }
    }

    fn remove(&self, task_id: &str) {
        self.runs.lock().expect("task runs").remove(task_id);
    }
}

/// One `codex app-server` process for one task, started on first use.
pub struct LocalTaskRuntime {
    config: AppServerConfig,
    approvals: TaskApprovals,
    ui: Option<EventSink>,
    runs: Option<Arc<TaskRuns>>,
    started: Option<Started>,
}

struct Started {
    task_id: String,
    session: Arc<Session>,
    messages: Receiver<Value>,
}

impl LocalTaskRuntime {
    /// Runtime for the CLI: approvals are declined.
    pub fn start(task: &TaskDefinition) -> Result<Self, CoordinatorError> {
        let config = AppServerConfig::local(&task.cwd).with_active_task(&task.id);
        Ok(Self::with_config(config))
    }

    pub fn with_config(config: AppServerConfig) -> Self {
        Self {
            config,
            approvals: TaskApprovals::Decline,
            ui: None,
            runs: None,
            started: None,
        }
    }

    /// Runtime for the app: events go to `ui` and approvals wait for an
    /// answer through `runs`.
    pub fn interactive(config: AppServerConfig, ui: EventSink, runs: Arc<TaskRuns>) -> Self {
        Self {
            config,
            approvals: TaskApprovals::Interactive,
            ui: Some(ui),
            runs: Some(runs),
            started: None,
        }
    }

    fn started(&mut self, task: &TaskDefinition) -> Result<&Started, String> {
        if self.started.is_none() {
            let (sender, messages) = mpsc::channel();
            let sender = Mutex::new(sender);
            let raw: RawSink = Arc::new(move |message: &Value| {
                let _ = sender.lock().expect("task messages").send(message.clone());
            });
            let ui = self.ui.clone();
            let sink: EventSink = Arc::new(move |event: SessionEvent| {
                if let Some(ui) = &ui {
                    ui(event);
                }
            });
            let session = Arc::new(
                Session::start_observed(&self.config, sink, Some(raw))
                    .map_err(|error| error.to_string())?,
            );
            if let Some(runs) = &self.runs {
                runs.register(&task.id, session.clone());
            }
            self.started = Some(Started {
                task_id: task.id.clone(),
                session,
                messages,
            });
        }
        Ok(self.started.as_ref().expect("runtime started"))
    }
}

impl Drop for LocalTaskRuntime {
    fn drop(&mut self) {
        if let (Some(runs), Some(started)) = (&self.runs, &self.started) {
            runs.remove(&started.task_id);
        }
    }
}

fn is_approval(method: &str) -> bool {
    matches!(
        method,
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval"
    )
}

impl TaskRuntime for LocalTaskRuntime {
    fn start_thread(&mut self, task: &TaskDefinition) -> Result<String, String> {
        let params = task_thread_params(task).map_err(|error| error.to_string())?;
        let result = self
            .started(task)?
            .session
            .call("thread/start", params, REQUEST_TIMEOUT)
            .map_err(|error| error.to_string())?;
        result["thread"]["id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| "thread/start returned no thread id".to_owned())
    }

    fn run_turn(&mut self, task: &TaskDefinition, thread_id: &str) -> Result<RuntimeTurn, String> {
        self.run_turn_with_evidence(task, thread_id, &mut |_| Ok(()))
    }

    fn resume_thread(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
    ) -> Result<Option<RuntimeTurn>, String> {
        self.resume_thread_for_turn(task, thread_id, None)
    }

    fn resume_thread_for_turn(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
        expected_turn_id: Option<&str>,
    ) -> Result<Option<RuntimeTurn>, String> {
        let params = task_resume_params(task, thread_id).map_err(|error| error.to_string())?;
        let result = self
            .started(task)?
            .session
            .call("thread/resume", params, REQUEST_TIMEOUT)
            .map_err(|error| error.to_string())?;
        resumed_task_turn(&result, thread_id, expected_turn_id)
            .map(|turn| turn.map(runtime_turn))
            .map_err(|error| error.to_string())
    }

    fn run_turn_with_evidence(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
        evidence: &mut dyn FnMut(&Value) -> Result<(), String>,
    ) -> Result<RuntimeTurn, String> {
        let params = task_turn_params(task, thread_id).map_err(|error| error.to_string())?;
        let approvals = self.approvals;
        let runs = self.runs.clone();
        let started = self.started(task)?;
        let result = started
            .session
            .call("turn/start", params, REQUEST_TIMEOUT)
            .map_err(|error| error.to_string())?;
        let turn_id = result["turn"]["id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| "turn/start returned no turn id".to_owned())?
            .to_owned();
        if let Some(runs) = &runs {
            runs.set_turn(&task.id, thread_id, &turn_id);
        }
        evidence(&json!({ "method": "turn/started", "params": {
            "threadId": thread_id, "turn": { "id": turn_id }
        }}))?;
        let mut text = String::new();
        loop {
            let message = started
                .messages
                .recv_timeout(TURN_IDLE_TIMEOUT)
                .map_err(|error| match error {
                    mpsc::RecvTimeoutError::Timeout => "the turn went silent too long".to_owned(),
                    mpsc::RecvTimeoutError::Disconnected => "the app server closed".to_owned(),
                })?;
            let method = message["method"].as_str().unwrap_or_default();
            let params = &message["params"];
            if params["threadId"]
                .as_str()
                .is_some_and(|event_thread| event_thread != thread_id)
            {
                continue;
            }
            let event_turn = params["turnId"]
                .as_str()
                .or_else(|| params["turn"]["id"].as_str());
            if event_turn.is_some_and(|id| id != turn_id) {
                continue;
            }
            // Turn-scoped output is only evidence when both identities match.
            // In particular, malformed terminal notifications must not be durable.
            if matches!(method, "turn/completed" | "item/agentMessage/delta")
                && (params["threadId"].as_str() != Some(thread_id)
                    || event_turn != Some(turn_id.as_str()))
            {
                continue;
            }
            if message.get("id").is_some() {
                // A server request. Approvals are evidence; the session already
                // refused anything else without stopping the turn.
                if is_approval(method) {
                    evidence(&message)?;
                    if approvals == TaskApprovals::Decline {
                        started
                            .session
                            .respond_approval(message["id"].clone(), ApprovalDecision::Decline)
                            .map_err(|error| error.to_string())?;
                    }
                }
                continue;
            }
            evidence(&message)?;
            if method == "item/agentMessage/delta" {
                if let Some(delta) = params["delta"].as_str() {
                    text.push_str(delta);
                }
            }
            if method == "turn/completed" {
                let turn = &params["turn"];
                let status = turn["status"]
                    .as_str()
                    .ok_or_else(|| "turn has no status".to_owned())?;
                if text.is_empty() {
                    text = agent_text(turn);
                }
                return Ok(runtime_turn(AppServerTurn {
                    thread_id: thread_id.to_owned(),
                    turn_id,
                    text,
                    status: status.to_owned(),
                    error: turn["error"]["message"].as_str().map(ToOwned::to_owned),
                }));
            }
        }
    }

    fn verify(&mut self, task: &TaskDefinition) -> Result<Value, String> {
        let params = task_verify_params(task).map_err(|error| error.to_string())?;
        let result = self
            .started(task)?
            .session
            .call("command/exec", params, VERIFY_TIMEOUT)
            .map_err(|error| error.to_string())?;
        if result["exitCode"].as_i64().is_none() {
            return Err("verification returned no exit code".into());
        }
        Ok(result)
    }
}

fn runtime_turn(turn: AppServerTurn) -> RuntimeTurn {
    RuntimeTurn {
        thread_id: turn.thread_id,
        turn_id: turn.turn_id,
        status: turn.status,
        text: turn.text,
    }
}
