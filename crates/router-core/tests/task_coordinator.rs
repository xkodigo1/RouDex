use router_core::{
    AppServerConfig, CoordinatorError, EventRecord, LocalTaskRuntime, RouterStore, TaskApproval,
    TaskCoordinator, TaskDefinition, TaskSandbox, TaskStatus,
};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

struct Fixture {
    cwd: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let cwd = std::env::temp_dir().join(format!("roudex-coordinator-{}", Uuid::new_v4()));
        fs::create_dir_all(&cwd).unwrap();
        Self { cwd }
    }

    fn task(&self, id: &str) -> TaskDefinition {
        TaskDefinition {
            id: id.into(),
            title: "Coordinated task".into(),
            prompt: "Perform the local task".into(),
            depends_on: vec![],
            status: TaskStatus::Planned,
            profile: "deep".into(),
            model: "gpt-6.1-sol".into(),
            effort: "high".into(),
            approval: "not_required".into(),
            scope: vec![".".into()],
            cwd: self.cwd.to_string_lossy().into_owned(),
            sandbox: TaskSandbox {
                kind: "workspaceWrite".into(),
                writable_roots: vec![".".into()],
            },
            acceptance: vec!["Verification succeeds".into()],
            verify: "echo verified".into(),
        }
    }

    fn runtime(&self, scenario: &str) -> LocalTaskRuntime {
        LocalTaskRuntime::with_config(AppServerConfig {
            program: PathBuf::from(env!("CARGO_BIN_EXE_fake-app-server")),
            arguments: vec![
                scenario.into(),
                self.log_path().to_string_lossy().into_owned(),
            ],
            working_directory: self.cwd.clone(),
            service_name: "coordinator-test".into(),
            active_task_id: Some("T-004".into()),
        })
    }

    fn log_path(&self) -> PathBuf {
        self.cwd.join("requests.jsonl")
    }

    fn requests(&self, method: &str) -> Vec<Value> {
        fs::read_to_string(self.log_path())
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .filter(|entry| entry["message"]["method"] == method)
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.cwd);
    }
}

#[test]
fn stdio_dispatch_persists_route_turn_deltas_and_independent_verification() {
    let fixture = Fixture::new();
    let db = fixture.cwd.join("router.sqlite3");
    let receipt = {
        let store = RouterStore::open(&db).unwrap();
        store.upsert_task("spec", &fixture.task("T-004")).unwrap();
        TaskCoordinator::new(&store)
            .dispatch_task("T-004", &mut fixture.runtime("success"))
            .unwrap()
    };
    assert_eq!(receipt.status, TaskStatus::Completed);
    let store = RouterStore::open(&db).unwrap();
    let run_id = receipt.run_id.unwrap();
    let run = store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(Some(run.route_id), receipt.route_id);
    assert_eq!(
        (
            run.profile.as_str(),
            run.model.as_str(),
            run.effort.as_str()
        ),
        ("deep", "gpt-6.1-sol", "high")
    );
    assert_eq!(run.status, "completed");
    assert_eq!(
        store.get_task("T-004").unwrap().unwrap().definition.status,
        TaskStatus::Completed
    );
    let events = store.events_for_run(&run_id).unwrap();
    for name in [
        "task.started",
        "thread.started",
        "turn/started",
        "item/agentMessage/delta",
        "turn/completed",
        "turn.completed",
        "task.verified",
    ] {
        assert!(
            events.iter().any(|event| event.event_type == name),
            "missing {name}"
        );
    }
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == "turn/completed")
            .count(),
        1
    );
    assert!(!events
        .iter()
        .any(|event| event.payload["delta"] == "ignore me"));
    let starts = fixture.requests("thread/start");
    assert_eq!(starts.len(), 1);
    assert_eq!(starts[0]["internal"], "1");
    assert_eq!(starts[0]["activeTask"], "T-004");
    let params = &starts[0]["message"]["params"];
    assert_eq!(params["model"], "gpt-6.1-sol");
    assert_eq!(params["config"]["model_reasoning_effort"], "high");
    let turns = fixture.requests("turn/start");
    let params = &turns[0]["message"]["params"];
    assert_eq!(params["model"], "gpt-6.1-sol");
    assert_eq!(params["effort"], "high");
    assert_eq!(params["approvalPolicy"], "never");
    assert_eq!(params["sandboxPolicy"]["networkAccess"], false);
    assert_eq!(
        params["sandboxPolicy"]["writableRoots"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let verify = fixture.requests("command/exec");
    assert_eq!(verify.len(), 1);
    assert_eq!(
        verify[0]["message"]["params"]["sandboxPolicy"],
        params["sandboxPolicy"]
    );
    assert_eq!(
        verify[0]["message"]["params"]["env"]["ROUDEX_ACTIVE_TASK_ID"],
        "T-004"
    );
    let replay = TaskCoordinator::new(&store)
        .dispatch_task("T-004", &mut fixture.runtime("success"))
        .unwrap();
    assert_eq!(replay.run_id.as_deref(), Some(run_id.as_str()));
    assert_eq!(fixture.requests("turn/start").len(), 1);
}

#[test]
fn batch_rechecks_dependencies_and_never_constructs_unapproved_runtime() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    let mut dependent = fixture.task("A-dependent");
    dependent.depends_on = vec!["Z-first".into()];
    store.upsert_task("spec", &dependent).unwrap();
    store.upsert_task("spec", &fixture.task("Z-first")).unwrap();
    let mut manual = fixture.task("M-manual");
    manual.approval = "manual".into();
    store.upsert_task("spec", &manual).unwrap();
    let mut factories = Vec::new();
    let receipts = TaskCoordinator::new(&store)
        .dispatch_ready_tasks_with_runtime("spec", |task| {
            factories.push(task.id.clone());
            Ok(fixture.runtime("success"))
        })
        .unwrap();
    assert_eq!(factories, ["Z-first", "A-dependent"]);
    assert_eq!(
        receipts
            .iter()
            .filter(|r| r.status == TaskStatus::Completed)
            .count(),
        2
    );
    assert!(receipts
        .iter()
        .any(|r| r.task_id == "M-manual" && r.status == TaskStatus::AwaitingApproval));
    assert_eq!(store.count_runs().unwrap(), 2);
    assert!(TaskCoordinator::new(&store)
        .dispatch_ready_tasks_with_runtime("spec", |_| {
            panic!("completed and unapproved tasks must not construct runtimes");
            #[allow(unreachable_code)]
            Ok(fixture.runtime("success"))
        })
        .unwrap()
        .iter()
        .all(|r| r.status == TaskStatus::AwaitingApproval));
}

#[test]
fn missing_dependency_prevents_runtime_and_route_creation() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    let mut task = fixture.task("T-004");
    task.depends_on = vec!["missing".into()];
    store.upsert_task("spec", &task).unwrap();
    let receipt = TaskCoordinator::new(&store)
        .dispatch_task(&task.id, &mut fixture.runtime("success"))
        .unwrap();
    assert_eq!(receipt.status, TaskStatus::Blocked);
    assert_eq!(store.count_runs().unwrap(), 0);
    assert!(!fixture.log_path().exists());
}

#[test]
fn route_conflict_is_recorded_without_constructing_runtime() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    let mut task = fixture.task("T-004");
    task.effort = "low".into();
    store.upsert_task("spec", &task).unwrap();
    let receipts = TaskCoordinator::new(&store)
        .dispatch_ready_tasks_with_runtime::<LocalTaskRuntime, _>("spec", |_| {
            panic!("invalid route")
        })
        .unwrap();
    assert_eq!(receipts[0].status, TaskStatus::Blocked);
    let events = store
        .events_for_run(receipts[0].run_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(events[0].event_type, "task.blocked");
    assert!(!fixture.log_path().exists());
}

#[test]
fn runtime_factory_failure_is_persisted_and_does_not_abort_independent_tasks() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    for id in ["A-fails", "B-succeeds"] {
        store.upsert_task("spec", &fixture.task(id)).unwrap();
    }
    let receipts = TaskCoordinator::new(&store)
        .dispatch_ready_tasks_with_runtime("spec", |task| {
            if task.id == "A-fails" {
                Err(CoordinatorError::Runtime("missing executable".into()))
            } else {
                Ok(fixture.runtime("success"))
            }
        })
        .unwrap();
    assert_eq!(receipts[0].status, TaskStatus::Failed);
    assert_eq!(receipts[1].status, TaskStatus::Completed);
    let events = store
        .events_for_run(receipts[0].run_id.as_ref().unwrap())
        .unwrap();
    assert!(events.iter().any(|e| e.event_type == "thread.start.failed"));
}

#[test]
fn protocol_errors_and_verification_failure_never_complete_tasks() {
    for (scenario, expected) in [
        ("model-rejected", TaskStatus::Failed),
        ("turn-failed", TaskStatus::Failed),
        ("verify-failed", TaskStatus::Failed),
        ("disconnect", TaskStatus::Interrupted),
        ("turn-interrupted", TaskStatus::Interrupted),
    ] {
        let fixture = Fixture::new();
        let store = RouterStore::in_memory().unwrap();
        store.upsert_task("spec", &fixture.task("T-004")).unwrap();
        let receipt = TaskCoordinator::new(&store)
            .dispatch_task("T-004", &mut fixture.runtime(scenario))
            .unwrap();
        assert_eq!(receipt.status, expected, "{scenario}");
        let run = store
            .get_run(receipt.run_id.as_ref().unwrap())
            .unwrap()
            .unwrap();
        assert_ne!(run.status, "completed");
        if scenario != "verify-failed" {
            assert!(fixture.requests("command/exec").is_empty());
        }
    }
}

#[test]
fn approval_unblocks_dispatch_without_starting_an_early_worker() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    let mut task = fixture.task("T-004");
    task.approval = "manual".into();
    store.upsert_task("spec", &task).unwrap();
    let coordinator = TaskCoordinator::new(&store);
    let pending = coordinator
        .dispatch_task(&task.id, &mut fixture.runtime("success"))
        .unwrap();
    assert_eq!(pending.status, TaskStatus::AwaitingApproval);
    assert!(!fixture.log_path().exists());
    store
        .record_task_approval(&TaskApproval {
            task_id: task.id.clone(),
            decision: "approved".into(),
            reason: Some("test decision".into()),
        })
        .unwrap();
    let done = coordinator
        .dispatch_task(&task.id, &mut fixture.runtime("success"))
        .unwrap();
    assert_eq!(done.status, TaskStatus::Completed);
    assert_eq!(fixture.requests("thread/start").len(), 1);
}

#[test]
fn resume_reuses_run_and_thread_and_is_idempotent_after_completion() {
    for scenario in [
        "success",
        "resume-completed",
        "resume-missing",
        "resume-wrong-turn",
    ] {
        let fixture = Fixture::new();
        let store = RouterStore::in_memory().unwrap();
        store.upsert_task("spec", &fixture.task("T-004")).unwrap();
        let coordinator = TaskCoordinator::new(&store);
        let interrupted = coordinator
            .dispatch_task("T-004", &mut fixture.runtime("disconnect"))
            .unwrap();
        let run_id = interrupted.run_id.as_ref().unwrap();
        assert_eq!(interrupted.status, TaskStatus::Interrupted);
        let resumed = coordinator
            .resume_run(run_id, &mut fixture.runtime(scenario))
            .unwrap();
        assert_eq!(resumed.run_id, interrupted.run_id);
        assert_eq!(resumed.route_id, interrupted.route_id);
        assert_eq!(fixture.requests("thread/start").len(), 1);
        assert_eq!(fixture.requests("thread/resume").len(), 1);
        assert_eq!(
            fixture.requests("turn/start").len(),
            if scenario == "success" { 2 } else { 1 }
        );
        assert_eq!(
            resumed.status,
            if matches!(scenario, "resume-missing" | "resume-wrong-turn") {
                TaskStatus::Blocked
            } else {
                TaskStatus::Completed
            }
        );
        let events_before = store.events_for_run(run_id).unwrap();
        coordinator
            .resume_run(run_id, &mut fixture.runtime(scenario))
            .unwrap();
        assert_eq!(store.events_for_run(run_id).unwrap(), events_before);
        assert_eq!(fixture.requests("thread/resume").len(), 1);
        assert_eq!(store.count_runs().unwrap(), 1);
        let receipts = coordinator
            .dispatch_ready_tasks_with_runtime::<LocalTaskRuntime, _>("spec", |_| {
                panic!("failed recovery must not replace the thread in a later batch")
            })
            .unwrap();
        assert!(receipts.iter().all(|r| r.status == TaskStatus::Blocked));
    }
}

#[test]
fn durable_terminal_evidence_prevents_replay_after_a_crash() {
    for raw in [false, true] {
        let fixture = Fixture::new();
        let store = RouterStore::in_memory().unwrap();
        store.upsert_task("spec", &fixture.task("T-004")).unwrap();
        let coordinator = TaskCoordinator::new(&store);
        let interrupted = coordinator
            .dispatch_task("T-004", &mut fixture.runtime("disconnect"))
            .unwrap();
        let run_id = interrupted.run_id.unwrap();
        store.record_event(&EventRecord {
            run_id: run_id.clone(),
            event_type: if raw { "turn/completed" } else { "turn.completed" }.into(),
            payload: if raw {
                json!({ "threadId": "fake-thread", "turn": { "id": "fake-turn", "status": "completed" } })
            } else {
                json!({ "threadId": "fake-thread", "turnId": "fake-turn", "status": "completed", "text": "durable completion" })
            },
        }).unwrap();
        let receipt = coordinator
            .resume_run(&run_id, &mut fixture.runtime("success"))
            .unwrap();
        assert_eq!(receipt.status, TaskStatus::Completed);
        assert_eq!(
            fixture.requests("turn/start").len(),
            1,
            "durable turn must not replay"
        );
        assert_eq!(fixture.requests("command/exec").len(), 1);
        assert_eq!(
            store
                .events_for_run(&run_id)
                .unwrap()
                .iter()
                .filter(|e| e.event_type == "turn.completed")
                .count(),
            1
        );
    }
}
