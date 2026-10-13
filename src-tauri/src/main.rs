#[tauri::command]
fn local_snapshot() -> Result<roudex_desktop::LocalSnapshot, String> {
    roudex_desktop::local_snapshot(&router_core::RouterStore::default_path())
}

#[tauri::command]
fn local_run_events(run_id: String) -> Result<roudex_desktop::RunEvents, String> {
    roudex_desktop::local_run_events(&router_core::RouterStore::default_path(), &run_id)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![local_snapshot, local_run_events])
        .run(tauri::generate_context!())
        .expect("error while running desktop application");
}
