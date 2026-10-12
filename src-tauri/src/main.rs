use router_core::RouterStore;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct LocalStatus {
    state_path: PathBuf,
    run_count: i64,
}

#[tauri::command]
fn local_status() -> Result<LocalStatus, String> {
    let store =
        RouterStore::open(RouterStore::default_path()).map_err(|error| error.to_string())?;
    Ok(LocalStatus {
        state_path: store.path().to_path_buf(),
        run_count: store.count_runs().map_err(|error| error.to_string())?,
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![local_status])
        .run(tauri::generate_context!())
        .expect("error while running desktop application");
}
