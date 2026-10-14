use router_core::{
    RouterStore, RoutingConfig, RuntimeTurn, TaskCoordinator, TaskDefinition, TaskRuntime,
};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("roudex-preview-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn preview(&self, args: &[&str], config: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_router-cli"))
            .arg("route-preview")
            .args(args)
            .current_dir(&self.0)
            .env("LOCALAPPDATA", &self.0)
            .env("CODEX_HOME", self.0.join("unused-codex"))
            .env("ROUDEX_CODEX_BIN", self.0.join("missing-runtime"))
            // Read-only previews are permitted within an active worker too.
            .env("ROUDEX_ROUTER_INTERNAL", "1")
            .env("ROUDEX_ACTIVE_TASK_ID", "synthetic-preview-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        feed(child.stdin.take(), config.as_bytes());
        child.wait_with_output().unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn rejected(output: Output, message: &str) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains(message));
}

fn candidate() -> Value {
    json!({"version":1,"revision":7,"overrides":[
        {"profile":"deep","model":"gpt-6-astra","effort":"medium"}
    ]})
}

// Synthetic coordinator execution only; no App Server process or approval.
struct DispatchProbe;
impl TaskRuntime for DispatchProbe {
    fn start_thread(&mut self, _: &TaskDefinition) -> Result<String, String> {
        Err("synthetic stop after route persistence".into())
    }
    fn run_turn(&mut self, _: &TaskDefinition, _: &str) -> Result<RuntimeTurn, String> {
        panic!("probe must never start a turn")
    }
}

#[test]
fn every_builtin_preview_equals_the_coordinator_dispatch_assignment() {
    let fixture = Fixture::new();
    for profile in ["micro", "routine", "standard", "deep", "frontier"] {
        let preview = success(fixture.preview(&[profile], ""));
        assert_eq!(preview["reason"], "built-in");
        let route = router_core::routing::resolve_profile(profile).unwrap();
        let task: TaskDefinition = serde_json::from_value(json!({
            "id":"synthetic", "title":"Synthetic route parity", "prompt":"Synthetic only",
            "depends_on":[], "status":"ready", "profile":profile,
            "model":route.model, "effort":route.effort, "approval":"not_required",
            "scope":["fixture"], "cwd":".",
            "sandbox":{"type":"readOnly","writable_roots":[]},
            "acceptance":["assignment parity"], "verify":"synthetic"
        }))
        .unwrap();
        let store = RouterStore::in_memory().unwrap();
        store.upsert_task("synthetic", &task).unwrap();
        let receipt = TaskCoordinator::new(&store)
            .dispatch_task("synthetic", &mut DispatchProbe)
            .unwrap();
        let run = store
            .get_run(receipt.run_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(preview["profile"], run.profile);
        assert_eq!(preview["model"], run.model);
        assert_eq!(preview["effort"], run.effort);
    }
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 0);
}

#[test]
fn explicit_candidate_and_partial_fallback_use_the_validated_resolver() {
    let fixture = Fixture::new();
    let document = candidate().to_string();
    let config = RoutingConfig::from_json(&document).unwrap();
    for profile in ["deep", "standard"] {
        let preview = success(fixture.preview(&[profile, "--config-stdin"], &document));
        let route = config.resolve_profile(profile).unwrap();
        assert_eq!(preview["profile"], route.profile);
        assert_eq!(preview["model"], route.model);
        assert_eq!(preview["effort"], route.effort);
        assert_eq!(preview["revision"], 7);
        assert_eq!(
            preview["reason"],
            if profile == "deep" {
                "explicit"
            } else {
                "built-in"
            }
        );
    }
    // Candidate preview does not activate settings for future invocations.
    assert_eq!(
        success(fixture.preview(&["deep"], ""))["model"],
        "gpt-6.1-sol"
    );
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 0);
}

#[test]
fn explicit_rule_is_reported_even_when_it_matches_the_builtin() {
    let fixture = Fixture::new();
    let mut config = candidate();
    config["overrides"][0]["model"] = json!("gpt-6.1-sol");
    config["overrides"][0]["effort"] = json!("high");
    assert_eq!(
        success(fixture.preview(&["deep", "--config-stdin"], &config.to_string()))["reason"],
        "explicit"
    );
}

#[test]
fn conflicting_task_overrides_are_rejected_for_builtin_and_candidate_rules() {
    let fixture = Fixture::new();
    for args in [
        vec!["deep", "--model", "gpt-6-astra"],
        vec!["deep", "--effort", "low"],
        vec!["deep", "--config-stdin", "--model", "gpt-6.1-sol"],
        vec!["deep", "--config-stdin", "--effort", "high"],
    ] {
        rejected(fixture.preview(&args, &candidate().to_string()), "conflict");
    }
    success(fixture.preview(&["deep", "--model", "gpt-6.1-sol", "--effort", "high"], ""));
    success(fixture.preview(
        &[
            "deep",
            "--config-stdin",
            "--model",
            "gpt-6-astra",
            "--effort",
            "medium",
        ],
        &candidate().to_string(),
    ));
}

#[test]
fn invalid_candidates_and_unknown_profiles_fail_without_fallback() {
    let fixture = Fixture::new();
    for body in ["", "null", "{}", "{"] {
        rejected(
            fixture.preview(&["deep", "--config-stdin"], body),
            "invalid routing configuration",
        );
    }
    for (field, value) in [
        ("profile", "unknown"),
        ("model", "unknown"),
        ("effort", "unknown"),
    ] {
        let mut config = candidate();
        config["overrides"][0][field] = json!(value);
        rejected(
            fixture.preview(&["deep", "--config-stdin"], &config.to_string()),
            "routing",
        );
    }
    for document in [
        json!({"version":2,"revision":7,"overrides":[]}),
        json!({"version":1,"revision":0,"overrides":[]}),
        json!({"version":1,"revision":7,"overrides":[
            {"profile":"deep","model":"gpt-6-astra","effort":"medium"},
            {"profile":"deep","model":"gpt-6.1-sol","effort":"high"}
        ]}),
    ] {
        rejected(
            fixture.preview(&["deep", "--config-stdin"], &document.to_string()),
            "routing",
        );
    }
    rejected(fixture.preview(&["unknown"], ""), "unknown routing profile");
    rejected(
        fixture.preview(&["unknown", "--config-stdin"], &candidate().to_string()),
        "unknown routing profile",
    );
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 0);
}

#[test]
fn malformed_arguments_are_rejected() {
    let fixture = Fixture::new();
    for args in [
        vec![],
        vec![""],
        vec!["deep", "extra"],
        vec!["deep", "--model"],
        vec!["deep", "--effort", ""],
        vec!["deep", "--model", "--config-stdin"],
        vec!["deep", "--config-stdin", "--config-stdin"],
        vec!["deep", "--model", "gpt-6.1-sol", "--model", "gpt-6.1-sol"],
        vec!["deep", "--effort", "high", "--effort", "high"],
    ] {
        rejected(fixture.preview(&args, ""), "Usage:");
    }
}

#[test]
fn preview_preserves_existing_state_and_succeeds_without_runtime_or_writable_store() {
    let fixture = Fixture::new();
    let path = fixture.0.join("RouDex/router.sqlite3");
    let store = RouterStore::open(&path).unwrap();
    assert_eq!(store.count_runs().unwrap(), 0);
    drop(store);
    let original = fs::read(&path).unwrap();
    rejected(
        fixture.preview(&["deep", "--config-stdin"], "{}"),
        "invalid routing configuration",
    );
    rejected(
        fixture.preview(&["deep", "--effort", "low"], ""),
        "conflict",
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    for _ in 0..2 {
        let preview = success(fixture.preview(&["standard"], ""));
        assert!(preview.get("runId").is_none());
        assert!(preview.get("threadId").is_none());
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    let store = RouterStore::open(&path).unwrap();
    assert_eq!(store.count_runs().unwrap(), 0);
    drop(store);
    fs::remove_dir_all(fixture.0.join("RouDex")).unwrap();
    fs::write(fixture.0.join("RouDex"), "synthetic inaccessible store").unwrap();
    success(fixture.preview(&["standard"], ""));
    assert_eq!(
        fs::read_to_string(fixture.0.join("RouDex")).unwrap(),
        "synthetic inaccessible store"
    );
    assert!(!fixture.0.join("unused-codex").exists());
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
