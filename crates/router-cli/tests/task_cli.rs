use router_core::{
    resolve_task_route, EventRecord, RiskRules, RouteDecision, RouteReceipt, RouteStatus,
    RouterConfig, RouterStore, TaskDefinition, TaskSandbox, TaskStatus, Tier,
};
use serde_json::Value;
use std::fs;
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
    feed(child.stdin.take(), body.as_bytes());
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
            assert!(!output.status.success());
            assert!(String::from_utf8_lossy(&output.stderr).contains("worker activo"));
            assert!(!root.exists(), "internal command must not touch state");
        }
    }
}

fn lifecycle_task() -> TaskDefinition {
    TaskDefinition {
        id: "T-lifecycle".into(),
        title: "Lifecycle".into(),
        prompt: "Original task".into(),
        depends_on: vec![],
        status: TaskStatus::Ready,
        tier: Tier::Estandar,
        profile: None,
        model: String::new(),
        effort: String::new(),
        milestone: None,
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
                &task,
                "spec",
                &receipt("route", "run", &route(&task)),
                &route(&task),
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
            &next,
            "spec-test",
            &receipt("reload-route", "reload-run", &route(&next)),
            &route(&next),
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
        milestones: Vec::new(),
        tasks,
        warnings: Vec::new(),
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

/// The route the coordinator would give a task with the default tiers.
fn route(task: &TaskDefinition) -> RouteDecision {
    resolve_task_route(task, None, &RouterConfig::default(), &RiskRules::default())
}

fn receipt(route_id: &str, run_id: &str, route: &RouteDecision) -> RouteReceipt {
    RouteReceipt {
        route_id: route_id.into(),
        run_id: run_id.into(),
        profile: route.tier.as_str().into(),
        model: route.model.clone(),
        effort: route.effort.clone(),
        status: RouteStatus::Accepted,
    }
}

#[test]
fn spec_load_translates_legacy_profiles_and_reports_it() {
    let root = temp_root("legacy-spec");
    fs::create_dir_all(&root).unwrap();
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/003-desktop-hook/tasks.json")
        .to_string_lossy()
        .into_owned();
    let response = run(&["spec-load", manifest.as_str()], "", &root, &[]);
    assert_eq!(response["specId"], "003-desktop-hook");
    let warnings = response["warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|warning| warning.as_str().unwrap().contains("tier estandar")));
    let store = RouterStore::open(root.join("RouDex/router.sqlite3")).unwrap();
    let task = store.get_task("R003-T001").unwrap().unwrap().definition;
    assert_eq!(task.tier, Tier::Estandar);
    assert!(task.model.is_empty(), "router.toml chooses the model");
    drop(store);
    fs::remove_dir_all(root).unwrap();
}
