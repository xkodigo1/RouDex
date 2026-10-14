use router_core::{
    AppServerClient, AppServerConfig, TaskDefinition, TaskSandbox, TaskStatus, Tier, TurnSettings,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[test]
fn verification_passes_a_deterministic_isolated_cargo_target_per_task() {
    let cwd = std::env::temp_dir().join(format!("roudex-verification-{}", Uuid::new_v4()));
    fs::create_dir_all(&cwd).expect("create test cwd");
    let log_path = cwd.join("requests.jsonl");
    let config = AppServerConfig {
        program: PathBuf::from(env!("CARGO_BIN_EXE_fake-app-server")),
        arguments: vec!["success".into(), log_path.to_string_lossy().into_owned()],
        working_directory: cwd.clone(),
        service_name: "protocol-test".into(),
        active_task_id: None,
    };
    let mut task = TaskDefinition {
        id: "T-004".into(),
        title: "Verification isolation".into(),
        prompt: "Verify locally".into(),
        depends_on: vec![],
        status: TaskStatus::Running,
        tier: Tier::Fuerte,
        profile: None,
        model: "gpt-6.1-sol".into(),
        effort: "high".into(),
        milestone: None,
        approval: "not_required".into(),
        scope: vec![".".into()],
        cwd: cwd.to_string_lossy().into_owned(),
        sandbox: TaskSandbox {
            kind: "workspaceWrite".into(),
            writable_roots: vec![".".into()],
        },
        acceptance: vec!["Verification uses an isolated cargo target".into()],
        verify: "cargo test --workspace".into(),
    };
    {
        let mut client = AppServerClient::start(&config).expect("start fake app server");
        for id in ["T-004", "T-004", "../T-004"] {
            task.id = id.into();
            assert_eq!(
                client.verify_task(&task).expect("verify task")["exitCode"],
                0
            );
        }
    }
    let requests: Vec<Value> = fs::read_to_string(&log_path)
        .expect("read protocol log")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("parse request"))
        .filter(|entry| entry["message"]["method"] == "command/exec")
        .map(|entry| entry["message"]["params"].clone())
        .collect();
    assert_eq!(requests.len(), 3);
    let absolute_cwd = fs::canonicalize(&cwd).expect("absolute test cwd");
    for (request, (id, directory)) in requests.iter().zip([
        ("T-004", "task-542d303034"),
        ("T-004", "task-542d303034"),
        ("../T-004", "task-2e2e2f542d303034"),
    ]) {
        let target = absolute_cwd
            .join("target")
            .join("roudex-verification")
            .join(directory);
        assert_eq!(
            request["env"]["CARGO_TARGET_DIR"],
            target.to_string_lossy().as_ref()
        );
        assert_eq!(request["env"]["ROUDEX_ROUTER_INTERNAL"], "1");
        assert_eq!(request["env"]["ROUDEX_ACTIVE_TASK_ID"], id);
        assert_eq!(request["cwd"], absolute_cwd.to_string_lossy().as_ref());
        assert_eq!(request["sandboxPolicy"]["networkAccess"], false);
    }
    fs::remove_dir_all(&cwd).expect("remove test cwd");
}

#[test]
fn app_server_stdio_receives_assigned_model_and_effort() {
    let cwd = std::env::temp_dir().join(format!("roudex-app-server-{}", Uuid::new_v4()));
    fs::create_dir_all(&cwd).expect("create test cwd");
    let config = AppServerConfig {
        program: PathBuf::from(env!("CARGO_BIN_EXE_fake-app-server")),
        arguments: Vec::new(),
        working_directory: cwd.clone(),
        service_name: "protocol-test".to_owned(),
        active_task_id: Some("T-004".to_owned()),
    };

    {
        let mut client = AppServerClient::start(&config).expect("start fake app server");
        let thread_id = client
            .start_thread("gpt-6.1-sol", &cwd, "never", "workspace-write")
            .expect("start thread");
        let settings = TurnSettings {
            model: "gpt-6.1-sol",
            effort: "high",
            cwd: &cwd,
            approval_policy: "never",
            sandbox: "workspace-write",
        };
        let turn = client
            .run_turn(&thread_id, "run the protocol test", &settings)
            .expect("run turn");
        assert_eq!(thread_id, "fake-thread");
        assert_eq!(turn.turn_id, "fake-turn");
        assert_eq!(turn.status, "completed");
        assert_eq!(turn.text, "fake completion");
    }
    let _ = fs::remove_dir_all(&cwd);
}
