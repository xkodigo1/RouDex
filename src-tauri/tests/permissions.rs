//! The app window must be allowed to call the core plugin commands the UI
//! uses. The browser fixtures mock the whole IPC, so only the real ACL built
//! from `tauri.conf.json` and `capabilities/` can catch a missing permission.

use serde_json::json;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindowBuilder;

fn call_from_main_window(cmd: &str, body: serde_json::Value) -> Result<(), String> {
    let app = mock_builder()
        .build(tauri::generate_context!("tauri.conf.json"))
        .expect("mock app");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("main window");
    get_ipc_response(
        &window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "http://localhost:5173".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[test]
fn the_window_can_listen_to_session_events() {
    // Without it the UI never sees streaming text or the end of a turn.
    call_from_main_window(
        "plugin:event|listen",
        json!({ "event": "roudex://session", "target": { "kind": "Any" }, "handler": 0 }),
    )
    .unwrap();
}
