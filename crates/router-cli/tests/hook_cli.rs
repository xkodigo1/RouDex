//! `router-cli hook`, `hook-install` and `hook-uninstall` against the built
//! binary, with isolated `LOCALAPPDATA`, `APPDATA`, `ProgramData`,
//! `CODEX_HOME` and `ROUDEX_HOME`.

use router_core::RouterStore;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct Sandbox(PathBuf);

impl Sandbox {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "roudex-hook-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(dir.join("project")).unwrap();
        Self(dir)
    }

    fn run(&self, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_router-cli"));
        command
            .args(args)
            .env("LOCALAPPDATA", self.0.join("appdata"))
            .env("APPDATA", self.0.join("roaming"))
            .env("ProgramData", self.0.join("programdata"))
            .env("CODEX_HOME", self.0.join("codex"))
            .env("ROUDEX_HOME", self.home())
            .env_remove("ROUDEX_ROUTER_INTERNAL")
            .env_remove("ROUDEX_ACTIVE_TASK_ID")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (name, value) in env {
            command.env(name, value);
        }
        let mut child = command.spawn().unwrap();
        feed(child.stdin.take(), stdin.as_bytes());
        child.wait_with_output().unwrap()
    }

    fn hook(&self, prompt: &str, model: &str, turn: &str) -> Output {
        let input = json!({
            "session_id": "session-1",
            "turn_id": turn,
            "transcript_path": null,
            "cwd": self.0.join("project"),
            "hook_event_name": "UserPromptSubmit",
            "model": model,
            "permission_mode": "default",
            "prompt": prompt
        });
        self.run(&["hook"], &input.to_string(), &[])
    }

    fn home(&self) -> PathBuf {
        self.0.join("home")
    }

    fn router_toml(&self, contents: &str) {
        let dir = self.0.join("appdata").join("RouDex");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("router.toml"), contents).unwrap();
    }

    /// The hook store lives in ROUDEX_HOME, outside AppData.
    fn store(&self) -> RouterStore {
        let path = self.home().join("desktop-hook.sqlite3");
        assert!(path.exists(), "the hook store is in ROUDEX_HOME");
        RouterStore::open(path).unwrap()
    }

    fn hooks_json(&self) -> PathBuf {
        self.0.join("codex").join("hooks.json")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stdout_json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

const RISKY: &str = "aplica la migración en producción";

const BLOCKING: &str = r#"version = 1
fallback_tier = "estandar"

[tiers.economico]
model = "gpt-6-luna"
effort = "high"

[tiers.estandar]
model = "gpt-6.1-sol"
effort = "medium"

[tiers.fuerte]
model = "gpt-6.1-sol"
effort = "high"

[classifier]
model = "gpt-6-luna"
effort = "low"

[desktop_hook]
block_high_risk = true
"#;

#[test]
fn risky_prompts_warn_and_are_recorded() {
    let sandbox = Sandbox::new("warn");
    let response = stdout_json(&sandbox.hook(RISKY, "gpt-6-luna", "turn-1"));
    let message = response["systemMessage"].as_str().unwrap();
    assert!(
        message.contains("tier fuerte (gpt-6.1-sol high)"),
        "{message}"
    );
    assert!(response.get("decision").is_none());
    assert_eq!(
        response["hookSpecificOutput"]["hookEventName"],
        "UserPromptSubmit"
    );
    let summary = sandbox.store().desktop_summary().unwrap();
    assert_eq!((summary.prompts, summary.warned, summary.fuerte), (1, 1, 1));
}

#[test]
fn the_blocking_option_blocks_risky_prompts() {
    let sandbox = Sandbox::new("block");
    sandbox.router_toml(BLOCKING);
    let response = stdout_json(&sandbox.hook(RISKY, "gpt-6-luna", "turn-1"));
    assert_eq!(response["decision"], "block");
    assert!(response["reason"]
        .as_str()
        .unwrap()
        .contains("gpt-6.1-sol con esfuerzo high"));
    // The same prompt on the strong model passes with context only.
    let strong = stdout_json(&sandbox.hook(RISKY, "gpt-6.1-sol", "turn-2"));
    assert!(strong.get("decision").is_none());
    assert!(strong.get("systemMessage").is_none());
    assert_eq!(sandbox.store().desktop_summary().unwrap().blocked, 1);
}

#[test]
fn a_repeated_turn_is_recorded_once() {
    let sandbox = Sandbox::new("repeat");
    stdout_json(&sandbox.hook("explica el README", "gpt-6-luna", "turn-1"));
    stdout_json(&sandbox.hook("explica el README", "gpt-6-luna", "turn-1"));
    let summary = sandbox.store().desktop_summary().unwrap();
    assert_eq!((summary.prompts, summary.economico), (1, 1));
}

#[test]
fn failures_let_the_prompt_through() {
    let sandbox = Sandbox::new("fail-open");
    for stdin in ["", "not json", "{\"prompt\": \"sin sesión\"}"] {
        let output = sandbox.run(&["hook"], stdin, &[]);
        assert!(output.status.success(), "stdin {stdin:?}");
        assert!(output.stdout.is_empty(), "stdin {stdin:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("RouDex hook"));
    }
    sandbox.router_toml("version = 1\nsurprise = true\n");
    let output = sandbox.hook(RISKY, "gpt-6-luna", "turn-1");
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("router configuration"));
    // Codex Desktop hides stderr, so every failure also lands in hook.log,
    // with the field names but never the prompt.
    let log = fs::read_to_string(sandbox.home().join("hook.log")).unwrap();
    assert_eq!(log.lines().count(), 4, "{log}");
    assert!(log.contains("missing field `session_id`") && log.contains("campos: prompt:string"));
    assert!(!log.contains("sin sesión") && !log.contains("migración"));
}

#[test]
fn roudex_internal_turns_are_ignored() {
    let sandbox = Sandbox::new("internal");
    let input = json!({ "session_id": "s", "turn_id": "t", "cwd": sandbox.0.join("project"),
        "model": "gpt-6-luna", "prompt": RISKY });
    let output = sandbox.run(
        &["hook"],
        &input.to_string(),
        &[("ROUDEX_ROUTER_INTERNAL", "1")],
    );
    assert!(output.status.success());
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    assert!(!sandbox.home().join("desktop-hook.sqlite3").exists());
    assert!(!sandbox.home().join("hook.log").exists());
}

fn hooks(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn install_is_idempotent_and_uninstall_keeps_other_hooks() {
    let sandbox = Sandbox::new("install");
    fs::create_dir_all(sandbox.0.join("codex")).unwrap();
    let existing = json!({
        "hooks": {
            "UserPromptSubmit": [
                { "hooks": [{ "type": "command", "command": "powershell -File C:/old/route-prompt.ps1" }] },
                { "hooks": [{ "type": "command", "command": "notify.exe" }] }
            ],
            "Stop": [{ "hooks": [{ "type": "command", "command": "beep.exe" }] }]
        }
    });
    fs::write(sandbox.hooks_json(), existing.to_string()).unwrap();

    let first = stdout_json(&sandbox.run(&["hook-install"], "", &[]));
    assert_eq!(
        (first["removedLegacy"].as_u64(), first["replaced"].as_u64()),
        (Some(1), Some(0))
    );
    let second = stdout_json(&sandbox.run(&["hook-install"], "", &[]));
    assert_eq!(
        (
            second["removedLegacy"].as_u64(),
            second["replaced"].as_u64()
        ),
        (Some(0), Some(1))
    );

    let document = hooks(&sandbox.hooks_json());
    let groups = document["hooks"]["UserPromptSubmit"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0]["hooks"][0]["command"], "notify.exe");
    let command = groups[1]["hooks"][0]["command"].as_str().unwrap();
    let binary = sandbox.home().join("bin").join(if cfg!(windows) {
        "router-cli.exe"
    } else {
        "router-cli"
    });
    assert!(
        binary.exists(),
        "hook-install copies router-cli to ROUDEX_HOME"
    );
    assert_eq!(
        command,
        format!("{} hook", binary.to_string_lossy().replace('\\', "/"))
    );
    assert!(first["legacyManagedHook"].is_null());
    assert_eq!(
        document["hooks"]["Stop"][0]["hooks"][0]["command"],
        "beep.exe"
    );

    let removed = stdout_json(&sandbox.run(&["hook-uninstall"], "", &[]));
    assert_eq!(removed["removed"], 1);
    let document = hooks(&sandbox.hooks_json());
    assert_eq!(
        document["hooks"]["UserPromptSubmit"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        document["hooks"]["Stop"][0]["hooks"][0]["command"],
        "beep.exe"
    );
}

#[test]
fn install_creates_hooks_json_and_uninstall_without_one_is_a_no_op() {
    let sandbox = Sandbox::new("fresh");
    let removed = stdout_json(&sandbox.run(&["hook-uninstall"], "", &[]));
    assert_eq!(removed["removed"], 0);
    assert!(!sandbox.hooks_json().exists());
    stdout_json(&sandbox.run(&["hook-install"], "", &[]));
    let document = hooks(&sandbox.hooks_json());
    assert_eq!(
        document["hooks"]["UserPromptSubmit"][0]["hooks"][0]["type"],
        "command"
    );
}

/// Shells that may run a hook command; Codex does not say which one it uses.
/// On Windows `sh` only exists with Git Bash or similar, so it is tested
/// when present; cmd and PowerShell always are.
fn shells(command: &str) -> Vec<Command> {
    let shell = |program: &str, args: &[&str]| {
        let mut shell = Command::new(program);
        shell.args(args).arg(command);
        shell
    };
    let has_sh = Command::new("sh")
        .args(["-c", "exit 0"])
        .status()
        .is_ok_and(|status| status.success());
    let mut shells = Vec::new();
    if has_sh || !cfg!(windows) {
        shells.push(shell("sh", &["-c"]));
    }
    if cfg!(windows) {
        shells.push(shell("cmd", &["/d", "/c"]));
        shells.push(shell(
            "powershell",
            &["-NoProfile", "-NonInteractive", "-Command"],
        ));
    }
    shells
}

#[test]
fn the_installed_command_runs_in_every_shell() {
    let sandbox = Sandbox::new("command");
    stdout_json(&sandbox.run(&["hook-install"], "", &[]));
    let document = hooks(&sandbox.hooks_json());
    let command = document["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_owned();
    let shells = shells(&command);
    let tried = shells.len() as i64;
    for (turn, mut shell) in shells.into_iter().enumerate() {
        let input = json!({ "session_id": "s", "turn_id": turn.to_string(),
            "cwd": sandbox.0.join("project"), "model": "gpt-6-luna", "prompt": RISKY });
        let mut child = shell
            .env("ROUDEX_HOME", sandbox.home())
            .env("LOCALAPPDATA", sandbox.0.join("appdata"))
            .env_remove("ROUDEX_ROUTER_INTERNAL")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        feed(child.stdin.take(), input.to_string().as_bytes());
        let output = child.wait_with_output().unwrap();
        let response = stdout_json(&output);
        assert!(
            response["systemMessage"].is_string(),
            "{:?}: {response}",
            shell.get_program()
        );
    }
    assert_eq!(sandbox.store().desktop_summary().unwrap().warned, tried);
}

#[test]
fn install_refuses_app_data_and_reports_the_v05_managed_hook() {
    let sandbox = Sandbox::new("guards");
    // A home under LOCALAPPDATA would be a private copy for Codex Desktop.
    let inside = sandbox.0.join("appdata").join("RouDex");
    let output = sandbox.run(
        &["hook-install"],
        "",
        &[("ROUDEX_HOME", inside.to_str().unwrap())],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("LOCALAPPDATA"));
    assert!(!sandbox.hooks_json().exists());

    let managed = sandbox.0.join("programdata").join("OpenAI").join("Codex");
    fs::create_dir_all(&managed).unwrap();
    fs::write(
        managed.join("requirements.toml"),
        "# RouDex managed desktop routing hook
[[hooks.UserPromptSubmit]]
",
    )
    .unwrap();
    let installed = sandbox.run(&["hook-install"], "", &[]);
    let report = stdout_json(&installed);
    assert!(report["legacyManagedHook"]
        .as_str()
        .unwrap()
        .ends_with("requirements.toml"));
    assert!(String::from_utf8_lossy(&installed.stderr).contains("v0.5.0"));
}

/// Feed a child's stdin. The command may exit before reading it (for example
/// when it rejects its arguments), which closes the pipe on Unix.
fn feed(stdin: Option<std::process::ChildStdin>, input: &[u8]) {
    use std::io::Write;
    match stdin.expect("child stdin").write_all(input) {
        Err(error) if error.kind() != std::io::ErrorKind::BrokenPipe => {
            panic!("write child stdin: {error}")
        }
        _ => {}
    }
}
