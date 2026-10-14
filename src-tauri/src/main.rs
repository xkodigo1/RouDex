use router_core::{
    escalate_local, project_dir, route_local, AppServerClient, AppServerConfig, ApprovalDecision,
    ChatService, ChatThread, ChatTurn, Dashboard, RouteDecision, RouterConfig, RouterStore,
    SentTurn, ThreadSummary,
};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tauri::{Emitter, Manager};

/// Every session event is emitted to the UI under this name.
const SESSION_EVENT: &str = "roudex://session";

struct AppState {
    chat: Arc<ChatService>,
}

/// Run blocking work (process I/O, SQLite) off the async runtime.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
fn local_snapshot() -> Result<roudex_desktop::LocalSnapshot, String> {
    roudex_desktop::local_snapshot(&RouterStore::default_path())
}

#[tauri::command]
fn local_run_events(run_id: String) -> Result<roudex_desktop::RunEvents, String> {
    roudex_desktop::local_run_events(&RouterStore::default_path(), &run_id)
}

/// Router decision for a prompt; nothing is executed.
#[tauri::command]
async fn router_decide(project: String, prompt: String) -> Result<RouteDecision, String> {
    blocking(move || route_local(&project_dir(Path::new(&project))?, &prompt)).await
}

#[tauri::command]
fn router_escalate(decision: RouteDecision) -> Result<RouteDecision, String> {
    escalate_local(&decision)
}

/// Active router configuration (tier targets), without starting a runtime.
#[tauri::command]
fn router_config() -> Result<RouterConfig, String> {
    RouterConfig::load_or_default(&RouterConfig::default_path()).map_err(|error| error.to_string())
}

/// Runtime catalog and problems with the configured tiers.
#[tauri::command]
async fn router_models(project: String) -> Result<Value, String> {
    blocking(move || {
        let project = project_dir(Path::new(&project))?;
        let config = RouterConfig::load_or_default(&RouterConfig::default_path())
            .map_err(|error| error.to_string())?;
        let catalog = AppServerClient::start(&AppServerConfig::local(&project))
            .and_then(|mut client| client.list_models())
            .map_err(|error| error.to_string())?;
        Ok(json!({
            "config": config,
            "issues": config.catalog_issues(&catalog),
            "catalog": catalog,
        }))
    })
    .await
}

/// Send a prompt with the decision the user reviewed (possibly overridden).
#[tauri::command]
async fn chat_send(
    state: tauri::State<'_, AppState>,
    project: String,
    thread_id: Option<String>,
    prompt: String,
    decision: RouteDecision,
) -> Result<SentTurn, String> {
    let chat = state.chat.clone();
    blocking(move || {
        let project = project_dir(Path::new(&project))?;
        chat.send(&project, thread_id.as_deref(), &prompt, &decision)
    })
    .await
}

#[tauri::command]
async fn chat_approval(
    state: tauri::State<'_, AppState>,
    project: String,
    request_id: Value,
    decision: ApprovalDecision,
) -> Result<(), String> {
    let chat = state.chat.clone();
    blocking(move || {
        chat.respond_approval(&project_dir(Path::new(&project))?, request_id, decision)
    })
    .await
}

#[tauri::command]
async fn chat_interrupt(
    state: tauri::State<'_, AppState>,
    project: String,
    thread_id: String,
    turn_id: String,
) -> Result<(), String> {
    let chat = state.chat.clone();
    blocking(move || chat.interrupt(&project_dir(Path::new(&project))?, &thread_id, &turn_id)).await
}

#[tauri::command]
fn chat_threads(
    state: tauri::State<'_, AppState>,
    project: String,
) -> Result<Vec<ChatThread>, String> {
    let project = project_dir(Path::new(&project))?;
    state
        .chat
        .recorder()
        .with_store(|store| store.chat_threads(Some(&project)))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn chat_turns(
    state: tauri::State<'_, AppState>,
    thread_id: String,
) -> Result<Vec<ChatTurn>, String> {
    state
        .chat
        .recorder()
        .with_store(|store| store.chat_turns(&thread_id))
        .map_err(|error| error.to_string())
}

/// Everything the summary screen shows, from the local store.
#[tauri::command]
fn dashboard_summary(state: tauri::State<'_, AppState>) -> Result<Dashboard, String> {
    state
        .chat
        .recorder()
        .with_store(|store| store.dashboard())
        .map_err(|error| error.to_string())
}

/// Threads for the thread list; all projects when `project` is omitted.
#[tauri::command]
fn thread_list(
    state: tauri::State<'_, AppState>,
    project: Option<String>,
    include_archived: Option<bool>,
) -> Result<Vec<ThreadSummary>, String> {
    let project = project
        .map(|path| project_dir(Path::new(&path)))
        .transpose()?;
    state
        .chat
        .recorder()
        .with_store(|store| {
            store.thread_summaries(project.as_deref(), include_archived.unwrap_or(false), None)
        })
        .map_err(|error| error.to_string())
}

/// Project folders that already have threads, most recent first.
#[tauri::command]
fn chat_projects(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    state
        .chat
        .recorder()
        .with_store(|store| store.chat_projects())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn thread_rename(
    state: tauri::State<'_, AppState>,
    thread_id: String,
    title: String,
) -> Result<bool, String> {
    state
        .chat
        .recorder()
        .with_store(|store| store.rename_chat_thread(&thread_id, &title))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn thread_archive(
    state: tauri::State<'_, AppState>,
    thread_id: String,
    archived: bool,
) -> Result<bool, String> {
    state
        .chat
        .recorder()
        .with_store(|store| store.archive_chat_thread(&thread_id, archived))
        .map_err(|error| error.to_string())
}

/// Token usage per model plus the latest rate-limit snapshot. Reads the
/// limits from the runtime only when no snapshot was seen yet.
#[tauri::command]
async fn usage_summary(
    state: tauri::State<'_, AppState>,
    project: String,
) -> Result<Value, String> {
    let chat = state.chat.clone();
    blocking(move || {
        let models = chat
            .recorder()
            .with_store(|store| store.usage_by_model())
            .map_err(|error| error.to_string())?;
        let rate_limits = match chat.recorder().last_rate_limits() {
            Some(snapshot) => snapshot,
            None => chat.rate_limits(&project_dir(Path::new(&project))?)?["rateLimits"].clone(),
        };
        Ok(json!({ "models": models, "rateLimits": rate_limits }))
    })
    .await
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let store = RouterStore::open(RouterStore::default_path())?;
            let chat = ChatService::new(
                store,
                Box::new(|cwd: &Path| AppServerConfig::local(cwd)),
                Arc::new(move |event| {
                    let _ = handle.emit(SESSION_EVENT, &event);
                }),
            );
            app.manage(AppState {
                chat: Arc::new(chat),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            local_snapshot,
            local_run_events,
            router_decide,
            router_escalate,
            router_config,
            router_models,
            chat_send,
            chat_approval,
            chat_interrupt,
            chat_threads,
            chat_turns,
            usage_summary,
            dashboard_summary,
            thread_list,
            chat_projects,
            thread_rename,
            thread_archive
        ])
        .run(tauri::generate_context!())
        .expect("error while running desktop application");
}
