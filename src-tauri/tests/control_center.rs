// R002-T002 verification record, 2026-10-13 (synthetic local stores only):
// cargo test --offline -p roudex-desktop --test control_center
// Exit 0: 5 passed, 0 failed, 0 ignored, 0 filtered out.
// cargo test --offline -p roudex-desktop
// Exit 0: control_center 5 passed and existing local_state 5 passed.
// No native UI or live model acceptance is claimed by these checks.

use roudex_desktop::local_snapshot;
use router_core::{RoutePromptInput, RouteReceipt, RouteStatus, RouterStore, TaskDefinition};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "roudex-control-center-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        Self(path.join("state.sqlite3"))
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

fn task(store: &RouterStore, spec: &str, id: &str, status: &str) {
    let definition: TaskDefinition = serde_json::from_value(json!({
        "id": id, "title": "Synthetic task", "prompt": "Synthetic read model fixture",
        "depends_on": [], "status": status, "profile": "standard", "model": "gpt-6.1-sol",
        "effort": "medium", "approval": "manual", "scope": ["src-tauri"], "cwd": ".",
        "sandbox": {"type": "workspaceWrite", "writable_roots": ["src-tauri"]},
        "acceptance": ["Read only"], "verify": "synthetic fixture"
    }))
    .unwrap();
    store.upsert_task(spec, &definition).unwrap();
}

fn run(store: &RouterStore, id: &str, task_id: &str) {
    store
        .record_route(
            &RoutePromptInput {
                prompt: "Synthetic run".into(),
                session_id: "fixture".into(),
                turn_id: task_id.into(),
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

fn serialized(fixture: &Fixture) -> Value {
    serde_json::to_value(local_snapshot(&fixture.0).unwrap()).unwrap()
}

#[test]
fn spec_and_status_totals_count_tasks_once_despite_retries_and_missing_runs() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let spec = "002-product-roadmap";
    task(&store, spec, "R002-T001", "completed");
    task(&store, spec, "R002-T002", "running");
    task(&store, spec, "R002-T003", "blocked");
    task(&store, spec, "R002-T006", "planned");
    run(&store, "attempt-1", "R002-T002");
    run(&store, "attempt-2", "R002-T002");
    run(&store, "legacy-unlinked", "desktop-turn");
    let before = std::fs::read(&fixture.0).unwrap();
    let snapshot = serialized(&fixture);
    assert_eq!(snapshot["task_total"], 4);
    assert_eq!(snapshot["status_counts"]["completed"], 1);
    assert_eq!(snapshot["status_counts"]["running"], 1);
    assert_eq!(snapshot["status_counts"]["blocked"], 1);
    assert_eq!(snapshot["status_counts"]["planned"], 1);
    assert_eq!(snapshot["status_counts"]["failed"], 0);
    let groups = snapshot["milestones"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["spec_id"], spec);
    assert_eq!(groups[0]["milestone_id"], Value::Null);
    assert_eq!(groups[0]["task_total"], 4);
    assert_eq!(groups[0]["status_counts"]["running"], 1);
    assert_eq!(groups[0]["status_counts"]["blocked"], 1);
    assert_eq!(snapshot["runs"].as_array().unwrap().len(), 3);
    assert_eq!(snapshot["runs"][0]["task_id"], Value::Null);
    assert_eq!(std::fs::read(&fixture.0).unwrap(), before);
}

#[test]
fn selection_keys_survive_refresh_status_changes_and_newer_runs() {
    let fixture = Fixture::new();
    let store = fixture.store();
    task(&store, "002-product-roadmap", "R002-T002", "planned");
    run(&store, "original", "R002-T002");
    let before = serialized(&fixture);
    task(&store, "another-spec", "sorts-before", "ready");
    run(&store, "retry", "R002-T002");
    let connection = Connection::open(&fixture.0).unwrap();
    connection
        .execute(
            "UPDATE tasks SET status='running' WHERE task_id='R002-T002'",
            [],
        )
        .unwrap();
    let after = serialized(&fixture);
    let original_task = after["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "R002-T002")
        .unwrap();
    let original_run = after["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["run_id"] == "original")
        .unwrap();
    assert_eq!(
        original_task["selection_key"],
        before["tasks"][0]["selection_key"]
    );
    assert_eq!(original_task["selection_key"], "R002-T002");
    assert_eq!(
        original_run["selection_key"],
        before["runs"][0]["selection_key"]
    );
    assert_eq!(original_run["selection_key"], "original");
    assert_eq!(after["runs"][0]["selection_key"], "retry");
    assert_eq!(after["status_counts"]["running"], 1);
    assert_eq!(after["status_counts"]["planned"], 0);
}

#[test]
fn unmapped_tasks_are_explicit_and_membership_is_spec_scoped() {
    let fixture = Fixture::new();
    let store = fixture.store();
    task(&store, "other-spec", "R002-T002", "interrupted");
    task(
        &store,
        "002-product-roadmap",
        "new-task",
        "awaiting_approval",
    );
    let snapshot = serialized(&fixture);
    assert_eq!(snapshot["task_total"], 2);
    assert_eq!(snapshot["milestones"].as_array().unwrap().len(), 2);
    for group in snapshot["milestones"].as_array().unwrap() {
        assert_eq!(group["milestone_id"], Value::Null);
        assert_eq!(group["task_total"], 1);
    }
    for task in snapshot["tasks"].as_array().unwrap() {
        assert_eq!(task["milestone_id"], Value::Null);
    }
}

#[test]
fn missing_and_initialized_empty_stores_are_distinct_without_writes() {
    let fixture = Fixture::new();
    let missing = serialized(&fixture);
    assert_eq!(missing["initialized"], false);
    assert_eq!(missing["task_total"], 0);
    assert_eq!(missing["status_counts"]["completed"], 0);
    assert_eq!(missing["milestones"], json!([]));
    assert!(!fixture.0.parent().unwrap().exists());
    drop(fixture.store());
    let before = std::fs::read(&fixture.0).unwrap();
    let empty = serialized(&fixture);
    assert_eq!(empty["initialized"], true);
    assert_eq!(empty["task_total"], 0);
    assert_eq!(empty["milestones"], json!([]));
    assert_eq!(std::fs::read(&fixture.0).unwrap(), before);
}

#[test]
fn unreadable_incompatible_and_corrupt_stores_are_distinct_and_never_migrated() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(&fixture.0).unwrap();
    assert!(local_snapshot(&fixture.0)
        .unwrap_err()
        .starts_with("store_unreadable:"));
    assert!(fixture.0.is_dir());
    std::fs::remove_dir(&fixture.0).unwrap();
    let connection = Connection::open(&fixture.0).unwrap();
    connection
        .execute_batch("CREATE TABLE unrelated (value TEXT);")
        .unwrap();
    drop(connection);
    let before = std::fs::read(&fixture.0).unwrap();
    assert!(local_snapshot(&fixture.0)
        .unwrap_err()
        .starts_with("store_incompatible:"));
    assert_eq!(std::fs::read(&fixture.0).unwrap(), before);
    std::fs::write(&fixture.0, b"corrupt synthetic database").unwrap();
    assert!(local_snapshot(&fixture.0)
        .unwrap_err()
        .starts_with("store_incompatible:"));
    assert_eq!(
        std::fs::read(&fixture.0).unwrap(),
        b"corrupt synthetic database"
    );
}
