use roudex_desktop::{local_run_events, local_snapshot};
use router_core::{
    EventRecord, RoutePromptInput, RouteReceipt, RouteStatus, RouterStore, TaskApproval,
    TaskDefinition,
};
use rusqlite::Connection;
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "roudex-desktop-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self(dir.join("state.sqlite3"))
    }
    fn store(&self) -> RouterStore {
        RouterStore::open(&self.0).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

fn task(id: &str) -> TaskDefinition {
    serde_json::from_value(json!({
        "id": id, "title": "Local UI fixture", "prompt": "Read state", "depends_on": [],
        "status": "awaiting_approval", "profile": "standard", "model": "gpt-6.1-sol", "effort": "medium",
        "approval": "manual", "scope": ["src-tauri"], "cwd": ".",
        "sandbox": {"type": "workspaceWrite", "writable_roots": ["src-tauri"]},
        "acceptance": ["Local-only"], "verify": "cargo test --workspace"
    })).unwrap()
}

fn run(store: &RouterStore, id: &str, turn_id: &str) {
    store
        .record_route(
            &RoutePromptInput {
                prompt: "fixture".into(),
                session_id: "spec".into(),
                turn_id: turn_id.into(),
                cwd: ".".into(),
                permission_mode: None,
            },
            &RouteReceipt {
                route_id: format!("route-{id}"),
                run_id: id.into(),
                profile: "standard".into(),
                model: "gpt-6.1-sol".into(),
                effort: "medium".into(),
                status: RouteStatus::Accepted,
            },
        )
        .unwrap();
}
fn event(store: &RouterStore, run_id: &str, kind: &str, payload: serde_json::Value) {
    store
        .record_event(&EventRecord {
            run_id: run_id.into(),
            event_type: kind.into(),
            payload,
        })
        .unwrap();
}

#[test]
fn missing_store_is_empty_and_observation_does_not_create_it() {
    let fixture = Fixture::new();
    let snapshot = local_snapshot(&fixture.0).unwrap();
    assert!(!snapshot.initialized);
    assert!(snapshot.tasks.is_empty() && snapshot.runs.is_empty() && snapshot.approvals.is_empty());
    assert!(!fixture.0.parent().unwrap().exists());
    assert!(local_run_events(&fixture.0, "absent").is_err());
    assert!(!fixture.0.exists());
}

#[test]
fn lists_specs_approval_history_runs_and_real_failures_without_mutation() {
    let fixture = Fixture::new();
    let store = fixture.store();
    store.upsert_task("spec-a", &task("T-A")).unwrap();
    let mut second = task("T-B");
    second.depends_on = vec!["T-A".into()];
    store.upsert_task("spec-b", &second).unwrap();
    for decision in ["denied", "approved"] {
        store
            .record_task_approval(&TaskApproval {
                task_id: "T-A".into(),
                decision: decision.into(),
                reason: Some("Explicit fixture".into()),
            })
            .unwrap();
    }
    run(&store, "run-a", "T-A");
    run(&store, "legacy", "desktop-turn");
    event(
        &store,
        "run-a",
        "verification.failed",
        json!({"reason":"Check failed"}),
    );
    event(
        &store,
        "run-a",
        "turn/completed",
        json!({"threadId":"thread", "turn":{"id":"nested-turn", "status":"failed", "error":{"message":"Runtime failed"}}}),
    );
    event(
        &store,
        "run-a",
        "turn.completed",
        json!({"threadId":"thread", "turnId":"turn", "status":"interrupted"}),
    );
    event(
        &store,
        "run-a",
        "item/updated",
        json!({"error":"Protocol failure"}),
    );
    event(
        &store,
        "run-a",
        "task.verified",
        json!({"result":{"exitCode":0}}),
    );
    let before = std::fs::read(&fixture.0).unwrap();
    let snapshot = local_snapshot(&fixture.0).unwrap();
    assert!(snapshot.initialized);
    assert_eq!(snapshot.tasks.len(), 2);
    assert_eq!(snapshot.tasks[1]["dependencies"], "T-A");
    assert_eq!(snapshot.runs[0]["task_id"], serde_json::Value::Null);
    assert_eq!(snapshot.runs[1]["task_id"], "T-A");
    assert_eq!(snapshot.approvals[0]["decision"], "approved");
    assert_eq!(snapshot.approvals[0]["latest"], true);
    assert_eq!(snapshot.approvals[1]["latest"], false);
    assert_eq!(snapshot.error_count, 4);
    assert_eq!(snapshot.errors.len(), 4);
    let events = local_run_events(&fixture.0, "run-a").unwrap();
    assert_eq!(events.total, 5);
    assert_eq!(events.events[0]["payload"]["reason"], "Check failed");
    assert_eq!(std::fs::read(&fixture.0).unwrap(), before);
}

#[test]
fn event_windows_are_bounded_ordered_and_isolated_by_run() {
    let fixture = Fixture::new();
    let store = fixture.store();
    run(&store, "first", "task-a");
    run(&store, "second", "task-b");
    for i in 0..205 {
        event(&store, "first", "runtime.error", json!({"index":i}));
    }
    event(
        &store,
        "second",
        "item/updated",
        json!({"text":"Other run"}),
    );
    let events = local_run_events(&fixture.0, "first").unwrap();
    assert_eq!(events.total, 205);
    assert_eq!(events.events.len(), 200);
    assert_eq!(events.events[0]["payload"]["index"], 5);
    assert_eq!(events.events[199]["payload"]["index"], 204);
    let snapshot = local_snapshot(&fixture.0).unwrap();
    assert_eq!(snapshot.error_count, 205);
    assert_eq!(snapshot.errors.len(), 200);
    assert_eq!(snapshot.errors[0]["payload"]["index"], 204);
    assert!(local_run_events(&fixture.0, "' OR 1=1 --").is_err());
}

#[test]
fn invalid_database_is_reported_and_never_replaced_or_migrated() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.0.parent().unwrap()).unwrap();
    std::fs::write(&fixture.0, b"invalid SQLite").unwrap();
    assert!(local_snapshot(&fixture.0).is_err());
    assert_eq!(std::fs::read(&fixture.0).unwrap(), b"invalid SQLite");
    std::fs::remove_file(&fixture.0).unwrap();
    let connection = Connection::open(&fixture.0).unwrap();
    connection
        .execute_batch("CREATE TABLE unrelated (value TEXT);")
        .unwrap();
    assert!(local_snapshot(&fixture.0).is_err());
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'tasks'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn malformed_event_payload_is_an_error_instead_of_partial_success() {
    let fixture = Fixture::new();
    let store = fixture.store();
    run(&store, "run", "task");
    let connection = Connection::open(&fixture.0).unwrap();
    connection.execute("INSERT INTO events (run_id, event_type, payload, created_at) VALUES ('run', 'runtime.error', 'broken', '0')", []).unwrap();
    assert!(local_snapshot(&fixture.0).is_err());
    assert!(local_run_events(&fixture.0, "run").is_err());
}
