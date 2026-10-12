use router_core::RouterStore;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("roudex-{label}-{}-{stamp}", std::process::id()))
}

fn binary() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_router-cli")
            .expect("Cargo must expose the router-cli test binary"),
    )
}

fn input(prompt: &str) -> String {
    serde_json::json!({
        "prompt": prompt,
        "session_id": "session-test",
        "turn_id": "turn-test",
        "cwd": ".",
        "permission_mode": "default"
    })
    .to_string()
}

fn run(args: &[&str], body: &str, state_root: &Path, extra_env: &[(&str, &str)]) -> Value {
    let mut command = Command::new(binary());
    command
        .args(args)
        .env("LOCALAPPDATA", state_root)
        .env("CODEX_HOME", state_root.join("codex"))
        .env_remove("ROUDEX_ROUTER_INTERNAL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in extra_env {
        command.env(name, value);
    }
    let mut child = command.spawn().expect("start router-cli");
    child
        .stdin
        .take()
        .expect("router stdin")
        .write_all(body.as_bytes())
        .expect("write router input");
    let output = child.wait_with_output().expect("wait for router-cli");
    assert!(
        output.status.success(),
        "router-cli failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if output.stdout.iter().all(|byte| byte.is_ascii_whitespace()) {
        Value::Null
    } else {
        serde_json::from_slice(&output.stdout).expect("JSON hook response")
    }
}

#[test]
fn record_only_returns_a_route_receipt_and_persists_it() {
    let root = temp_root("record");
    let response = run(
        &["route-prompt", "--record-only"],
        &input("Actualizar el README con instrucciones"),
        &root,
        &[],
    );

    assert_eq!(response["continue"], true);
    assert!(response["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("context")
        .contains("runId="));
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).expect("state store");
    assert_eq!(store.count_runs().expect("run count"), 1);
    drop(store);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn router_failure_blocks_the_prompt() {
    let root = temp_root("fail-closed");
    let response = run(
        &["route-prompt"],
        &input("Analizar una tarea"),
        &root,
        &[("ROUDEX_CODEX_BIN", "missing-codex-binary")],
    );

    assert_eq!(response["continue"], false);
    assert_eq!(response["decision"], "block");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn internal_worker_bypasses_the_hook_once() {
    let root = temp_root("internal");
    let mut command = Command::new(binary());
    command
        .args(["route-prompt"])
        .env("LOCALAPPDATA", &root)
        .env("ROUDEX_ROUTER_INTERNAL", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = command.spawn().expect("start router-cli");
    child
        .stdin
        .take()
        .expect("router stdin")
        .write_all(input("worker message").as_bytes())
        .expect("write router input");
    let output = child.wait_with_output().expect("wait for router-cli");
    let response: Value = serde_json::from_slice(&output.stdout).expect("JSON hook response");
    assert_eq!(response, serde_json::json!({"continue": true}));
    assert!(!root.exists());
}

#[test]
fn hook_installation_is_idempotent() {
    let root = temp_root("hooks");
    let first = run(&["install-hook"], "", &root, &[]);
    assert!(first.is_null());
    let second = run(&["install-hook"], "", &root, &[]);
    assert!(second.is_null());

    let hooks: Value = serde_json::from_str(
        &fs::read_to_string(root.join("codex/hooks.json")).expect("hooks file"),
    )
    .expect("hooks JSON");
    let entries = hooks["hooks"]["UserPromptSubmit"]
        .as_array()
        .expect("prompt hooks");
    assert_eq!(entries.len(), 1);
    assert!(entries[0]["hooks"][0]["command"]
        .as_str()
        .expect("hook command")
        .contains("route-prompt --record-only"));
    assert_eq!(
        entries[0]["hooks"][0]["commandWindows"],
        entries[0]["hooks"][0]["command"]
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn managed_hook_installation_is_explicit_and_idempotent() {
    let root = temp_root("managed-hooks");
    let requirements = root.join("requirements.toml");
    let managed_dir = root.join("managed");
    let requirements_value = requirements.to_string_lossy().to_string();
    let managed_dir_value = managed_dir.to_string_lossy().to_string();
    let environment = [
        (
            "ROUDEX_CODEX_REQUIREMENTS_PATH",
            requirements_value.as_str(),
        ),
        ("ROUDEX_MANAGED_HOOK_DIR", managed_dir_value.as_str()),
    ];

    let first = run(&["install-managed-hook"], "", &root, &environment);
    assert!(first.is_null());
    let second = run(&["install-managed-hook"], "", &root, &environment);
    assert!(second.is_null());

    let document = fs::read_to_string(&requirements).expect("managed requirements");
    assert!(document.contains("# RouDex managed desktop routing hook"));
    assert!(document.contains("hooks = true"));
    assert!(document.contains("[[hooks.UserPromptSubmit]]"));
    assert!(document.contains("command_windows"));
    assert!(document.contains("route-prompt --record-only"));
    assert!(managed_dir.join("router-cli.exe").exists());
    assert!(!root.join("codex/hooks.json").exists());

    let removed = run(&["uninstall-managed-hook"], "", &root, &environment);
    assert!(removed.is_null());
    assert!(!requirements.exists());
    assert!(!managed_dir.join("router-cli.exe").exists());
    fs::remove_dir_all(root).expect("cleanup");
}
