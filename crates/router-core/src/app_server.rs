use crate::domain::TaskDefinition;
use crate::tiers::{CatalogModel, ModelTarget};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppServerError {
    #[error("could not start the local app server: {0}")]
    Start(#[source] std::io::Error),
    #[error("app server transport error: {0}")]
    Transport(#[source] std::io::Error),
    #[error("app server protocol error: {0}")]
    Protocol(String),
    #[error("app server returned an error: {0}")]
    Remote(String),
    #[error("app server returned a non-completed turn: {0}")]
    Turn(String),
    #[error("could not persist app server evidence: {0}")]
    Evidence(String),
}

#[derive(Debug, Clone)]
pub struct AppServerConfig {
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub service_name: String,
    pub active_task_id: Option<String>,
}

impl AppServerConfig {
    pub fn local(working_directory: impl AsRef<Path>) -> Self {
        let program = std::env::var_os("ROUDEX_CODEX_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("codex"));
        Self {
            program,
            arguments: vec!["app-server".to_owned(), "--stdio".to_owned()],
            working_directory: working_directory.as_ref().to_path_buf(),
            service_name: "roudex".to_owned(),
            active_task_id: None,
        }
    }

    pub fn with_active_task(mut self, task_id: impl Into<String>) -> Self {
        self.active_task_id = Some(task_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppServerTurn {
    pub thread_id: String,
    pub turn_id: String,
    pub text: String,
    pub status: String,
    /// Runtime error message of a non-completed turn (e.g. usage limits).
    pub error: Option<String>,
}

pub struct AppServerClient {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Result<Value, AppServerError>>,
    response_timeout: Duration,
    next_request_id: u64,
}

impl AppServerClient {
    pub fn start(config: &AppServerConfig) -> Result<Self, AppServerError> {
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

        let mut child = command.spawn().map_err(AppServerError::Start)?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppServerError::Protocol("app server stdin unavailable".to_owned()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppServerError::Protocol("app server stdout unavailable".to_owned()))?;

        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let message = match line {
                    Ok(line) if line.trim().is_empty() => continue,
                    Ok(line) => serde_json::from_str(&line)
                        .map_err(|error| AppServerError::Protocol(error.to_string())),
                    Err(error) => Err(AppServerError::Transport(error)),
                };
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        let mut client = Self {
            child,
            stdin,
            messages,
            response_timeout: Duration::from_secs(30),
            next_request_id: 1,
        };
        client.initialize(&config.service_name)?;
        Ok(client)
    }

    fn initialize(&mut self, service_name: &str) -> Result<(), AppServerError> {
        let id = self.next_id();
        self.send(json!({
            "method": "initialize",
            "id": id,
            "params": {
                "clientInfo": {
                    "name": service_name,
                    "title": "Local task router",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }))?;
        self.wait_for_response(id)?;
        self.send(json!({ "method": "initialized", "params": {} }))
    }

    pub fn start_thread(
        &mut self,
        model: &str,
        cwd: &Path,
        approval_policy: &str,
        sandbox: &str,
    ) -> Result<String, AppServerError> {
        let id = self.next_id();
        self.send(json!({
            "method": "thread/start",
            "id": id,
            "params": {
                "model": model,
                "cwd": cwd,
                "approvalPolicy": approval_policy,
                "sandbox": sandbox,
                "serviceName": "roudex"
            }
        }))?;
        let response = self.wait_for_response(id)?;
        response["result"]["thread"]["id"]
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AppServerError::Protocol("thread/start returned no thread id".to_owned())
            })
    }

    pub fn resume_thread(&mut self, thread_id: &str) -> Result<(), AppServerError> {
        let id = self.next_id();
        self.send(json!({
            "method": "thread/resume",
            "id": id,
            "params": { "threadId": thread_id }
        }))?;
        self.wait_for_response(id).map(|_| ())
    }

    pub fn run_turn(
        &mut self,
        thread_id: &str,
        prompt: &str,
        model: &str,
        effort: &str,
        cwd: &Path,
        approval_policy: &str,
        sandbox: &str,
    ) -> Result<AppServerTurn, AppServerError> {
        self.run_turn_params(
            json!({
                "threadId": thread_id,
                "input": [{ "type": "text", "text": prompt }],
                "cwd": cwd,
                "approvalPolicy": approval_policy,
                "sandboxPolicy": sandbox_policy(sandbox),
                "model": model,
                "effort": effort
            }),
            &mut |_| Ok(()),
        )
    }

    /// Runtime model catalog (`model/list`), following pagination.
    pub fn list_models(&mut self) -> Result<Vec<CatalogModel>, AppServerError> {
        let mut models = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let response = self.request(
                "model/list",
                json!({ "includeHidden": false, "cursor": cursor }),
            )?;
            let result = &response["result"];
            let data = result["data"].as_array().ok_or_else(|| {
                AppServerError::Protocol("model/list returned no data array".into())
            })?;
            for entry in data {
                models.push(catalog_model(entry)?);
            }
            match result["nextCursor"].as_str() {
                Some(next) if !next.is_empty() => cursor = Some(next.to_owned()),
                _ => return Ok(models),
            }
        }
    }

    /// One-shot classification turn: ephemeral, read-only thread that never
    /// asks for approvals. Returns the final assistant text.
    pub fn classify(
        &mut self,
        target: &ModelTarget,
        cwd: &Path,
        instructions: &str,
        input: &str,
        output_schema: Value,
    ) -> Result<String, AppServerError> {
        let response = self.request(
            "thread/start",
            json!({
                "model": target.model,
                "cwd": cwd,
                "approvalPolicy": "never",
                "sandbox": "read-only",
                "ephemeral": true,
                "developerInstructions": instructions,
                "serviceName": "roudex-classifier"
            }),
        )?;
        let thread_id = response["result"]["thread"]["id"]
            .as_str()
            .ok_or_else(|| AppServerError::Protocol("thread/start returned no thread id".into()))?
            .to_owned();
        let turn = self.run_turn_params(
            json!({
                "threadId": thread_id,
                "input": [{ "type": "text", "text": input }],
                "cwd": cwd,
                "approvalPolicy": "never",
                "sandboxPolicy": { "type": "readOnly" },
                "model": target.model,
                "effort": target.effort,
                "outputSchema": output_schema
            }),
            &mut |_| Ok(()),
        )?;
        if turn.status != "completed" {
            return Err(AppServerError::Turn(match turn.error {
                Some(error) => format!("{}: {error}", turn.status),
                None => turn.status,
            }));
        }
        Ok(turn.text)
    }

    pub fn start_task_thread(&mut self, task: &TaskDefinition) -> Result<String, AppServerError> {
        let params = task_thread_params(task)?;
        let response = self.request("thread/start", params)?;
        response["result"]["thread"]["id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppServerError::Protocol("thread/start returned no thread id".into()))
    }

    /// Resume persisted history before deciding whether another turn is safe.
    pub fn resume_task_thread(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
    ) -> Result<Option<AppServerTurn>, AppServerError> {
        self.resume_task_thread_for_turn(task, thread_id, None)
    }

    pub fn resume_task_thread_for_turn(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
        expected_turn_id: Option<&str>,
    ) -> Result<Option<AppServerTurn>, AppServerError> {
        let mut params = task_thread_params(task)?;
        params["threadId"] = json!(thread_id);
        // serviceName is a thread/start field, not a resume override.
        params
            .as_object_mut()
            .expect("thread parameters")
            .remove("serviceName");
        let response = self.request("thread/resume", params)?;
        let thread = &response["result"]["thread"];
        if thread["id"].as_str() != Some(thread_id) {
            return Err(AppServerError::Protocol(
                "thread/resume returned a different thread".into(),
            ));
        }
        let turns = thread["turns"]
            .as_array()
            .ok_or_else(|| AppServerError::Protocol("resumed thread has no turn history".into()))?;
        if expected_turn_id.is_some_and(|expected| {
            turns.last().and_then(|turn| turn["id"].as_str()) != Some(expected)
        }) {
            return Err(AppServerError::Protocol(
                "resumed history does not match the persisted turn; explicit recovery required"
                    .into(),
            ));
        }
        match turns.last() {
            Some(turn) if turn["status"] == "completed" => Ok(Some(AppServerTurn {
                thread_id: thread_id.into(),
                turn_id: turn["id"]
                    .as_str()
                    .ok_or_else(|| AppServerError::Protocol("resumed turn has no id".into()))?
                    .into(),
                text: agent_text(turn),
                status: "completed".into(),
                error: None,
            })),
            Some(turn) if turn["status"] != "interrupted" => Err(AppServerError::Turn(format!(
                "resumed thread cannot start another turn: {}",
                turn["status"]
            ))),
            _ => Ok(None),
        }
    }

    pub fn run_task_turn(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
        evidence: &mut dyn FnMut(&Value) -> Result<(), String>,
    ) -> Result<AppServerTurn, AppServerError> {
        self.run_turn_params(
            json!({
                "threadId": thread_id,
                "input": [{ "type": "text", "text": task.prompt }],
                "cwd": absolute_cwd(task)?, "approvalPolicy": "never",
                "sandboxPolicy": task_sandbox_policy(task)?,
                "model": task.model, "effort": task.effort
            }),
            evidence,
        )
    }

    pub fn verify_task(&mut self, task: &TaskDefinition) -> Result<Value, AppServerError> {
        // The command executes in the same server sandbox, never in the coordinator host shell.
        let cwd = absolute_cwd(task)?;
        // Keep cargo away from the running coordinator executable on Windows.
        // Hex encoding keeps arbitrary task ids inside one distinct path component.
        let task_directory = format!(
            "task-{}",
            task.id
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let cargo_target_dir = cwd
            .join("target")
            .join("roudex-verification")
            .join(task_directory);
        let command = if cfg!(windows) {
            vec!["cmd.exe", "/D", "/S", "/C", task.verify.as_str()]
        } else {
            vec!["sh", "-c", task.verify.as_str()]
        };
        let id = self.next_id();
        self.send(json!({ "id": id, "method": "command/exec", "params": {
            "command": command, "cwd": cwd,
            "sandboxPolicy": task_sandbox_policy(task)?, "timeoutMs": 120000,
            "env": { "ROUDEX_ROUTER_INTERNAL": "1", "ROUDEX_ACTIVE_TASK_ID": task.id,
                "CARGO_TARGET_DIR": cargo_target_dir }
        }}))?;
        let previous_timeout = self.response_timeout;
        self.response_timeout = Duration::from_secs(150);
        let response = self.wait_for_response(id);
        self.response_timeout = previous_timeout;
        let response = response?;
        if response["result"]["exitCode"].as_i64().is_none() {
            return Err(AppServerError::Protocol(
                "verification returned no exit code".into(),
            ));
        }
        Ok(response["result"].clone())
    }

    fn run_turn_params(
        &mut self,
        params: Value,
        evidence: &mut dyn FnMut(&Value) -> Result<(), String>,
    ) -> Result<AppServerTurn, AppServerError> {
        let thread_id = params["threadId"].as_str().unwrap_or_default().to_owned();
        let id = self.next_id();
        self.send(json!({ "method": "turn/start", "id": id, "params": params }))?;
        let mut pending = Vec::new();
        let turn_id = loop {
            let message = self.read_message()?;
            self.reject_server_request(&message)?;
            if message["id"].as_u64() == Some(id) && message.get("method").is_none() {
                if let Some(error) = message.get("error") {
                    return Err(AppServerError::Remote(error.to_string()));
                }
                break message["result"]["turn"]["id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| {
                        AppServerError::Protocol("turn/start returned no turn id".into())
                    })?
                    .to_owned();
            }
            pending.push(message);
        };
        evidence(&json!({ "method": "turn/started", "params": {
            "threadId": thread_id, "turn": { "id": turn_id }
        }}))
        .map_err(AppServerError::Evidence)?;
        let mut pending = pending.into_iter();
        let mut text = String::new();
        loop {
            let message = match pending.next() {
                Some(message) => message,
                None => self
                    .messages
                    .recv_timeout(Duration::from_secs(1800))
                    .map_err(|error| {
                        AppServerError::Protocol(format!("waiting for turn: {error}"))
                    })??,
            };
            self.reject_server_request(&message)?;
            let params = &message["params"];
            if params["threadId"]
                .as_str()
                .is_some_and(|event_thread_id| event_thread_id != thread_id)
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
            if matches!(
                message["method"].as_str(),
                Some("turn/completed" | "item/agentMessage/delta")
            ) && (params["threadId"].as_str() != Some(thread_id.as_str())
                || event_turn != Some(turn_id.as_str()))
            {
                continue;
            }
            if message.get("method").is_some() {
                evidence(&message).map_err(AppServerError::Evidence)?;
            }
            if message["method"] == "item/agentMessage/delta" {
                if let Some(delta) = params["delta"].as_str() {
                    text.push_str(delta);
                }
            }
            if message["method"] == "turn/completed"
                && params["threadId"].as_str() == Some(thread_id.as_str())
                && event_turn == Some(turn_id.as_str())
            {
                let turn = &params["turn"];
                let status = turn["status"]
                    .as_str()
                    .ok_or_else(|| AppServerError::Protocol("turn has no status".into()))?;
                if text.is_empty() {
                    text = agent_text(turn);
                }
                return Ok(AppServerTurn {
                    thread_id,
                    turn_id,
                    text,
                    status: status.into(),
                    error: turn["error"]["message"].as_str().map(ToOwned::to_owned),
                });
            }
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, AppServerError> {
        let id = self.next_id();
        self.send(json!({ "method": method, "id": id, "params": params }))?;
        self.wait_for_response(id)
    }

    fn reject_server_request(&mut self, message: &Value) -> Result<(), AppServerError> {
        if message.get("method").is_some() && message.get("id").is_some() {
            self.send(json!({ "id": message["id"], "error": {
                "code": -32601, "message": "RouDex does not authorize interactive server requests"
            }}))?;
            return Err(AppServerError::Protocol(format!(
                "unsupported server request: {}",
                message["method"]
            )));
        }
        Ok(())
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    fn send(&mut self, message: Value) -> Result<(), AppServerError> {
        serde_json::to_writer(&mut self.stdin, &message)
            .map_err(|error| AppServerError::Protocol(error.to_string()))?;
        self.stdin
            .write_all(b"\n")
            .map_err(AppServerError::Transport)?;
        self.stdin.flush().map_err(AppServerError::Transport)
    }

    fn read_message(&mut self) -> Result<Value, AppServerError> {
        self.messages
            .recv_timeout(self.response_timeout)
            .map_err(|error| {
                AppServerError::Protocol(format!("waiting for app server response: {error}"))
            })?
    }

    fn wait_for_response(&mut self, request_id: u64) -> Result<Value, AppServerError> {
        loop {
            let message = self.read_message()?;
            self.reject_server_request(&message)?;
            if message.get("id").and_then(Value::as_u64) != Some(request_id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(AppServerError::Remote(error.to_string()));
            }
            return Ok(message);
        }
    }
}

fn absolute_cwd(task: &TaskDefinition) -> Result<PathBuf, AppServerError> {
    std::fs::canonicalize(&task.cwd).map_err(AppServerError::Transport)
}

fn task_thread_params(task: &TaskDefinition) -> Result<Value, AppServerError> {
    let policy = task_sandbox_policy(task)?;
    let mut config = json!({ "model_reasoning_effort": task.effort });
    if task.sandbox.kind == "workspaceWrite" {
        config["sandbox_workspace_write.writable_roots"] = policy["writableRoots"].clone();
        config["sandbox_workspace_write.network_access"] = json!(false);
    }
    Ok(
        json!({ "model": task.model, "cwd": absolute_cwd(task)?, "approvalPolicy": "never",
            "sandbox": match task.sandbox.kind.as_str() {
                "readOnly" => "read-only", "workspaceWrite" => "workspace-write",
                _ => "danger-full-access"
            },
            "config": config,
            "serviceName": "roudex"
        }),
    )
}

pub(crate) fn task_sandbox_policy(task: &TaskDefinition) -> Result<Value, AppServerError> {
    match task.sandbox.kind.as_str() {
        "readOnly" => Ok(json!({ "type": "readOnly", "networkAccess": false })),
        "workspaceWrite" if !task.sandbox.writable_roots.is_empty() => {
            let cwd = absolute_cwd(task)?;
            let roots = task
                .sandbox
                .writable_roots
                .iter()
                .map(|root| {
                    let path = cwd.join(root);
                    let path = std::fs::canonicalize(path).map_err(AppServerError::Transport)?;
                    if !path.starts_with(&cwd) {
                        return Err(AppServerError::Protocol(
                            "writable root is outside task cwd".into(),
                        ));
                    }
                    Ok(path)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({ "type": "workspaceWrite", "writableRoots": roots, "networkAccess": false }))
        }
        "dangerFullAccess" => Ok(json!({ "type": "dangerFullAccess" })),
        _ => Err(AppServerError::Protocol(
            "invalid task sandbox or missing writable roots".into(),
        )),
    }
}

fn catalog_model(entry: &Value) -> Result<CatalogModel, AppServerError> {
    let text = |key: &str| {
        entry[key]
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppServerError::Protocol(format!("model/list entry without {key}")))
    };
    Ok(CatalogModel {
        id: text("id")?,
        model: text("model")?,
        display_name: text("displayName")?,
        hidden: entry["hidden"].as_bool().unwrap_or(false),
        default_effort: text("defaultReasoningEffort")?,
        efforts: entry["supportedReasoningEfforts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|option| option["reasoningEffort"].as_str())
            .map(ToOwned::to_owned)
            .collect(),
    })
}

fn agent_text(turn: &Value) -> String {
    turn["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["type"] == "agentMessage")
        .filter_map(|item| item["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

impl Drop for AppServerClient {
    fn drop(&mut self) {
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn sandbox_policy(name: &str) -> Value {
    match name {
        "read-only" => json!({ "type": "readOnly" }),
        "danger-full-access" => json!({ "type": "dangerFullAccess" }),
        _ => json!({ "type": "workspaceWrite" }),
    }
}
