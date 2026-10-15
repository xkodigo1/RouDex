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
            .unwrap_or_else(|| {
                discover_codex(
                    std::env::var_os("PATH").as_deref(),
                    std::env::var_os("LOCALAPPDATA").as_deref(),
                )
            });
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

/// Locate the Codex executable: `codex` on PATH first, then the newest copy
/// installed by Codex Desktop (`%LOCALAPPDATA%\OpenAI\Codex\bin\<hash>\`), whose
/// folder changes on every update. Falls back to the bare name.
pub fn discover_codex(
    path_env: Option<&std::ffi::OsStr>,
    local_app_data: Option<&std::ffi::OsStr>,
) -> PathBuf {
    let names: &[&str] = if cfg!(windows) {
        &["codex.exe", "codex.cmd"]
    } else {
        &["codex"]
    };
    if let Some(found) = path_env
        .into_iter()
        .flat_map(std::env::split_paths)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
    {
        return found;
    }
    let desktop =
        local_app_data.map(|root| Path::new(root).join("OpenAI").join("Codex").join("bin"));
    desktop
        .and_then(|bin| std::fs::read_dir(bin).ok())
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join(names[0]))
        .filter_map(|candidate| {
            let modified = candidate.metadata().ok()?.modified().ok()?;
            Some((modified, candidate))
        })
        .max()
        .map(|(_, candidate)| candidate)
        .unwrap_or_else(|| PathBuf::from("codex"))
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

pub(crate) fn absolute_cwd(task: &TaskDefinition) -> Result<PathBuf, AppServerError> {
    std::fs::canonicalize(&task.cwd).map_err(AppServerError::Transport)
}

/// Task turns ask before acting outside their sandbox; the UI or the CLI
/// answers.
pub(crate) const TASK_APPROVAL_POLICY: &str = "on-request";

/// `thread/start` for a task, with its routed model and effort.
pub(crate) fn task_thread_params(task: &TaskDefinition) -> Result<Value, AppServerError> {
    let policy = task_sandbox_policy(task)?;
    let mut config = json!({ "model_reasoning_effort": task.effort });
    if task.sandbox.kind == "workspaceWrite" {
        config["sandbox_workspace_write.writable_roots"] = policy["writableRoots"].clone();
        config["sandbox_workspace_write.network_access"] = json!(false);
    }
    let sandbox = match task.sandbox.kind.as_str() {
        "readOnly" => "read-only",
        _ => "workspace-write",
    };
    Ok(json!({ "model": task.model, "cwd": absolute_cwd(task)?,
        "approvalPolicy": TASK_APPROVAL_POLICY, "sandbox": sandbox, "config": config,
        "serviceName": "roudex"
    }))
}

/// `thread/resume` for a task thread; the same overrides as its start.
pub(crate) fn task_resume_params(
    task: &TaskDefinition,
    thread_id: &str,
) -> Result<Value, AppServerError> {
    let mut params = task_thread_params(task)?;
    params["threadId"] = json!(thread_id);
    // serviceName is a thread/start field, not a resume override.
    params
        .as_object_mut()
        .expect("thread parameters")
        .remove("serviceName");
    Ok(params)
}

/// `turn/start` that sends the task prompt.
pub(crate) fn task_turn_params(
    task: &TaskDefinition,
    thread_id: &str,
) -> Result<Value, AppServerError> {
    Ok(json!({
        "threadId": thread_id,
        "input": [{ "type": "text", "text": task.prompt }],
        "cwd": absolute_cwd(task)?, "approvalPolicy": TASK_APPROVAL_POLICY,
        "sandboxPolicy": task_sandbox_policy(task)?,
        "model": task.model, "effort": task.effort
    }))
}

/// Read a resumed thread: its last turn must be the persisted one when
/// `expected_turn_id` is given. A completed last turn is returned so it is not
/// replayed; an interrupted (or absent) one allows another turn.
pub(crate) fn resumed_task_turn(
    result: &Value,
    thread_id: &str,
    expected_turn_id: Option<&str>,
) -> Result<Option<AppServerTurn>, AppServerError> {
    let thread = &result["thread"];
    if thread["id"].as_str() != Some(thread_id) {
        return Err(AppServerError::Protocol(
            "thread/resume returned a different thread".into(),
        ));
    }
    let turns = thread["turns"]
        .as_array()
        .ok_or_else(|| AppServerError::Protocol("resumed thread has no turn history".into()))?;
    if expected_turn_id
        .is_some_and(|expected| turns.last().and_then(|turn| turn["id"].as_str()) != Some(expected))
    {
        return Err(AppServerError::Protocol(
            "resumed history does not match the persisted turn; explicit recovery required".into(),
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

/// `command/exec` for a task's verify command, in the task sandbox and never
/// in the host shell. Cargo builds into a per-task target directory so it
/// stays away from the running coordinator executable on Windows.
pub(crate) fn task_verify_params(task: &TaskDefinition) -> Result<Value, AppServerError> {
    let cwd = absolute_cwd(task)?;
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
    // On Windows the command reaches cmd.exe through an environment variable.
    // As an argument, process quoting would escape its double quotes as \",
    // which cmd.exe does not read, and any quoted argument would break.
    let command = if cfg!(windows) {
        vec!["cmd.exe", "/D", "/C", "%ROUDEX_VERIFY%"]
    } else {
        vec!["sh", "-c", task.verify.as_str()]
    };
    Ok(json!({
        "command": command, "cwd": cwd,
        "sandboxPolicy": task_sandbox_policy(task)?, "timeoutMs": 120000,
        "env": { "ROUDEX_ROUTER_INTERNAL": "1", "ROUDEX_ACTIVE_TASK_ID": task.id,
            "CARGO_TARGET_DIR": cargo_target_dir, "ROUDEX_VERIFY": task.verify }
    }))
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

pub(crate) fn agent_text(turn: &Value) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn executable(dir: &Path, name: &str, age_secs: u64) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        let file = std::fs::File::create(&path).unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(age_secs))
            .unwrap();
        path
    }

    #[test]
    fn discovers_codex_on_path_before_desktop_copies() {
        let root = std::env::temp_dir().join(format!("roudex-discover-{}", uuid::Uuid::new_v4()));
        let name = if cfg!(windows) { "codex.exe" } else { "codex" };
        let bin = root.join("OpenAI").join("Codex").join("bin");
        executable(&bin.join("old-hash"), name, 3600);
        let newest = executable(&bin.join("new-hash"), name, 10);
        assert_eq!(discover_codex(None, Some(root.as_os_str())), newest);

        let on_path = executable(&root.join("path-dir"), name, 7200);
        let path_env = std::env::join_paths([root.join("missing"), root.join("path-dir")]).unwrap();
        assert_eq!(
            discover_codex(Some(path_env.as_os_str()), Some(root.as_os_str())),
            on_path
        );
        assert_eq!(discover_codex(None, None), PathBuf::from("codex"));
        let _ = std::fs::remove_dir_all(root);
    }

    /// A verify command reaches the shell as written, quotes included, after
    /// the same argument quoting Codex applies when it starts the process.
    #[test]
    fn verify_commands_keep_their_quotes() {
        let cwd = std::env::temp_dir().join(format!("roudex-verify-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(cwd.join("a b.txt"), "ok").unwrap();
        let run = |verify: &str| {
            let task: TaskDefinition = serde_json::from_value(json!({
                "id": "T-1", "title": "t", "prompt": "p", "depends_on": [], "status": "ready",
                "tier": "economico", "approval": "not_required", "scope": ["."],
                "cwd": cwd, "sandbox": { "type": "workspaceWrite", "writable_roots": ["."] },
                "acceptance": ["a"], "verify": verify
            }))
            .unwrap();
            let params = task_verify_params(&task).unwrap();
            let argv: Vec<&str> = params["command"]
                .as_array()
                .unwrap()
                .iter()
                .map(|arg| arg.as_str().unwrap())
                .collect();
            let mut command = Command::new(argv[0]);
            command.args(&argv[1..]).current_dir(&cwd);
            for (key, value) in params["env"].as_object().unwrap() {
                command.env(key, value.as_str().unwrap());
            }
            let output = command.output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            (output.status.code(), stdout)
        };
        if cfg!(windows) {
            assert_eq!(run(r#"echo "a b""#), (Some(0), r#""a b""#.to_owned()));
            assert_eq!(
                run(r#"if exist "a b.txt" (exit 0) else (exit 7)"#).0,
                Some(0)
            );
            assert_eq!(
                run(r#"if exist "c d.txt" (exit 0) else (exit 7)"#).0,
                Some(7)
            );
            assert_eq!(run(r#"echo "x" && exit 3"#).0, Some(3));
        } else {
            assert_eq!(run(r#"test -f "a b.txt""#).0, Some(0));
            assert_eq!(run(r#"test -f "c d.txt""#).0, Some(1));
            assert_eq!(run(r#"echo "x" && exit 3"#).0, Some(3));
        }
        let _ = std::fs::remove_dir_all(cwd);
    }
}
