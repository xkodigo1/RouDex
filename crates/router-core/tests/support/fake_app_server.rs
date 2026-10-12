use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout());
    let scenario = std::env::args().nth(1).unwrap_or_else(|| "success".into());
    let log_path = std::env::args().nth(2);
    for line in stdin.lock().lines() {
        let line = line.expect("fake app server stdin");
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = serde_json::from_str(&line).expect("fake app server request");
        if let Some(path) = &log_path {
            let mut log = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("request log");
            serde_json::to_writer(
                &mut log,
                &json!({ "message": message,
                    "internal": std::env::var("ROUDEX_ROUTER_INTERNAL").ok(),
                    "activeTask": std::env::var("ROUDEX_ACTIVE_TASK_ID").ok()
                }),
            )
            .expect("log request");
            writeln!(log).expect("log newline");
        }
        let method = message["method"].as_str().unwrap_or_default();
        let id = message["id"].as_u64();
        match method {
            "initialize" => respond(
                &mut stdout,
                json!({ "id": id, "result": { "serverInfo": { "name": "fake" } } }),
            ),
            "initialized" => {}
            "thread/start" => {
                assert_eq!(message["params"]["model"], "gpt-6.1-sol");
                assert_eq!(message["params"]["sandbox"], "workspace-write");
                if scenario == "model-rejected" {
                    respond(
                        &mut stdout,
                        json!({ "id": id, "error": { "message": "assigned model unavailable" } }),
                    );
                    continue;
                }
                respond(
                    &mut stdout,
                    json!({ "id": id, "result": { "thread": { "id": "fake-thread" } } }),
                );
            }
            "thread/resume" => {
                assert_eq!(message["params"]["model"], "gpt-6.1-sol");
                assert_eq!(
                    message["params"]["config"]["model_reasoning_effort"],
                    "high"
                );
                if scenario == "resume-missing" {
                    respond(
                        &mut stdout,
                        json!({ "id": id, "error": { "message": "thread not found" } }),
                    );
                    continue;
                }
                let status = if scenario == "resume-completed" {
                    "completed"
                } else {
                    "interrupted"
                };
                respond(
                    &mut stdout,
                    json!({ "id": id, "result": { "thread": {
                    "id": "fake-thread", "turns": [{
                        "id": if scenario == "resume-wrong-turn" { "wrong-turn" } else { "fake-turn" },
                        "status": if scenario == "resume-wrong-turn" { "completed" } else { status },
                        "items": [{ "type": "agentMessage", "text": "recovered completion" }] }]
                } } }),
                );
            }
            "turn/start" => {
                assert_eq!(message["params"]["model"], "gpt-6.1-sol");
                assert_eq!(message["params"]["effort"], "high");
                // Interleaved notifications may arrive before the request response.
                respond(
                    &mut stdout,
                    json!({ "method": "item/agentMessage/delta", "params": {
                        "threadId": "unrelated-thread", "turnId": "fake-turn", "delta": "ignore me"
                    }}),
                );
                respond(
                    &mut stdout,
                    json!({ "id": id, "result": { "turn": { "id": "fake-turn" } } }),
                );
                if scenario == "disconnect" {
                    break;
                }
                respond(
                    &mut stdout,
                    json!({ "method": "turn/completed", "params": {
                        "threadId": "fake-thread", "turn": { "id": "other-turn", "status": "completed" }
                    }}),
                );
                respond(
                    &mut stdout,
                    json!({ "method": "turn/completed", "params": {
                        "turn": { "status": "completed" }
                    }}),
                );
                respond(
                    &mut stdout,
                    json!({
                        "method": "item/agentMessage/delta",
                        "params": { "threadId": "fake-thread", "turnId": "fake-turn", "delta": "fake completion" }
                    }),
                );
                respond(
                    &mut stdout,
                    json!({
                        "method": "turn/completed",
                        "params": {
                            "threadId": "fake-thread",
                            "turn": { "id": "fake-turn", "status": match scenario.as_str() {
                                "turn-failed" => "failed", "turn-interrupted" => "interrupted", _ => "completed"
                            } }
                        }
                    }),
                );
            }
            "command/exec" => respond(
                &mut stdout,
                json!({ "id": id, "result": {
                    "exitCode": if scenario == "verify-failed" { 1 } else { 0 },
                    "stdout": "verification output", "stderr": ""
                }}),
            ),
            _ => panic!("unexpected app server method: {method}"),
        }
    }
}

fn respond(stdout: &mut impl Write, message: Value) {
    serde_json::to_writer(&mut *stdout, &message).expect("fake app server response");
    stdout.write_all(b"\n").expect("fake app server newline");
    stdout.flush().expect("fake app server flush");
}
