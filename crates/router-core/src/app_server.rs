use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
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
}

#[derive(Debug, Clone)]
pub struct AppServerConfig {
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub service_name: String,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppServerTurn {
    pub thread_id: String,
    pub turn_id: String,
    pub text: String,
    pub status: String,
}

pub struct AppServerClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
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

        let mut child = command.spawn().map_err(AppServerError::Start)?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppServerError::Protocol("app server stdin unavailable".to_owned()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppServerError::Protocol("app server stdout unavailable".to_owned()))?;

        let mut client = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
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
                "sandboxPolicy": sandbox_policy(sandbox),
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
        let id = self.next_id();
        self.send(json!({
            "method": "turn/start",
            "id": id,
            "params": {
                "threadId": thread_id,
                "input": [{ "type": "text", "text": prompt }],
                "cwd": cwd,
                "approvalPolicy": approval_policy,
                "sandboxPolicy": sandbox_policy(sandbox),
                "model": model,
                "effort": effort
            }
        }))?;

        let mut text = String::new();
        let mut response_received = false;
        let mut completed: Option<Value> = None;
        while completed.is_none() {
            let message = self.read_message()?;
            if message.get("id").and_then(Value::as_u64) == Some(id) {
                response_received = true;
                if let Some(error) = message.get("error") {
                    return Err(AppServerError::Remote(error.to_string()));
                }
            }
            if message.get("method").and_then(Value::as_str) == Some("item/agentMessage/delta") {
                if let Some(delta) = message["params"]["delta"].as_str() {
                    text.push_str(delta);
                }
            }
            if message.get("method").and_then(Value::as_str) == Some("turn/completed") {
                completed = Some(message["params"]["turn"].clone());
            }
        }

        if !response_received {
            return Err(AppServerError::Protocol(
                "turn/completed arrived without turn/start response".to_owned(),
            ));
        }
        let turn = completed.expect("loop guarantees a completed turn");
        let status = turn["status"].as_str().unwrap_or("unknown").to_owned();
        if status != "completed" {
            return Err(AppServerError::Turn(status));
        }
        let turn_id = turn["id"]
            .as_str()
            .ok_or_else(|| AppServerError::Protocol("turn/completed returned no id".to_owned()))?;
        Ok(AppServerTurn {
            thread_id: thread_id.to_owned(),
            turn_id: turn_id.to_owned(),
            text,
            status,
        })
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
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = self
                .stdout
                .read_line(&mut line)
                .map_err(AppServerError::Transport)?;
            if bytes == 0 {
                return Err(AppServerError::Protocol(
                    "app server closed stdout unexpectedly".to_owned(),
                ));
            }
            if !line.trim().is_empty() {
                return serde_json::from_str(line.trim())
                    .map_err(|error| AppServerError::Protocol(error.to_string()));
            }
        }
    }

    fn wait_for_response(&mut self, request_id: u64) -> Result<Value, AppServerError> {
        loop {
            let message = self.read_message()?;
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
