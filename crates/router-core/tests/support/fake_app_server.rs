use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout());
    let scenario = std::env::args().nth(1).unwrap_or_else(|| "success".into());
    let log_path = std::env::args().nth(2);
    let mut chat = Chat::default();
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
        if chat.handle(&mut stdout, &message) {
            continue;
        }
        match method {
            "initialize" => respond(
                &mut stdout,
                json!({ "id": id, "result": { "serverInfo": { "name": "fake" } } }),
            ),
            "initialized" => {}
            "model/list" => {
                let entry = |id: &str, efforts: &[&str]| {
                    json!({ "id": id, "model": id, "displayName": id, "hidden": false,
                        "defaultReasoningEffort": "medium", "description": "",
                        "isDefault": false,
                        "supportedReasoningEfforts": efforts.iter()
                            .map(|e| json!({ "reasoningEffort": e, "description": "" }))
                            .collect::<Vec<_>>() })
                };
                let first_page = message["params"]["cursor"].is_null();
                let result = if first_page {
                    json!({ "data": [entry("gpt-6-luna", &["low", "medium", "high"])],
                        "nextCursor": "page-2" })
                } else {
                    json!({ "data": [entry("gpt-6.1-sol", &["low", "medium", "high", "xhigh"])],
                        "nextCursor": null })
                };
                respond(&mut stdout, json!({ "id": id, "result": result }));
            }
            "thread/start" if message["params"]["ephemeral"] == true => {
                assert_eq!(message["params"]["sandbox"], "read-only");
                assert_eq!(message["params"]["approvalPolicy"], "never");
                assert!(message["params"]["developerInstructions"].is_string());
                respond(
                    &mut stdout,
                    json!({ "id": id, "result": { "thread": { "id": "classifier-thread" } } }),
                );
            }
            "turn/start" if message["params"]["threadId"] == "classifier-thread" => {
                assert!(message["params"]["outputSchema"].is_object());
                assert_eq!(message["params"]["sandboxPolicy"]["type"], "readOnly");
                respond(
                    &mut stdout,
                    json!({ "id": id, "result": { "turn": { "id": "classifier-turn" } } }),
                );
                let text = if scenario == "classify-garbage" {
                    "no es json"
                } else {
                    "{\"tier\":\"fuerte\",\"reason\":\"fake classification\"}"
                };
                respond(
                    &mut stdout,
                    json!({ "method": "item/agentMessage/delta", "params": {
                        "threadId": "classifier-thread", "turnId": "classifier-turn", "delta": text
                    }}),
                );
                respond(
                    &mut stdout,
                    json!({ "method": "turn/completed", "params": {
                        "threadId": "classifier-thread",
                        "turn": { "id": "classifier-turn",
                            "status": if scenario == "classify-failed" { "failed" } else { "completed" },
                            "error": if scenario == "classify-failed" {
                                json!({ "message": "usage limit reached" })
                            } else { Value::Null } }
                    }}),
                );
            }
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
                let mut turns = json!([{
                    "id": if matches!(scenario.as_str(), "resume-wrong-turn" | "resume-wrong-interrupted-turn") {
                        "wrong-turn"
                    } else {
                        "fake-turn"
                    },
                    "status": if scenario == "resume-wrong-turn" { "completed" } else { status },
                    "items": [{ "type": "agentMessage", "text": "recovered completion" }]
                }]);
                if scenario == "resume-empty-history" {
                    turns = json!([]);
                } else if scenario == "resume-missing-turn-id" {
                    turns[0].as_object_mut().unwrap().remove("id");
                }
                respond(
                    &mut stdout,
                    json!({ "id": id, "result": { "thread": {
                    "id": "fake-thread", "turns": turns
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

/// Interactive chat scenario: threads served for `serviceName: roudex-chat`.
#[derive(Default)]
struct Chat {
    turns: u32,
    tokens: i64,
    waiting: Option<(u64, String)>,
    running: Option<String>,
}

impl Chat {
    fn handle(&mut self, out: &mut impl Write, message: &Value) -> bool {
        let method = message["method"].as_str().unwrap_or_default();
        let id = message["id"].clone();
        let params = &message["params"];
        match method {
            "" => {
                // Response to one of our server requests.
                let Some((request_id, turn_id)) = self.waiting.take() else {
                    return true;
                };
                assert_eq!(message["id"].as_u64(), Some(request_id));
                let outcome = match message["result"]["decision"].as_str() {
                    Some(decision) => format!("decision={decision}"),
                    None => format!("refused={}", message["error"]["code"]),
                };
                self.complete(out, &turn_id, &outcome, "completed");
                true
            }
            "thread/start" if params["serviceName"] == "roudex-chat" => {
                assert_eq!(params["approvalPolicy"], "on-request");
                assert_eq!(params["sandbox"], "workspace-write");
                respond(
                    out,
                    json!({ "id": id, "result": { "thread": { "id": "chat-thread" } } }),
                );
                true
            }
            "thread/resume"
                if params["threadId"]
                    .as_str()
                    .is_some_and(|t| t.starts_with("chat-")) =>
            {
                respond(
                    out,
                    json!({ "id": id, "result": { "thread": { "id": params["threadId"] } } }),
                );
                true
            }
            "turn/start" if params["threadId"] == "chat-thread" => {
                assert_eq!(params["approvalPolicy"], "on-request");
                assert_eq!(params["sandboxPolicy"]["type"], "workspaceWrite");
                self.turns += 1;
                let turn_id = format!("chat-turn-{}", self.turns);
                let prompt = params["input"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                respond(
                    out,
                    json!({ "id": id, "result": { "turn": { "id": turn_id } } }),
                );
                respond(
                    out,
                    json!({ "method": "turn/started", "params": {
                    "threadId": "chat-thread", "turn": { "id": turn_id, "status": "inProgress" } } }),
                );
                if prompt.contains("approval") {
                    self.waiting = Some((900, turn_id.clone()));
                    respond(
                        out,
                        json!({ "id": 900, "method": "item/commandExecution/requestApproval",
                        "params": { "threadId": "chat-thread", "turnId": turn_id, "itemId": "cmd-1",
                            "command": "cargo test", "cwd": ".", "reason": "run tests" } }),
                    );
                } else if prompt.contains("unsupported") {
                    self.waiting = Some((901, turn_id.clone()));
                    respond(
                        out,
                        json!({ "id": 901, "method": "item/tool/requestUserInput",
                        "params": { "threadId": "chat-thread", "turnId": turn_id } }),
                    );
                } else if prompt.contains("hang") {
                    self.running = Some(turn_id);
                } else {
                    self.complete(out, &turn_id, &format!("echo: {prompt}"), "completed");
                }
                true
            }
            "turn/interrupt" => {
                respond(out, json!({ "id": id, "result": {} }));
                if let Some(turn_id) = self.running.take() {
                    self.complete(out, &turn_id, "", "interrupted");
                }
                true
            }
            "account/rateLimits/read" => {
                respond(
                    out,
                    json!({ "id": id, "result": { "rateLimits": {
                    "primary": { "usedPercent": 42, "windowDurationMins": 300, "resetsAt": 1 } } } }),
                );
                true
            }
            _ => false,
        }
    }

    fn complete(&mut self, out: &mut impl Write, turn_id: &str, text: &str, status: &str) {
        if !text.is_empty() {
            for chunk in [&text[..text.len() / 2], &text[text.len() / 2..]] {
                respond(
                    out,
                    json!({ "method": "item/agentMessage/delta", "params": {
                    "threadId": "chat-thread", "turnId": turn_id, "itemId": "msg", "delta": chunk } }),
                );
            }
        }
        self.tokens += 100;
        respond(
            out,
            json!({ "method": "thread/tokenUsage/updated", "params": {
            "threadId": "chat-thread", "turnId": turn_id, "tokenUsage": {
                "total": { "inputTokens": self.tokens - 40, "cachedInputTokens": 10,
                    "outputTokens": 40, "reasoningOutputTokens": 5, "totalTokens": self.tokens },
                "last": { "inputTokens": 60, "cachedInputTokens": 10, "outputTokens": 40,
                    "reasoningOutputTokens": 5, "totalTokens": 100 } } } }),
        );
        respond(
            out,
            json!({ "method": "account/rateLimits/updated", "params": {
            "rateLimits": { "primary": { "usedPercent": 43 } } } }),
        );
        respond(
            out,
            json!({ "method": "turn/completed", "params": {
            "threadId": "chat-thread", "turn": { "id": turn_id, "status": status, "error": null } } }),
        );
    }
}
