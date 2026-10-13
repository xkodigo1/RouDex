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
fn desktop_delivery_creates_a_blocked_task_and_replays_without_execution() {
    let root = temp_root("ingress");
    let body = input("Actualizar el README con instrucciones");
    let response = run(&["route-prompt", "--record-only"], &body, &root, &[]);
    assert_eq!(response["continue"], false);
    assert_eq!(response["decision"], "block");
    assert_eq!(response["status"], "blocked");
    assert_eq!(response["replayed"], false);
    assert!(response.get("runId").is_none());
    let replay = run(&["route-prompt"], &body, &root, &[]);
    assert_eq!(replay["taskId"], response["taskId"]);
    assert_eq!(replay["replayed"], true);
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap();
    let task_id = response["taskId"].as_str().unwrap();
    let task = store.get_task(task_id).unwrap().unwrap().definition;
    assert_eq!(task.prompt, "Actualizar el README con instrucciones");
    assert_eq!(task.profile, "routine");
    assert_eq!(task.approval, "manual");
    assert_eq!(task.sandbox.kind, "readOnly");
    // Approval alone cannot fill in the missing execution contract.
    run(&["task-approve", task_id], "", &root, &[]);
    let dispatch = run(
        &["task-run", task_id],
        "",
        &root,
        &[("ROUDEX_CODEX_BIN", "missing-codex-binary")],
    );
    assert_eq!(dispatch["status"], "blocked");
    assert_eq!(store.count_runs().unwrap(), 0);
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn router_storage_failure_blocks_the_prompt() {
    let root = temp_root("fail-closed");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("RouDex"), "not a directory").unwrap();
    let response = run(&["route-prompt"], &input("Analizar una tarea"), &root, &[]);
    assert_eq!(response["continue"], false);
    assert_eq!(response["decision"], "block");
    assert!(response.get("taskId").is_none());
    fs::remove_dir_all(root).unwrap();
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
            vec!["spec-load", "missing.json"],
            vec!["task-dispatch", "spec"],
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
    let loaded = run(&["spec-load", manifest_arg.as_str()], "", &root, &[]);
    assert_eq!(loaded, response);
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).expect("state store");
    assert_eq!(
        store
            .get_task("T-002")
            .expect("task")
            .expect("exists")
            .spec_id,
        "spec-test"
    );
    // Reloading a stale manifest must not advertise completed work as ready.
    store.set_task_status("T-002", TaskStatus::Ready).unwrap();
    store
        .record_task_approval(&router_core::TaskApproval {
            task_id: "T-002".into(),
            decision: "approved".into(),
            reason: None,
        })
        .unwrap();
    let next = store.get_task("T-002").unwrap().unwrap().definition;
    assert!(store
        .begin_task_run(
            &next,
            "spec-test",
            &RouteReceipt {
                route_id: "reload-route".into(),
                run_id: "reload-run".into(),
                profile: next.profile.clone(),
                model: next.model.clone(),
                effort: next.effort.clone(),
                status: RouteStatus::Accepted,
            },
        )
        .unwrap());
    store
        .finish_task_run("T-002", "reload-run", TaskStatus::Completed, None)
        .unwrap();
    let reloaded = run(&["spec-load", manifest_arg.as_str()], "", &root, &[]);
    assert_eq!(reloaded["readyTaskIds"], serde_json::json!([]));
    assert_eq!(
        store.get_task("T-002").unwrap().unwrap().definition.status,
        TaskStatus::Completed
    );
    drop(store);
    fs::remove_dir_all(root).expect("cleanup");
}

fn assert_invalid_graph_is_not_imported(tasks: Vec<TaskDefinition>, expected_error: &str) {
    let root = temp_root("invalid-spec");
    fs::create_dir_all(&root).unwrap();
    let manifest = router_core::TaskManifest {
        spec_id: "invalid".into(),
        title: "Invalid".into(),
        version: 1,
        tasks,
    };
    let path = root.join("tasks.json");
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let output = Command::new(binary())
        .arg("spec-load")
        .arg(&path)
        .env("LOCALAPPDATA", &root)
        .env_remove("ROUDEX_ROUTER_INTERNAL")
        .env_remove("ROUDEX_ACTIVE_TASK_ID")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(expected_error));
    // Validation must stop before opening the store or reaching dispatch.
    assert!(!root.join("RouDex/router.sqlite3").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn spec_load_rejects_missing_dependency_before_importing_any_tasks() {
    let mut task = lifecycle_task();
    task.depends_on = vec!["missing".into()];
    assert_invalid_graph_is_not_imported(vec![task], "depends on missing task missing");
}

#[test]
fn spec_load_rejects_duplicate_ids_before_importing_any_tasks() {
    let task = lifecycle_task();
    let expected_error = format!("duplicate task id: {}", task.id);
    assert_invalid_graph_is_not_imported(vec![task.clone(), task], &expected_error);
}

#[test]
fn spec_load_rejects_cycle_before_importing_any_tasks() {
    let mut first = lifecycle_task();
    let mut second = first.clone();
    second.id = "cycle-dependent".into();
    first.depends_on = vec![second.id.clone()];
    second.depends_on = vec![first.id.clone()];
    assert_invalid_graph_is_not_imported(vec![first, second], "dependency cycle includes task");
}

#[test]
fn batch_cli_observes_dependencies_approvals_and_exposes_persisted_status() {
    let root = temp_root("batch-cli");
    let db = root.join("RouDex/router.sqlite3");
    let task = lifecycle_task();
    let mut dependent = task.clone();
    dependent.id = "dependent".into();
    dependent.depends_on = vec![task.id.clone()];
    let store = RouterStore::open(&db).unwrap();
    store.upsert_task("spec", &task).unwrap();
    store.upsert_task("spec", &dependent).unwrap();
    let env = [("ROUDEX_CODEX_BIN", "missing-codex-binary")];
    let dispatch = run(&["task-dispatch", "spec"], "", &root, &env);
    assert_eq!(dispatch["receipts"].as_array().unwrap().len(), 1);
    assert_eq!(dispatch["receipts"][0]["status"], "awaiting_approval");
    assert_eq!(store.count_runs().unwrap(), 0);
    run(&["task-approve", &task.id, "denied"], "", &root, &env);
    let denied = run(&["task-dispatch", "spec"], "", &root, &env);
    assert_eq!(denied["receipts"][0]["status"], "blocked");
    assert_eq!(store.count_runs().unwrap(), 0);
    run(&["task-approve", &task.id], "", &root, &env);
    let failed = run(&["task-dispatch", "spec"], "", &root, &env);
    assert_eq!(failed["receipts"][0]["status"], "failed");
    let run_id = failed["receipts"][0]["run_id"].as_str().unwrap();
    let listed = run(&["task-list", "spec"], "", &root, &env);
    assert_eq!(listed["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(listed["tasks"][0]["task"]["status"], "failed");
    assert_eq!(listed["tasks"][0]["approval"]["decision"], "approved");
    assert_eq!(listed["tasks"][0]["latestRun"]["run_id"], run_id);
    assert_eq!(listed["tasks"][1]["dependenciesCompleted"], false);
    assert!(listed["tasks"][1]["latestRun"].is_null());
    assert!(run(&["run-events", run_id], "", &root, &env)
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "thread.start.failed"));
    assert!(run(&["task-dispatch", "spec"], "", &root, &env)["receipts"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(run(&["task-list", "unknown"], "", &root, &env)["tasks"]
        .as_array()
        .unwrap()
        .is_empty());
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn neutral_commands_reject_missing_extra_and_empty_arguments() {
    for args in [
        vec!["spec-load"],
        vec!["spec-load", "a", "b"],
        vec!["task-list"],
        vec!["task-list", ""],
        vec!["task-dispatch"],
        vec!["task-dispatch", "spec", "extra"],
        vec!["task-approve", "task", "approved", "extra"],
    ] {
        let root = temp_root("arguments");
        let output = Command::new(binary())
            .args(&args)
            .env("LOCALAPPDATA", &root)
            .env_remove("ROUDEX_ROUTER_INTERNAL")
            .env_remove("ROUDEX_ACTIVE_TASK_ID")
            .output()
            .unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("Usage:"));
        assert!(!root.exists());
    }
}

#[test]
fn batch_dispatch_rejects_unknown_spec_without_starting_runtime() {
    let root = temp_root("unknown-spec");
    let output = Command::new(binary())
        .args(["task-dispatch", "unknown"])
        .env("LOCALAPPDATA", &root)
        .env("ROUDEX_CODEX_BIN", "missing-codex-binary")
        .env_remove("ROUDEX_ROUTER_INTERNAL")
        .env_remove("ROUDEX_ACTIVE_TASK_ID")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("spec not found"));
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap();
    assert_eq!(store.count_runs().unwrap(), 0);
    drop(store);
    fs::remove_dir_all(root).unwrap();
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
        .contains("route-prompt.ps1"));
    assert!(
        fs::read_to_string(root.join("codex/roudex/route-prompt.ps1"))
            .unwrap()
            .contains("ConvertFrom-Json")
    );
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

#[test]
fn delivery_identity_conflicts_and_invalid_messages_fail_closed() {
    let root = temp_root("invalid-ingress");
    let first = run(&["route-prompt"], &input("Original"), &root, &[]);
    let conflict = run(&["route-prompt"], &input("Changed"), &root, &[]);
    assert_eq!(conflict["decision"], "block");
    assert!(conflict["reason"].as_str().unwrap().contains("identity"));
    for field in ["prompt", "session_id", "turn_id", "cwd", "task_id"] {
        let mut body: Value = serde_json::from_str(&input("New")).unwrap();
        body[field] = " ".into();
        let result = run(&["route-prompt"], &body.to_string(), &root, &[]);
        assert_eq!(result["continue"], false);
        assert!(result.get("taskId").is_none());
    }
    for body in ["", "not JSON", "{}"] {
        assert_eq!(
            run(&["route-prompt"], body, &root, &[])["decision"],
            "block"
        );
    }
    assert_eq!(
        run(
            &["route-prompt", "--execute"],
            &input("Original"),
            &root,
            &[]
        )["decision"],
        "block"
    );
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap();
    assert_eq!(
        store
            .tasks_for_spec(first["specId"].as_str().unwrap())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .get_task(first["taskId"].as_str().unwrap())
            .unwrap()
            .unwrap()
            .definition
            .prompt,
        "Original"
    );
    assert_eq!(store.count_runs().unwrap(), 0);
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn desktop_update_preserves_contract_revokes_approval_and_rejects_stale_dispatch() {
    let root = temp_root("update-ingress");
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap();
    let task = lifecycle_task();
    store.upsert_task("spec", &task).unwrap();
    run(&["task-approve", &task.id], "", &root, &[]);
    let mut body: Value = serde_json::from_str(&input("Updated task prompt")).unwrap();
    body["task_id"] = task.id.clone().into();
    let result = run(&["route-prompt"], &body.to_string(), &root, &[]);
    assert_eq!(result["taskId"], task.id);
    assert_eq!(result["status"], "blocked");
    let updated = store.get_task(&task.id).unwrap().unwrap().definition;
    assert_eq!(updated.prompt, "Updated task prompt");
    assert_eq!(updated.scope, task.scope);
    assert_eq!(updated.sandbox, task.sandbox);
    assert_eq!(updated.verify, task.verify);
    assert_eq!(updated.profile, task.profile);
    assert_eq!(
        store
            .latest_task_approval(&task.id)
            .unwrap()
            .unwrap()
            .decision,
        "denied"
    );
    let env = [("ROUDEX_CODEX_BIN", "missing-codex-binary")];
    assert_eq!(
        run(&["task-run", &task.id], "", &root, &env)["status"],
        "blocked"
    );
    run(&["task-approve", &task.id], "", &root, &[]);
    let receipt = RouteReceipt {
        route_id: "stale-route".into(),
        run_id: "stale-run".into(),
        profile: task.profile.clone(),
        model: task.model.clone(),
        effort: task.effort.clone(),
        status: RouteStatus::Accepted,
    };
    assert!(!store.begin_task_run(&task, "spec", &receipt).unwrap());
    assert_eq!(store.count_runs().unwrap(), 0);
    // Replaying the delivery never revokes a later, explicit approval.
    assert_eq!(
        run(&["route-prompt"], &body.to_string(), &root, &[])["replayed"],
        true
    );
    assert_eq!(
        store
            .latest_task_approval(&task.id)
            .unwrap()
            .unwrap()
            .decision,
        "approved"
    );
    let failed = run(&["task-run", &task.id], "", &root, &env);
    assert_eq!(failed["status"], "failed");
    assert_eq!(store.count_runs().unwrap(), 1);
    let evidence = store
        .events_for_run(failed["run_id"].as_str().unwrap())
        .unwrap();
    assert!(evidence
        .iter()
        .any(|event| event.event_type == "task.started"
            && event.payload["definition"]["prompt"] == "Updated task prompt"));
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ingress_cannot_overwrite_active_or_terminal_work_or_cross_working_directories() {
    for status in [
        TaskStatus::Running,
        TaskStatus::Completed,
        TaskStatus::Failed,
        TaskStatus::Interrupted,
        TaskStatus::Blocked,
    ] {
        let root = temp_root("immutable-ingress");
        let store = RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap();
        let task = lifecycle_task();
        store.upsert_task("spec", &task).unwrap();
        run(&["task-approve", &task.id], "", &root, &[]);
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
        if status != TaskStatus::Running {
            store.set_task_status(&task.id, status.clone()).unwrap();
        }
        let mut body: Value = serde_json::from_str(&input("Must not overwrite")).unwrap();
        body["task_id"] = task.id.clone().into();
        let response = run(&["route-prompt"], &body.to_string(), &root, &[]);
        assert!(response.get("taskId").is_none());
        assert_eq!(
            store.get_task(&task.id).unwrap().unwrap().definition.prompt,
            task.prompt
        );
        assert_eq!(
            store.get_task(&task.id).unwrap().unwrap().definition.status,
            status
        );
        assert_eq!(store.count_runs().unwrap(), 1);
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
    let root = temp_root("cross-cwd");
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap();
    let task = lifecycle_task();
    store.upsert_task("spec", &task).unwrap();
    let mut body: Value = serde_json::from_str(&input("Must not overwrite")).unwrap();
    body["cwd"] = root.to_string_lossy().as_ref().into();
    body["task_id"] = task.id.clone().into();
    assert!(
        run(&["route-prompt"], &body.to_string(), &root, &[])["reason"]
            .as_str()
            .unwrap()
            .contains("cwd")
    );
    body["task_id"] = "missing".into();
    assert!(
        run(&["route-prompt"], &body.to_string(), &root, &[])["reason"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_desktop_deliveries_persist_one_task() {
    let root = temp_root("concurrent-ingress");
    // Initialize the schema before testing concurrent ingress transactions.
    drop(RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap());
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let root = root.clone();
            std::thread::spawn(move || {
                run(&["route-prompt"], &input("Concurrent delivery"), &root, &[])
            })
        })
        .collect();
    let responses: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    let task_id = responses[0]["taskId"].as_str().expect("task id");
    assert_eq!(
        responses
            .iter()
            .filter(|response| response["replayed"] == false)
            .count(),
        1
    );
    for response in &responses {
        assert_eq!(response["taskId"], task_id);
        assert_eq!(response["continue"], false);
    }
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap();
    assert_eq!(
        store
            .tasks_for_spec(responses[0]["specId"].as_str().unwrap())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(store.count_runs().unwrap(), 0);
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
fn run_hook_script(router: &Path, body: &str, root: &Path, internal: bool) -> Value {
    let mut command = Command::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/route-prompt.ps1"))
        .arg("-RouterPath")
        .arg(router)
        .env("LOCALAPPDATA", root)
        .env_remove("ROUDEX_ROUTER_INTERNAL")
        .env_remove("ROUDEX_ACTIVE_TASK_ID")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if internal {
        command.env("ROUDEX_ROUTER_INTERNAL", "1");
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(body.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("fail-closed wrapper JSON")
}

#[cfg(windows)]
#[test]
fn powershell_boundary_blocks_missing_failed_and_malformed_routers() {
    let root = temp_root("wrapper");
    fs::create_dir_all(&root).unwrap();
    assert_eq!(
        run_hook_script(&root.join("missing.exe"), &input("Task"), &root, false)["decision"],
        "block"
    );
    let fake = root.join("fake.cmd");
    for output in [
        "@exit /b 1",
        "@echo garbage",
        "@echo {\"continue\":true}",
        "@echo {\"continue\":false,\"decision\":[\"block\",\"allow\"]}",
        "@echo {\"continue\":\"false\",\"decision\":\"block\"}",
        "@echo [{\"continue\":false,\"decision\":\"block\"}]",
        "@exit /b 0",
    ] {
        fs::write(&fake, output).unwrap();
        assert_eq!(
            run_hook_script(&fake, &input("Task"), &root, false)["decision"],
            "block"
        );
    }
    // Internal guard must work even when the incoming body is malformed.
    assert_eq!(
        run_hook_script(&binary(), "not JSON", &root, true),
        serde_json::json!({"continue":true})
    );
    let prompt = "Actualizar documentación: ejecución única 日本語";
    let response = run_hook_script(&binary(), &input(prompt), &root, false);
    assert_eq!(response["decision"], "block");
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap();
    assert_eq!(
        store
            .get_task(response["taskId"].as_str().unwrap())
            .unwrap()
            .unwrap()
            .definition
            .prompt,
        prompt
    );
    drop(store);
    fs::remove_dir_all(root).unwrap();
}
