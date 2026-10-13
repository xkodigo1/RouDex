use router_core::{
    EventRecord, RouteReceipt, RouteStatus, RouterStore, TaskDefinition, TaskSandbox, TaskStatus,
};
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
        .env_remove("ROUDEX_ACTIVE_TASK_ID")
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
fn either_worker_marker_prevents_nested_lifecycle_commands() {
    for marker in ["ROUDEX_ROUTER_INTERNAL", "ROUDEX_ACTIVE_TASK_ID"] {
        for args in [
            vec!["task-run", "T-005"],
            vec!["continue-run", "run"],
            vec!["task-approve", "T-005"],
            vec!["task-recover", "T-005"],
            vec!["task-retry", "T-005"],
            vec!["task-plan", "missing.json"],
        ] {
            let root = temp_root("nested-worker");
            let output = Command::new(binary())
                .args(&args)
                .env("LOCALAPPDATA", &root)
                .env_remove("ROUDEX_ROUTER_INTERNAL")
                .env_remove("ROUDEX_ACTIVE_TASK_ID")
                .env(marker, "1")
                .stdin(Stdio::null())
                .output()
                .unwrap();
            if args[0] == "continue-run" {
                let response: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(response["decision"], "block");
                assert_eq!(response["continue"], false);
            } else {
                assert!(!output.status.success());
                assert!(String::from_utf8_lossy(&output.stderr).contains("worker activo"));
            }
            assert!(!root.exists(), "internal command must not touch state");
        }
    }
}

#[test]
fn active_task_marker_alone_skips_ingress_without_persisting_a_run() {
    let root = temp_root("active-task");
    let response = run(
        &["route-prompt", "--record-only"],
        &input("worker message"),
        &root,
        &[("ROUDEX_ACTIVE_TASK_ID", "T-005")],
    );
    assert_eq!(response, serde_json::json!({"continue": true}));
    assert!(!root.exists());
}

fn lifecycle_task() -> TaskDefinition {
    TaskDefinition {
        id: "T-lifecycle".into(),
        title: "Lifecycle".into(),
        prompt: "Original task".into(),
        depends_on: vec![],
        status: TaskStatus::Ready,
        profile: "standard".into(),
        model: "gpt-6.1-sol".into(),
        effort: "medium".into(),
        approval: "manual".into(),
        scope: vec![".".into()],
        cwd: ".".into(),
        sandbox: TaskSandbox {
            kind: "workspaceWrite".into(),
            writable_roots: vec![".".into()],
        },
        acceptance: vec!["verified".into()],
        verify: "echo verified".into(),
    }
}

#[test]
fn cli_approval_denial_and_runtime_failure_keep_the_task_closed() {
    let root = temp_root("approval-lifecycle");
    let db = root.join("RouDex/router.sqlite3");
    let task = lifecycle_task();
    RouterStore::open(&db)
        .unwrap()
        .upsert_task("spec", &task)
        .unwrap();
    let env = [("ROUDEX_CODEX_BIN", "missing-codex-binary")];
    assert_eq!(
        run(&["task-run", &task.id], "", &root, &env)["status"],
        "awaiting_approval"
    );
    run(&["task-approve", &task.id, "denied"], "", &root, &env);
    for _ in 0..2 {
        assert_eq!(
            run(&["task-run", &task.id], "", &root, &env)["status"],
            "blocked"
        );
    }
    assert_eq!(RouterStore::open(&db).unwrap().count_runs().unwrap(), 0);
    run(&["task-approve", &task.id, "approved"], "", &root, &env);
    let failed = run(&["task-run", &task.id], "", &root, &env);
    assert_eq!(failed["status"], "failed");
    let store = RouterStore::open(&db).unwrap();
    let run_id = failed["run_id"].as_str().unwrap();
    assert_eq!(store.count_runs().unwrap(), 1);
    assert_eq!(
        store.get_task(&task.id).unwrap().unwrap().definition.status,
        TaskStatus::Failed
    );
    assert!(store
        .events_for_run(run_id)
        .unwrap()
        .iter()
        .any(|event| event.event_type == "thread.start.failed"));
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_continuation_uses_coordinator_approval_and_terminal_replay_gates() {
    for completed in [false, true] {
        let root = temp_root("task-continuation");
        let db = root.join("RouDex/router.sqlite3");
        let task = lifecycle_task();
        let store = RouterStore::open(&db).unwrap();
        store.upsert_task("spec", &task).unwrap();
        store
            .record_task_approval(&router_core::TaskApproval {
                task_id: task.id.clone(),
                decision: "approved".into(),
                reason: None,
            })
            .unwrap();
        store
            .begin_task_run(
                &task,
                "spec",
                &RouteReceipt {
                    route_id: "route".into(),
                    run_id: "run".into(),
                    profile: task.profile.clone(),
                    model: task.model.clone(),
                    effort: task.effort.clone(),
                    status: RouteStatus::Accepted,
                },
            )
            .unwrap();
        store
            .record_event(&EventRecord {
                run_id: "run".into(),
                event_type: "thread.started".into(),
                payload: serde_json::json!({"threadId": "thread"}),
            })
            .unwrap();
        store
            .record_event(&EventRecord {
                run_id: "run".into(),
                event_type: "turn/started".into(),
                payload: serde_json::json!({"threadId": "thread", "turn": {"id": "turn"}}),
            })
            .unwrap();
        store
            .finish_task_run(
                &task.id,
                "run",
                if completed {
                    TaskStatus::Completed
                } else {
                    TaskStatus::Interrupted
                },
                None,
            )
            .unwrap();
        store
            .record_task_approval(&router_core::TaskApproval {
                task_id: task.id.clone(),
                decision: "denied".into(),
                reason: Some("revoked".into()),
            })
            .unwrap();
        let before = store.events_for_run("run").unwrap();
        for body in ["", "Replacement task must not be executed"] {
            let result = run(
                &["continue-run", "run"],
                body,
                &root,
                &[("ROUDEX_CODEX_BIN", "missing-codex-binary")],
            );
            assert_eq!(result["run_id"], "run");
            assert_eq!(result["route_id"], "route");
            assert_eq!(
                result["status"],
                if completed {
                    "completed"
                } else {
                    "interrupted"
                }
            );
        }
        assert_eq!(store.events_for_run("run").unwrap(), before);
        assert_eq!(store.count_runs().unwrap(), 1);
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn task_plan_imports_manifest_and_reports_ready_tasks() {
    let root = temp_root("task-plan");
    let manifest_path = root.join("tasks.json");
    let manifest = serde_json::json!({
        "spec_id": "spec-test",
        "title": "Task test",
        "version": 1,
        "tasks": [
            {
                "id": "T-001",
                "title": "Foundation",
                "prompt": "Prepare the foundation",
                "depends_on": [],
                "status": "completed",
                "profile": "routine",
                "model": "gpt-6-luna",
                "effort": "medium",
                "approval": "not_required",
                "scope": ["crates/router-core"],
                "cwd": ".",
                "sandbox": {"type": "readOnly", "writable_roots": []},
                "acceptance": ["foundation exists"],
                "verify": "cargo test"
            },
            {
                "id": "T-002",
                "title": "Next",
                "prompt": "Implement the next task",
                "depends_on": ["T-001"],
                "status": "planned",
                "profile": "standard",
                "model": "gpt-6.1-sol",
                "effort": "medium",
                "approval": "manual",
                "scope": ["crates/router-core"],
                "cwd": ".",
                "sandbox": {"type": "workspaceWrite", "writable_roots": ["crates/router-core"]},
                "acceptance": ["next exists"],
                "verify": "cargo test"
            }
        ]
    });
    fs::create_dir_all(&root).expect("manifest directory");
    fs::write(
        &manifest_path,
        serde_json::to_vec(&manifest).expect("manifest JSON"),
    )
    .expect("manifest");
    let manifest_arg = manifest_path.to_string_lossy().to_string();
    let response = run(&["task-plan", manifest_arg.as_str()], "", &root, &[]);
    assert_eq!(response["specId"], "spec-test");
    assert_eq!(response["taskCount"], 2);
    assert_eq!(response["readyTaskIds"], serde_json::json!(["T-002"]));
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).expect("state store");
    assert_eq!(
        store
            .get_task("T-002")
            .expect("task")
            .expect("exists")
            .spec_id,
        "spec-test"
    );
    drop(store);
    fs::remove_dir_all(root).expect("cleanup");
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
    assert!(document.contains("route-prompt.ps1"));
    assert!(managed_dir.join("router-cli.exe").exists());
    assert!(managed_dir.join("route-prompt.ps1").exists());
    assert!(!root.join("codex/hooks.json").exists());

    let removed = run(&["uninstall-managed-hook"], "", &root, &environment);
    assert!(removed.is_null());
    assert!(!requirements.exists());
    assert!(!managed_dir.join("router-cli.exe").exists());
    assert!(!managed_dir.join("route-prompt.ps1").exists());
    fs::remove_dir_all(root).expect("cleanup");
}
