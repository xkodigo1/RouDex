use router_core::{AppServerClient, AppServerConfig};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

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

    (|| {
        let mut client = AppServerClient::start(&config).expect("start fake app server");
        let thread_id = client
            .start_thread("gpt-6.1-sol", &cwd, "never", "workspace-write")
            .expect("start thread");
        let turn = client
            .run_turn(
                &thread_id,
                "run the protocol test",
                "gpt-6.1-sol",
                "high",
                &cwd,
                "never",
                "workspace-write",
            )
            .expect("run turn");
        assert_eq!(thread_id, "fake-thread");
        assert_eq!(turn.turn_id, "fake-turn");
        assert_eq!(turn.status, "completed");
        assert_eq!(turn.text, "fake completion");
    })();
    let _ = fs::remove_dir_all(&cwd);
}
