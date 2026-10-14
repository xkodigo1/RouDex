use router_core::{
    project_dir, AppServerConfig, ApprovalDecision, ChatService, DecisionSource, RouteDecision,
    RouterStore, SessionEvent, Tier,
};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

struct Harness {
    root: PathBuf,
    project: PathBuf,
    events: Arc<Mutex<Vec<SessionEvent>>>,
    service: ChatService,
}

impl Harness {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("roudex-chat-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("project")).unwrap();
        let project = project_dir(&root.join("project")).unwrap();
        let store = RouterStore::open(root.join("router.sqlite3")).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink_events = events.clone();
        let service = ChatService::new(
            store,
            Box::new(|cwd| AppServerConfig {
                program: PathBuf::from(env!("CARGO_BIN_EXE_fake-app-server")),
                arguments: vec!["success".into()],
                working_directory: cwd.to_path_buf(),
                service_name: "chat-test".into(),
                active_task_id: None,
            }),
            Arc::new(move |event| sink_events.lock().unwrap().push(event)),
        );
        Self {
            root,
            project,
            events,
            service,
        }
    }

    fn wait_for(&self, what: &str, matches: impl Fn(&SessionEvent) -> bool) -> SessionEvent {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(event) = self.events.lock().unwrap().iter().find(|e| matches(e)) {
                return event.clone();
            }
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn completed(&self, turn: &str) -> SessionEvent {
        self.wait_for(
            "turn completion",
            |event| matches!(event, SessionEvent::TurnCompleted { turn_id, .. } if turn_id == turn),
        )
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn decision() -> RouteDecision {
    RouteDecision {
        tier: Tier::Estandar,
        model: "gpt-6.1-sol".into(),
        effort: "medium".into(),
        source: DecisionSource::Model,
        reason: "test".into(),
        risk: vec![],
    }
}

#[test]
fn turns_stream_persist_text_tokens_and_rate_limits() {
    let h = Harness::new();
    let first = h
        .service
        .send(&h.project, None, "hola   mundo", &decision())
        .unwrap();
    h.completed(&first.turn_id);
    let second = h
        .service
        .send(&h.project, Some(&first.thread_id), "otra vez", &decision())
        .unwrap();
    assert_eq!(second.thread_id, first.thread_id);
    h.completed(&second.turn_id);

    let turns = h
        .service
        .recorder()
        .with_store(|store| store.chat_turns(&first.thread_id))
        .unwrap();
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].response, "echo: hola   mundo");
    assert_eq!(turns[0].status, "completed");
    assert_eq!(turns[0].tier, "estandar");
    assert_eq!(turns[0].source, "model");
    // Cumulative thread totals become per-turn deltas.
    assert_eq!(turns[0].tokens.total, 100);
    assert_eq!(turns[1].tokens.total, 100);
    assert_eq!(turns[1].response, "echo: otra vez");

    let threads = h
        .service
        .recorder()
        .with_store(|store| store.chat_threads(Some(&h.project)))
        .unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].title, "hola mundo");

    let usage = h
        .service
        .recorder()
        .with_store(|store| store.usage_by_model())
        .unwrap();
    assert_eq!(usage.len(), 1);
    assert_eq!((usage[0].turns, usage[0].tokens.total), (2, 200));

    let limits = h.service.recorder().last_rate_limits().unwrap();
    assert_eq!(limits["primary"]["usedPercent"], 43);
    let read = h.service.rate_limits(&h.project).unwrap();
    assert_eq!(read["rateLimits"]["primary"]["usedPercent"], 42);
}

#[test]
fn approvals_wait_for_the_user_decision() {
    let h = Harness::new();
    let sent = h
        .service
        .send(&h.project, None, "needs approval", &decision())
        .unwrap();
    let request = h.wait_for("approval", |e| {
        matches!(e, SessionEvent::ApprovalRequested { .. })
    });
    let SessionEvent::ApprovalRequested {
        request_id,
        command,
        turn_id,
        ..
    } = request
    else {
        unreachable!()
    };
    assert_eq!(command.as_deref(), Some("cargo test"));
    assert_eq!(turn_id, sent.turn_id);
    h.service
        .respond_approval(&h.project, request_id, ApprovalDecision::Decline)
        .unwrap();
    h.completed(&sent.turn_id);
    let turns = h
        .service
        .recorder()
        .with_store(|store| store.chat_turns(&sent.thread_id))
        .unwrap();
    assert_eq!(turns[0].response, "decision=decline");
}

#[test]
fn unsupported_server_requests_are_refused_without_hanging() {
    let h = Harness::new();
    let sent = h
        .service
        .send(&h.project, None, "unsupported tool", &decision())
        .unwrap();
    h.wait_for("refusal notice", |e| {
        matches!(e, SessionEvent::RuntimeError { .. })
    });
    h.completed(&sent.turn_id);
    let turns = h
        .service
        .recorder()
        .with_store(|store| store.chat_turns(&sent.thread_id))
        .unwrap();
    assert_eq!(turns[0].response, "refused=-32601");
}

#[test]
fn interrupt_stops_a_running_turn() {
    let h = Harness::new();
    let sent = h
        .service
        .send(&h.project, None, "hang please", &decision())
        .unwrap();
    h.wait_for("turn start", |e| {
        matches!(e, SessionEvent::TurnStarted { .. })
    });
    h.service
        .interrupt(&h.project, &sent.thread_id, &sent.turn_id)
        .unwrap();
    let SessionEvent::TurnCompleted { status, .. } = h.completed(&sent.turn_id) else {
        unreachable!()
    };
    assert_eq!(status, "interrupted");
    let turns = h
        .service
        .recorder()
        .with_store(|store| store.chat_turns(&sent.thread_id))
        .unwrap();
    assert_eq!(turns[0].status, "interrupted");
}

#[test]
fn approval_without_a_session_is_an_error() {
    let h = Harness::new();
    assert!(h
        .service
        .respond_approval(&h.project, serde_json::json!(1), ApprovalDecision::Accept)
        .is_err());
}
