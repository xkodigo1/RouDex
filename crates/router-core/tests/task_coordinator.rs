use router_core::{
    resolve_task_route, AppServerConfig, CoordinatorError, EventRecord, LocalTaskRuntime,
    RiskRules, RouteReceipt, RouteStatus, RouterConfig, RouterStore, TaskApproval, TaskCoordinator,
    TaskDefinition, TaskSandbox, TaskStatus, Tier,
};
use router_core::{ApprovalDecision, EventSink, SessionEvent, TaskRuns};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
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
            tier: Tier::Fuerte,
            profile: None,
            model: String::new(),
            effort: String::new(),
            milestone: None,
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
        LocalTaskRuntime::with_config(self.config(scenario))
    }

    fn config(&self, scenario: &str) -> AppServerConfig {
        AppServerConfig {
            program: PathBuf::from(env!("CARGO_BIN_EXE_fake-app-server")),
            arguments: vec![
                scenario.into(),
                self.log_path().to_string_lossy().into_owned(),
            ],
            working_directory: self.cwd.clone(),
            service_name: "coordinator-test".into(),
            active_task_id: Some("T-004".into()),
        }
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
            .with_router(RouterConfig::default())
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
        ("fuerte", "gpt-6.1-sol", "high")
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
    assert_eq!(params["approvalPolicy"], "on-request");
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
        .with_router(RouterConfig::default())
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
        .with_router(RouterConfig::default())
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
        .with_router(RouterConfig::default())
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
        .with_router(RouterConfig::default())
        .dispatch_task(&task.id, &mut fixture.runtime("success"))
        .unwrap();
    assert_eq!(receipt.status, TaskStatus::Blocked);
    assert_eq!(store.count_runs().unwrap(), 0);
    assert!(!fixture.log_path().exists());
}

#[test]
fn invalid_project_risk_rules_block_without_constructing_runtime() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.cwd.join(".roudex")).unwrap();
    fs::write(
        fixture.cwd.join(".roudex").join("rules.toml"),
        "keywords = 3",
    )
    .unwrap();
    let store = RouterStore::in_memory().unwrap();
    store.upsert_task("spec", &fixture.task("T-004")).unwrap();
    let receipts = TaskCoordinator::new(&store)
        .with_router(RouterConfig::default())
        .dispatch_ready_tasks_with_runtime::<LocalTaskRuntime, _>("spec", |_| {
            panic!("an unroutable task must not start a runtime")
        })
        .unwrap();
    assert_eq!(receipts[0].status, TaskStatus::Blocked);
    let events = store
        .events_for_run(receipts[0].run_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(events[0].event_type, "task.blocked");
    assert!(events[0].payload["reason"]
        .as_str()
        .unwrap()
        .contains("rules.toml"));
    assert!(!fixture.log_path().exists());
}

/// Model, effort and route the coordinator recorded when the task started.
fn started(store: &RouterStore, run_id: &str) -> Value {
    store
        .events_for_run(run_id)
        .unwrap()
        .into_iter()
        .find(|event| event.event_type == "task.started")
        .expect("task.started")
        .payload
}

#[test]
fn runs_record_the_model_router_toml_had_at_dispatch() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    let mut first = fixture.task("T-001");
    first.tier = Tier::Estandar;
    let mut second = fixture.task("T-002");
    second.tier = Tier::Estandar;
    store.upsert_task("spec", &first).unwrap();
    store.upsert_task("spec", &second).unwrap();

    let before = TaskCoordinator::new(&store)
        .with_router(RouterConfig::default())
        .dispatch_task("T-001", &mut fixture.runtime("success"))
        .unwrap();
    let mut edited = RouterConfig::default();
    edited.tiers.estandar.model = "gpt-6-astra".into();
    edited.tiers.estandar.effort = "low".into();
    let after = TaskCoordinator::new(&store)
        .with_router(edited)
        .dispatch_task("T-002", &mut fixture.runtime("success"))
        .unwrap();

    let route = |receipt: &router_core::TaskExecutionReceipt| {
        let run = store
            .get_run(receipt.run_id.as_ref().unwrap())
            .unwrap()
            .unwrap();
        (run.profile, run.model, run.effort)
    };
    assert_eq!(
        route(&before),
        ("estandar".into(), "gpt-6.1-sol".into(), "medium".into())
    );
    assert_eq!(
        route(&after),
        ("estandar".into(), "gpt-6-astra".into(), "low".into())
    );
    let payload = started(&store, after.run_id.as_ref().unwrap());
    assert_eq!(payload["definition"]["model"], "gpt-6-astra");
    assert_eq!(payload["route"]["source"], "spec");
    // The stored task still names no model: the next dispatch routes again.
    assert!(store
        .get_task("T-002")
        .unwrap()
        .unwrap()
        .definition
        .model
        .is_empty());
}

#[test]
fn risk_forces_fuerte_and_a_manual_tier_wins_on_the_record() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    let mut risky = fixture.task("T-001");
    risky.tier = Tier::Economico;
    risky.prompt = "Aplica la migración de la base de datos en producción".into();
    let mut manual = risky.clone();
    manual.id = "T-002".into();
    store.upsert_task("spec", &risky).unwrap();
    store.upsert_task("spec", &manual).unwrap();
    assert!(store
        .set_task_tier_override("T-002", Some(Tier::Economico))
        .unwrap());

    // The fake app server only accepts the fuerte model; the tier and its
    // source are what this test checks.
    let mut config = RouterConfig::default();
    config.tiers.economico = config.tiers.fuerte.clone();
    let coordinator = TaskCoordinator::new(&store).with_router(config);
    let forced = coordinator
        .dispatch_task("T-001", &mut fixture.runtime("success"))
        .unwrap();
    let forced = started(&store, forced.run_id.as_ref().unwrap());
    assert_eq!(forced["tier"], "fuerte");
    assert_eq!(forced["route"]["source"], "risk");
    assert!(forced["route"]["reason"]
        .as_str()
        .unwrap()
        .contains("migracion"));

    let chosen = coordinator
        .dispatch_task("T-002", &mut fixture.runtime("success"))
        .unwrap();
    assert_eq!(chosen.status, TaskStatus::Completed);
    let chosen = started(&store, chosen.run_id.as_ref().unwrap());
    assert_eq!(chosen["tier"], "economico");
    assert_eq!(chosen["route"]["source"], "manual");
    assert!(!chosen["route"]["risk"].as_array().unwrap().is_empty());
    // A finished task keeps the tier it ran with.
    assert!(!store.set_task_tier_override("T-002", None).unwrap());
}

#[test]
fn runtime_factory_failure_is_persisted_and_does_not_abort_independent_tasks() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    for id in ["A-fails", "B-succeeds"] {
        store.upsert_task("spec", &fixture.task(id)).unwrap();
    }
    let receipts = TaskCoordinator::new(&store)
        .with_router(RouterConfig::default())
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
            .with_router(RouterConfig::default())
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
        } else {
            let events = store.events_for_run(&run.run_id).unwrap();
            let failure = events
                .iter()
                .find(|event| event.event_type == "verification.failed")
                .unwrap();
            assert_eq!(failure.payload["result"]["exitCode"], 1);
            assert_eq!(failure.payload["command"], "echo verified");
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
    let coordinator = TaskCoordinator::new(&store).with_router(RouterConfig::default());
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
        "resume-wrong-interrupted-turn",
        "resume-empty-history",
        "resume-missing-turn-id",
    ] {
        let fixture = Fixture::new();
        let store = RouterStore::in_memory().unwrap();
        store.upsert_task("spec", &fixture.task("T-004")).unwrap();
        let coordinator = TaskCoordinator::new(&store).with_router(RouterConfig::default());
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
            if matches!(scenario, "success" | "resume-completed") {
                TaskStatus::Completed
            } else {
                TaskStatus::Blocked
            }
        );
        let events_before = store.events_for_run(run_id).unwrap();
        if resumed.status == TaskStatus::Blocked {
            assert_eq!(
                events_before
                    .iter()
                    .filter(|event| event.event_type == "thread.resume.failed")
                    .count(),
                1
            );
            assert!(fixture.requests("command/exec").is_empty());
        }
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
fn superseded_interrupted_run_cannot_resume_or_change_the_current_run() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    store.upsert_task("spec", &fixture.task("T-004")).unwrap();
    let coordinator = TaskCoordinator::new(&store).with_router(RouterConfig::default());
    let first = coordinator
        .dispatch_task("T-004", &mut fixture.runtime("disconnect"))
        .unwrap();
    coordinator.retry_interrupted_task("T-004").unwrap();
    let second = coordinator
        .dispatch_task("T-004", &mut fixture.runtime("disconnect"))
        .unwrap();
    assert_ne!(first.run_id, second.run_id);
    let first_run_id = first.run_id.unwrap();
    let second_run_id = second.run_id.unwrap();
    let before = store.events_for_run(&first_run_id).unwrap();
    let error = coordinator
        .resume_run(&first_run_id, &mut fixture.runtime("success"))
        .unwrap_err();
    assert!(error.to_string().contains("superseded"));
    assert!(fixture.requests("thread/resume").is_empty());
    assert_eq!(fixture.requests("turn/start").len(), 2);
    assert_eq!(store.events_for_run(&first_run_id).unwrap(), before);
    assert_eq!(
        store.get_run(&second_run_id).unwrap().unwrap().status,
        "interrupted"
    );
    assert_eq!(
        store.get_task("T-004").unwrap().unwrap().definition.status,
        TaskStatus::Interrupted
    );
}

#[test]
fn resume_uses_the_immutable_assignment_when_the_manifest_changes() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    let mut task = fixture.task("T-004");
    store.upsert_task("spec", &task).unwrap();
    let coordinator = TaskCoordinator::new(&store).with_router(RouterConfig::default());
    let first = coordinator
        .dispatch_task("T-004", &mut fixture.runtime("disconnect"))
        .unwrap();
    task.tier = Tier::Economico;
    task.prompt = "An unrelated new instruction".into();
    task.verify = "An unrelated new verification".into();
    store.upsert_task("spec", &task).unwrap();
    let resumed = coordinator
        .resume_run(
            first.run_id.as_ref().unwrap(),
            &mut fixture.runtime("success"),
        )
        .unwrap();
    assert_eq!(resumed.status, TaskStatus::Completed);
    assert_eq!(resumed.route_id, first.route_id);
    assert_eq!(resumed.run_id, first.run_id);
    let turns = fixture.requests("turn/start");
    assert_eq!(turns[1]["message"]["params"]["model"], "gpt-6.1-sol");
    assert_eq!(turns[1]["message"]["params"]["effort"], "high");
    assert_eq!(
        turns[1]["message"]["params"]["input"][0]["text"],
        "Perform the local task"
    );
    let verification = fixture.requests("command/exec");
    assert_eq!(
        verification[0]["message"]["params"]["env"]["ROUDEX_VERIFY"],
        "echo verified"
    );
}

#[test]
fn durable_terminal_evidence_prevents_replay_after_a_crash() {
    for raw in [false, true] {
        let fixture = Fixture::new();
        let store = RouterStore::in_memory().unwrap();
        store.upsert_task("spec", &fixture.task("T-004")).unwrap();
        let coordinator = TaskCoordinator::new(&store).with_router(RouterConfig::default());
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

#[test]
fn denial_stays_blocked_until_an_explicit_approval() {
    let fixture = Fixture::new();
    let db = fixture.cwd.join("router.sqlite3");
    let store = RouterStore::open(&db).unwrap();
    let mut task = fixture.task("T-004");
    task.approval = "manual".into();
    store.upsert_task("spec", &task).unwrap();
    let coordinator = TaskCoordinator::new(&store).with_router(RouterConfig::default());
    coordinator
        .dispatch_task(&task.id, &mut fixture.runtime("success"))
        .unwrap();
    store
        .record_task_approval(&TaskApproval {
            task_id: task.id.clone(),
            decision: "denied".into(),
            reason: Some("review required".into()),
        })
        .unwrap();
    assert_eq!(
        store.get_task(&task.id).unwrap().unwrap().definition.status,
        TaskStatus::Blocked
    );
    for _ in 0..2 {
        let receipts = coordinator
            .dispatch_ready_tasks_with_runtime::<LocalTaskRuntime, _>("spec", |_| {
                panic!("denied task started")
            })
            .unwrap();
        assert_eq!(receipts[0].status, TaskStatus::Blocked);
    }
    assert_eq!(store.count_runs().unwrap(), 0);
    assert!(!fixture.log_path().exists());
    drop(store);
    let store = RouterStore::open(&db).unwrap();
    assert_eq!(
        store
            .latest_task_approval(&task.id)
            .unwrap()
            .unwrap()
            .reason
            .as_deref(),
        Some("review required")
    );
    assert!(store
        .record_task_approval(&TaskApproval {
            task_id: task.id.clone(),
            decision: "yes".into(),
            reason: None,
        })
        .is_err());
    assert_eq!(
        store
            .latest_task_approval(&task.id)
            .unwrap()
            .unwrap()
            .decision,
        "denied"
    );
    store
        .record_task_approval(&TaskApproval {
            task_id: task.id.clone(),
            decision: "approved".into(),
            reason: Some("reviewed".into()),
        })
        .unwrap();
    let done = TaskCoordinator::new(&store)
        .with_router(RouterConfig::default())
        .dispatch_task(&task.id, &mut fixture.runtime("success"))
        .unwrap();
    assert_eq!(done.status, TaskStatus::Completed);
    assert_eq!(fixture.requests("thread/start").len(), 1);
}

#[test]
fn resume_claim_rechecks_revoked_approval_dependencies_and_latest_run() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    let mut task = fixture.task("T-004");
    task.approval = "manual".into();
    store.upsert_task("spec", &task).unwrap();
    let approve = |decision: &str| {
        store
            .record_task_approval(&TaskApproval {
                task_id: task.id.clone(),
                decision: decision.into(),
                reason: None,
            })
            .unwrap()
    };
    approve("approved");
    let coordinator = TaskCoordinator::new(&store).with_router(RouterConfig::default());
    let first = coordinator
        .dispatch_task(&task.id, &mut fixture.runtime("disconnect"))
        .unwrap();
    let run_id = first.run_id.unwrap();
    approve("denied");
    let before = store.events_for_run(&run_id).unwrap();
    assert!(!store.begin_resume(&task.id, &run_id).unwrap());
    assert_eq!(
        coordinator
            .resume_run(&run_id, &mut fixture.runtime("success"))
            .unwrap()
            .status,
        TaskStatus::Interrupted
    );
    assert_eq!(store.events_for_run(&run_id).unwrap(), before);
    assert!(fixture.requests("thread/resume").is_empty());
    approve("approved");
    task.depends_on = vec!["missing".into()];
    store.upsert_task("spec", &task).unwrap();
    assert!(!store.begin_resume(&task.id, &run_id).unwrap());
    task.depends_on.clear();
    store.upsert_task("spec", &task).unwrap();
    coordinator.retry_interrupted_task(&task.id).unwrap();
    let second = coordinator
        .dispatch_task(&task.id, &mut fixture.runtime("disconnect"))
        .unwrap();
    assert!(!store.begin_resume(&task.id, &run_id).unwrap());
    assert!(store
        .begin_resume(&task.id, second.run_id.as_ref().unwrap())
        .unwrap());
    assert!(!store
        .begin_resume(&task.id, second.run_id.as_ref().unwrap())
        .unwrap());
}

#[test]
fn missing_recovery_identity_is_durably_blocked_until_an_explicit_retry() {
    for missing in ["definition", "thread", "turn"] {
        let fixture = Fixture::new();
        let db = fixture.cwd.join("router.sqlite3");
        let store = RouterStore::open(&db).unwrap();
        let mut task = fixture.task("T-004");
        task.status = TaskStatus::Ready;
        store.upsert_task("spec", &task).unwrap();
        let route =
            resolve_task_route(&task, None, &RouterConfig::default(), &RiskRules::default());
        assert!(store
            .begin_task_run(
                &task,
                &TaskDefinition {
                    model: route.model.clone(),
                    effort: route.effort.clone(),
                    ..task.clone()
                },
                "spec",
                &RouteReceipt {
                    route_id: "route".into(),
                    run_id: "run".into(),
                    profile: route.tier.as_str().into(),
                    model: route.model.clone(),
                    effort: route.effort.clone(),
                    status: RouteStatus::Accepted,
                },
                &route,
            )
            .unwrap());
        if missing != "thread" {
            store
                .record_event(&EventRecord {
                    run_id: "run".into(),
                    event_type: "thread.started".into(),
                    payload: json!({"threadId": "fake-thread"}),
                })
                .unwrap();
        }
        if missing == "definition" {
            rusqlite::Connection::open(&db)
                .unwrap()
                .execute("DELETE FROM events WHERE event_type = 'task.started'", [])
                .unwrap();
        }
        let coordinator = TaskCoordinator::new(&store).with_router(RouterConfig::default());
        coordinator
            .recover_running_task(&task.id, "worker crashed")
            .unwrap();
        let blocked = coordinator
            .resume_run("run", &mut fixture.runtime("success"))
            .unwrap();
        assert_eq!(blocked.status, TaskStatus::Blocked, "{missing}");
        assert!(blocked.text.unwrap().contains("explicit recovery required"));
        assert_eq!(store.get_run("run").unwrap().unwrap().status, "blocked");
        assert!(!fixture.log_path().exists());
        let before = store.events_for_run("run").unwrap();
        assert_eq!(
            before
                .iter()
                .filter(|event| event.event_type == "thread.resume.failed")
                .count(),
            1
        );
        coordinator
            .resume_run("run", &mut fixture.runtime("success"))
            .unwrap();
        coordinator
            .dispatch_task(&task.id, &mut fixture.runtime("success"))
            .unwrap();
        assert_eq!(store.events_for_run("run").unwrap(), before);
        assert_eq!(store.count_runs().unwrap(), 1);

        // A retry is the way out: a new run on a new thread.
        assert!(coordinator.is_retryable(&task.id).unwrap(), "{missing}");
        let retried = coordinator.retry_interrupted_task(&task.id).unwrap();
        assert_eq!(retried.status, TaskStatus::Ready, "{missing}");
        let rerun = coordinator
            .dispatch_task(&task.id, &mut fixture.runtime("success"))
            .unwrap();
        assert_eq!(rerun.status, TaskStatus::Completed, "{missing}");
        assert_ne!(rerun.run_id.as_deref(), Some("run"));
        assert_eq!(store.events_for_run("run").unwrap(), before);
        assert_eq!(store.count_runs().unwrap(), 2);
        assert!(!coordinator.is_retryable(&task.id).unwrap());
    }
}

#[test]
fn terminal_notifications_are_idempotent_for_each_thread_and_turn_after_reopen() {
    let fixture = Fixture::new();
    let db = fixture.cwd.join("router.sqlite3");
    let store = RouterStore::open(&db).unwrap();
    store.upsert_task("spec", &fixture.task("T-004")).unwrap();
    let receipt = TaskCoordinator::new(&store)
        .with_router(RouterConfig::default())
        .dispatch_task("T-004", &mut fixture.runtime("disconnect"))
        .unwrap();
    let run_id = receipt.run_id.unwrap();
    let events = [
        EventRecord {
            run_id: run_id.clone(),
            event_type: "turn/completed".into(),
            payload: json!({"threadId": "fake-thread", "turn": {"id": "fake-turn", "status": "completed"}}),
        },
        EventRecord {
            run_id: run_id.clone(),
            event_type: "turn.completed".into(),
            payload: json!({"threadId": "fake-thread", "turnId": "fake-turn", "status": "completed"}),
        },
        EventRecord {
            run_id: run_id.clone(),
            event_type: "turn.completed".into(),
            payload: json!({"threadId": "another-thread", "turnId": "fake-turn", "status": "completed"}),
        },
    ];
    for event in &events {
        store.record_event(event).unwrap();
    }
    let before = store.events_for_run(&run_id).unwrap();
    drop(store);
    let store = RouterStore::open(&db).unwrap();
    for event in &events {
        store.record_event(event).unwrap();
    }
    assert_eq!(store.events_for_run(&run_id).unwrap(), before);
    assert!(store
        .record_event(&EventRecord {
            run_id: run_id.clone(),
            event_type: "turn/completed".into(),
            payload: json!({"turn": {"status": "completed"}})
        })
        .is_err());
    assert_eq!(store.events_for_run(&run_id).unwrap(), before);
}

#[test]
fn terminal_state_and_audit_event_roll_back_together_on_database_failure() {
    let fixture = Fixture::new();
    let db = fixture.cwd.join("router.sqlite3");
    let store = RouterStore::open(&db).unwrap();
    store.upsert_task("spec", &fixture.task("T-004")).unwrap();
    let receipt = TaskCoordinator::new(&store)
        .with_router(RouterConfig::default())
        .dispatch_task("T-004", &mut fixture.runtime("disconnect"))
        .unwrap();
    let run_id = receipt.run_id.unwrap();
    assert!(store.begin_resume("T-004", &run_id).unwrap());
    let fault = rusqlite::Connection::open(&db).unwrap();
    fault.execute_batch("CREATE TRIGGER fail_audit BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END;").unwrap();
    assert!(store
        .finish_task_run(
            "T-004",
            &run_id,
            TaskStatus::Completed,
            Some(&EventRecord {
                run_id: run_id.clone(),
                event_type: "task.verified".into(),
                payload: json!({"result": {"exitCode": 0}}),
            })
        )
        .is_err());
    assert_eq!(
        store.get_task("T-004").unwrap().unwrap().definition.status,
        TaskStatus::Running
    );
    assert_eq!(store.get_run(&run_id).unwrap().unwrap().status, "running");
    fault.execute_batch("DROP TRIGGER fail_audit").unwrap();
    let coordinator = TaskCoordinator::new(&store).with_router(RouterConfig::default());
    coordinator
        .recover_running_task("T-004", "audit storage restored")
        .unwrap();
    assert_eq!(
        coordinator
            .resume_run(&run_id, &mut fixture.runtime("success"))
            .unwrap()
            .status,
        TaskStatus::Completed
    );
}

#[test]
fn concurrent_resume_claims_allow_only_one_coordinator_to_start() {
    let fixture = Fixture::new();
    let db = fixture.cwd.join("router.sqlite3");
    let store = RouterStore::open(&db).unwrap();
    store.upsert_task("spec", &fixture.task("T-004")).unwrap();
    let interrupted = TaskCoordinator::new(&store)
        .with_router(RouterConfig::default())
        .dispatch_task("T-004", &mut fixture.runtime("disconnect"))
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let db = db.clone();
            let barrier = barrier.clone();
            let run_id = interrupted.run_id.clone().unwrap();
            std::thread::spawn(move || {
                let store = RouterStore::open(db).unwrap();
                barrier.wait();
                store.begin_resume("T-004", &run_id).unwrap()
            })
        })
        .collect();
    let claims: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(claims.iter().filter(|claimed| **claimed).count(), 1);
    assert_eq!(
        store
            .events_for_run(interrupted.run_id.as_ref().unwrap())
            .unwrap()
            .iter()
            .filter(|event| event.event_type == "task.resuming")
            .count(),
        1
    );
    assert_eq!(
        store
            .get_run(interrupted.run_id.as_ref().unwrap())
            .unwrap()
            .unwrap()
            .status,
        "running"
    );
}

#[test]
fn approval_decision_and_state_roll_back_together_on_database_failure() {
    let fixture = Fixture::new();
    let db = fixture.cwd.join("router.sqlite3");
    let store = RouterStore::open(&db).unwrap();
    let mut task = fixture.task("T-004");
    task.approval = "manual".into();
    store.upsert_task("spec", &task).unwrap();
    TaskCoordinator::new(&store)
        .with_router(RouterConfig::default())
        .dispatch_task(&task.id, &mut fixture.runtime("success"))
        .unwrap();
    let fault = rusqlite::Connection::open(&db).unwrap();
    fault.execute_batch("CREATE TRIGGER fail_state BEFORE UPDATE ON tasks BEGIN SELECT RAISE(ABORT, 'state unavailable'); END;").unwrap();
    let decision = TaskApproval {
        task_id: task.id.clone(),
        decision: "denied".into(),
        reason: None,
    };
    assert!(store.record_task_approval(&decision).is_err());
    assert!(store.latest_task_approval(&task.id).unwrap().is_none());
    assert_eq!(
        store.get_task(&task.id).unwrap().unwrap().definition.status,
        TaskStatus::AwaitingApproval
    );
    fault.execute_batch("DROP TRIGGER fail_state").unwrap();
    store.record_task_approval(&decision).unwrap();
    assert_eq!(
        store.get_task(&task.id).unwrap().unwrap().definition.status,
        TaskStatus::Blocked
    );
    assert!(!fixture.log_path().exists());
}

#[test]
fn invalid_runtime_identity_or_status_fails_without_verification() {
    struct InvalidRuntime(&'static str);
    impl router_core::TaskRuntime for InvalidRuntime {
        fn start_thread(&mut self, _: &TaskDefinition) -> Result<String, String> {
            Ok(if self.0 == "empty-thread" {
                ""
            } else {
                "thread"
            }
            .into())
        }
        fn run_turn(
            &mut self,
            _: &TaskDefinition,
            _: &str,
        ) -> Result<router_core::RuntimeTurn, String> {
            assert_ne!(self.0, "empty-thread");
            Ok(router_core::RuntimeTurn {
                thread_id: if self.0 == "wrong-thread" {
                    "wrong"
                } else {
                    "thread"
                }
                .into(),
                turn_id: if self.0 == "empty-turn" { "" } else { "turn" }.into(),
                status: if self.0 == "non-terminal" {
                    "inProgress"
                } else {
                    "completed"
                }
                .into(),
                text: "untrusted result".into(),
            })
        }
        fn verify(&mut self, _: &TaskDefinition) -> Result<Value, String> {
            panic!("invalid runtime output cannot reach verification")
        }
    }
    for scenario in ["empty-thread", "wrong-thread", "empty-turn", "non-terminal"] {
        let fixture = Fixture::new();
        let store = RouterStore::in_memory().unwrap();
        store.upsert_task("spec", &fixture.task("T-004")).unwrap();
        let result = TaskCoordinator::new(&store)
            .with_router(RouterConfig::default())
            .dispatch_task("T-004", &mut InvalidRuntime(scenario))
            .unwrap();
        assert_eq!(result.status, TaskStatus::Failed, "{scenario}");
        assert_eq!(
            store
                .get_run(result.run_id.as_ref().unwrap())
                .unwrap()
                .unwrap()
                .status,
            "failed"
        );
        assert!(!store
            .events_for_run(result.run_id.as_ref().unwrap())
            .unwrap()
            .iter()
            .any(|event| event.event_type == "turn.completed"));
    }
}

/// A UI stand-in: every session event of the task lands in the receiver.
fn ui_events() -> (EventSink, mpsc::Receiver<SessionEvent>) {
    let (sender, receiver) = mpsc::channel();
    let sender = Mutex::new(sender);
    let sink: EventSink = Arc::new(move |event| {
        let _ = sender.lock().unwrap().send(event);
    });
    (sink, receiver)
}

/// Dispatch T-004 on another thread, as the app does, with a file store.
fn dispatch_in_background(
    fixture: &Fixture,
    runtime: LocalTaskRuntime,
) -> std::thread::JoinHandle<router_core::TaskExecutionReceipt> {
    let db = fixture.cwd.join("router.sqlite3");
    RouterStore::open(&db)
        .unwrap()
        .upsert_task("spec", &fixture.task("T-004"))
        .unwrap();
    std::thread::spawn(move || {
        let store = RouterStore::open(&db).unwrap();
        let mut runtime = runtime;
        TaskCoordinator::new(&store)
            .with_router(RouterConfig::default())
            .dispatch_task("T-004", &mut runtime)
            .unwrap()
    })
}

#[test]
fn cli_runs_decline_approvals_and_keep_them_as_evidence() {
    let fixture = Fixture::new();
    let store = RouterStore::in_memory().unwrap();
    store.upsert_task("spec", &fixture.task("T-004")).unwrap();
    let receipt = TaskCoordinator::new(&store)
        .with_router(RouterConfig::default())
        .dispatch_task("T-004", &mut fixture.runtime("approval"))
        .unwrap();
    assert_eq!(receipt.status, TaskStatus::Completed);
    assert_eq!(receipt.text.as_deref(), Some("decision=decline"));
    let events = store
        .events_for_run(receipt.run_id.as_ref().unwrap())
        .unwrap();
    let approval = events
        .iter()
        .find(|event| event.event_type == "item/commandExecution/requestApproval")
        .expect("the approval request is evidence");
    assert_eq!(approval.payload["command"], "cargo publish");
    // Task processes are marked as internal workers of this task.
    let started = &fixture.requests("thread/start")[0];
    assert_eq!(started["internal"], "1");
    assert_eq!(started["activeTask"], "T-004");
}

#[test]
fn app_runs_wait_for_the_ui_to_answer_approvals() {
    let fixture = Fixture::new();
    let runs = Arc::new(TaskRuns::default());
    let (ui, events) = ui_events();
    let worker = dispatch_in_background(
        &fixture,
        LocalTaskRuntime::interactive(fixture.config("approval"), ui, runs.clone()),
    );
    let request_id = loop {
        match events
            .recv_timeout(Duration::from_secs(10))
            .expect("approval event")
        {
            SessionEvent::ApprovalRequested {
                request_id,
                command,
                ..
            } => {
                assert_eq!(command.as_deref(), Some("cargo publish"));
                break request_id;
            }
            _ => continue,
        }
    };
    assert!(runs.is_running("T-004"));
    runs.respond("T-004", request_id, ApprovalDecision::Accept)
        .unwrap();
    let receipt = worker.join().unwrap();
    assert_eq!(receipt.status, TaskStatus::Completed);
    assert_eq!(receipt.text.as_deref(), Some("decision=accept"));
    assert!(
        !runs.is_running("T-004"),
        "finished runs leave the registry"
    );
}

#[test]
fn stopping_a_running_task_interrupts_its_turn() {
    let fixture = Fixture::new();
    let runs = Arc::new(TaskRuns::default());
    let (ui, _events) = ui_events();
    let worker = dispatch_in_background(
        &fixture,
        LocalTaskRuntime::interactive(fixture.config("hang"), ui, runs.clone()),
    );
    // The turn id is known once turn/start answers; retry until then.
    let deadline = Instant::now() + Duration::from_secs(10);
    while let Err(error) = runs.interrupt("T-004") {
        assert!(Instant::now() < deadline, "could not interrupt: {error}");
        std::thread::sleep(Duration::from_millis(20));
    }
    let receipt = worker.join().unwrap();
    assert_eq!(receipt.status, TaskStatus::Interrupted);
    assert!(runs.interrupt("T-004").is_err(), "nothing left to stop");
}
