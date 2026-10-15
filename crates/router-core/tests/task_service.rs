//! The app's task operations against the fake app server.

use router_core::{
    AppServerConfig, ApprovalDecision, RouterConfig, SessionEvent, TaskEvent, TaskService,
    TaskStatus, Tier,
};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

struct Project {
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("roudex-tasks-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("specs")).unwrap();
        Self { root }
    }

    fn spec(&self, slug: &str, manifest: Value) -> PathBuf {
        let folder = self.root.join("specs").join(slug);
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("tasks.json");
        fs::write(&path, manifest.to_string()).unwrap();
        path
    }

    /// A service whose runs talk to the fake app server in `scenario`.
    fn service(&self, scenario: &str) -> (TaskService, mpsc::Receiver<TaskEvent>) {
        let (sender, events) = mpsc::channel();
        let sender = Mutex::new(sender);
        let scenario = scenario.to_owned();
        let log = self.root.join("requests.jsonl");
        let service = TaskService::new(
            self.root.join("router.sqlite3"),
            Arc::new(move |cwd: &Path| AppServerConfig {
                program: PathBuf::from(env!("CARGO_BIN_EXE_fake-app-server")),
                arguments: vec![scenario.clone(), log.to_string_lossy().into_owned()],
                working_directory: cwd.to_path_buf(),
                service_name: "task-service-test".into(),
                active_task_id: None,
            }),
            Arc::new(move |event| {
                let _ = sender.lock().unwrap().send(event);
            }),
        )
        .with_router(RouterConfig::default());
        (service, events)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn task(id: &str, extra: Value) -> Value {
    let mut task = json!({
        "id": id, "title": id, "prompt": format!("Implementa {id}"), "depends_on": [],
        "status": "planned", "tier": "fuerte", "approval": "not_required", "scope": ["."],
        "cwd": ".", "sandbox": { "type": "workspaceWrite", "writable_roots": ["."] },
        "acceptance": ["funciona"], "verify": "echo ok"
    });
    for (key, value) in extra.as_object().unwrap() {
        task[key] = value.clone();
    }
    task
}

fn manifest(spec_id: &str, tasks: Vec<Value>) -> Value {
    json!({ "spec_id": spec_id, "title": format!("Spec {spec_id}"), "version": 1,
        "milestones": [{ "id": "H1", "title": "Hito", "task_ids": tasks.iter()
            .map(|task| task["id"].clone()).collect::<Vec<_>>() }],
        "tasks": tasks })
}

fn wait_finished(events: &mpsc::Receiver<TaskEvent>) -> (Vec<SessionEvent>, TaskEvent) {
    let mut session = Vec::new();
    loop {
        match events
            .recv_timeout(Duration::from_secs(15))
            .expect("task event")
        {
            TaskEvent::Session { event, .. } => session.push(event),
            done => return (session, done),
        }
    }
}

#[test]
fn project_specs_are_listed_and_loaded_with_their_warnings() {
    let project = Project::new();
    project.spec(
        "001-ok",
        manifest("001-ok", vec![task("T-1", json!({ "milestone": "H1" }))]),
    );
    project.spec(
        "002-legacy",
        manifest(
            "002-legacy",
            vec![task("T-2", json!({ "tier": null, "profile": "standard" }))],
        ),
    );
    project.spec("003-broken", json!({ "spec_id": "x", "tasks": [] }));
    let (service, _events) = project.service("success");

    let files = service.spec_files(&project.root).unwrap();
    assert_eq!(files.len(), 3);
    assert_eq!(files[0].spec_id.as_deref(), Some("001-ok"));
    assert!(
        files[2].error.is_some(),
        "a broken manifest is reported, not skipped"
    );
    assert!(!files[0].loaded);

    let legacy = service.load_spec(&files[1].path).unwrap();
    assert_eq!(legacy.tasks, 1);
    assert!(legacy.warnings[0].contains("tier estandar"));
    assert!(service.spec_files(&project.root).unwrap()[1].loaded);

    let board = service.board("002-legacy").unwrap();
    let card = &board.tasks[0];
    assert_eq!(card.task.tier, Tier::Estandar);
    // Relative cwd points at the project, not at the process directory.
    assert_eq!(
        Path::new(&card.task.cwd),
        router_core::project_dir(&project.root).unwrap()
    );
    assert_eq!(board.spec.milestones[0].id, "H1");
}

#[test]
fn the_board_shows_the_route_a_dispatch_would_take() {
    let project = Project::new();
    let path = project.spec(
        "001-routes",
        manifest(
            "001-routes",
            vec![
                task("T-1", json!({ "tier": "economico" })),
                task(
                    "T-2",
                    json!({ "tier": "economico", "prompt": "Borra datos de producción" }),
                ),
                task(
                    "T-3",
                    json!({ "approval": "manual", "depends_on": ["T-1"] }),
                ),
            ],
        ),
    );
    let (service, _events) = project.service("success");
    service.load_spec(&path).unwrap();

    let route = |board: &router_core::TaskBoard, id: &str| {
        let card = board.tasks.iter().find(|card| card.task.id == id).unwrap();
        let route = card.route.clone().unwrap();
        (route.tier, serde_json::to_value(route.source).unwrap())
    };
    let board = service.board("001-routes").unwrap();
    assert_eq!(route(&board, "T-1"), (Tier::Economico, json!("spec")));
    assert_eq!(route(&board, "T-2"), (Tier::Fuerte, json!("risk")));
    let t3 = board
        .tasks
        .iter()
        .find(|card| card.task.id == "T-3")
        .unwrap();
    assert!(!t3.dependencies_completed);
    assert!(t3.approval.is_none());

    service.set_tier("T-2", Some(Tier::Estandar)).unwrap();
    service.decide("T-3", true).unwrap();
    let board = service.board("001-routes").unwrap();
    assert_eq!(route(&board, "T-2"), (Tier::Estandar, json!("manual")));
    let t3 = board
        .tasks
        .iter()
        .find(|card| card.task.id == "T-3")
        .unwrap();
    assert_eq!(t3.approval.as_ref().unwrap().decision, "approved");
    assert!(service.set_tier("T-9", None).is_err());
}

#[test]
fn runs_stream_to_the_app_and_only_one_runs_at_a_time() {
    let project = Project::new();
    let path = project.spec(
        "001-run",
        manifest(
            "001-run",
            vec![task("T-1", json!({})), task("T-2", json!({}))],
        ),
    );
    let (service, events) = project.service("approval");
    service.load_spec(&path).unwrap();

    service.dispatch("T-1").unwrap();
    assert!(service.dispatch("T-2").is_err(), "one run at a time");
    let request_id = loop {
        match events
            .recv_timeout(Duration::from_secs(15))
            .expect("approval")
        {
            TaskEvent::Session {
                task_id,
                event: SessionEvent::ApprovalRequested { request_id, .. },
            } => {
                assert_eq!(task_id, "T-1");
                break request_id;
            }
            TaskEvent::Session { .. } => continue,
            other => panic!("unexpected {other:?}"),
        }
    };
    service
        .respond("T-1", request_id, ApprovalDecision::Accept)
        .unwrap();
    let (streamed, finished) = wait_finished(&events);
    let TaskEvent::Finished { task_id, receipt } = finished else {
        panic!("expected a finished run, got {finished:?}");
    };
    assert_eq!(task_id, "T-1");
    assert_eq!(receipt.status, TaskStatus::Completed);
    assert!(streamed
        .iter()
        .any(|event| matches!(event, SessionEvent::TurnCompleted { .. })));
    assert_eq!(service.active(), None);

    let run_id = receipt.run_id.unwrap();
    let recorded = service.run_events(&run_id).unwrap();
    assert!(recorded
        .iter()
        .any(|event| event.event_type == "task.verified"));
    let board = service.board("001-run").unwrap();
    assert!(!board.tasks[0].running);
    assert!(!board.tasks[0].retryable);
    assert_eq!(board.tasks[0].latest_run.as_ref().unwrap().run_id, run_id);
}

#[test]
fn ready_tasks_run_in_dependency_order() {
    let project = Project::new();
    let path = project.spec(
        "001-batch",
        manifest(
            "001-batch",
            vec![
                task("T-2", json!({ "depends_on": ["T-1"] })),
                task("T-1", json!({})),
            ],
        ),
    );
    let (service, events) = project.service("success");
    service.load_spec(&path).unwrap();
    service.dispatch_ready("001-batch").unwrap();
    let mut finished = Vec::new();
    while finished.len() < 2 {
        if let TaskEvent::Finished { task_id, receipt } =
            events.recv_timeout(Duration::from_secs(15)).expect("batch")
        {
            assert_eq!(receipt.status, TaskStatus::Completed);
            assert_eq!(service.active(), None, "the slot is free before results");
            finished.push(task_id);
        }
    }
    assert_eq!(finished, ["T-1", "T-2"]);
}
