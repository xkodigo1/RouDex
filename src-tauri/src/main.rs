use router_core::{
    desktop_hook, escalate_local, project_dir, route_local, AccessMode, AppServerClient,
    AppServerConfig, ApprovalDecision, ChatService, ChatThread, ChatTurn, Dashboard, Language,
    Migration, RouteDecision, RouterConfig, RouterStore, SavingsSummary, SentTurn, SpecFile,
    StoredSpec, TaskBoard, TaskExecutionReceipt, TaskService, ThreadSummary, Tier,
};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tauri::{Emitter, Manager};

/// Every session event is emitted to the UI under this name.
const SESSION_EVENT: &str = "roudex://session";
/// Events of spec task runs (session events tagged with their task, and the
/// end of each run).
const TASK_EVENT: &str = "roudex://task";

struct AppState {
    chat: Arc<ChatService>,
    tasks: Arc<TaskService>,
    /// Data copied from the pre-0.9.0 location at this start, if any.
    migration: Option<Migration>,
}

/// Run blocking work (process I/O, SQLite) off the async runtime.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())?
}

/// Router decision for a prompt; nothing is executed.
#[tauri::command]
async fn router_decide(
    state: tauri::State<'_, AppState>,
    project: String,
    prompt: String,
) -> Result<RouteDecision, String> {
    let chat = state.chat.clone();
    blocking(move || {
        let decision = route_local(&project_dir(Path::new(&project))?, &prompt)?;
        if let Some(usage) = &decision.router_usage {
            // The decision still counts if its cost cannot be kept.
            let _ = chat
                .recorder()
                .with_store(|store| store.record_router_usage(usage));
        }
        Ok(decision)
    })
    .await
}

/// Estimated cost and savings of the last 30 days (spec 005, D5).
#[tauri::command]
async fn savings_summary(state: tauri::State<'_, AppState>) -> Result<SavingsSummary, String> {
    let chat = state.chat.clone();
    blocking(move || {
        let config = RouterConfig::load_or_default(&RouterConfig::default_path())
            .map_err(|error| error.to_string())?;
        let sessions = config
            .savings
            .include_codex_desktop
            .then(router_core::codex_sessions::sessions_dir);
        chat.recorder()
            .with_store(|store| store.savings(&config, sessions.as_deref()))
            .map_err(|error| error.to_string())
    })
    .await
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

/// Count Codex Desktop turns in the savings estimate, or stop counting them.
#[tauri::command]
fn settings_set_codex_desktop(enabled: bool) -> Result<RouterConfig, String> {
    RouterConfig::write_setting(
        &RouterConfig::default_path(),
        Some("savings"),
        "include_codex_desktop",
        &enabled.to_string(),
    )
    .map_err(|error| error.to_string())
}

/// Store the interface language in router.toml, where the Desktop hook and
/// the router read it too. Only that line of the file changes.
#[tauri::command]
fn settings_set_language(language: Language) -> Result<RouterConfig, String> {
    let value = format!("\"{}\"", language.as_str());
    RouterConfig::write_setting(&RouterConfig::default_path(), None, "language", &value)
        .map_err(|error| error.to_string())
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
/// `access` picks the mode of a new thread (write when omitted); an existing
/// thread keeps its own.
#[tauri::command]
async fn chat_send(
    state: tauri::State<'_, AppState>,
    project: String,
    thread_id: Option<String>,
    prompt: String,
    decision: RouteDecision,
    access: Option<AccessMode>,
) -> Result<SentTurn, String> {
    let chat = state.chat.clone();
    blocking(move || {
        let project = project_dir(Path::new(&project))?;
        chat.send(
            &project,
            thread_id.as_deref(),
            &prompt,
            &decision,
            access.unwrap_or_default(),
        )
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

/// Codex Desktop decisions and whether the RouDex hook is in `hooks.json`.
#[tauri::command]
fn desktop_hook_summary() -> Result<Value, String> {
    // The hook writes to its own store outside AppData (see desktop_hook::home).
    let store_path = desktop_hook::store_path();
    let store = if store_path.exists() {
        RouterStore::open(&store_path)
    } else {
        RouterStore::in_memory()
    };
    let summary = store
        .and_then(|store| store.desktop_summary())
        .map_err(|error| error.to_string())?;
    let path = desktop_hook::hooks_path();
    let hook = match desktop_hook::read_hooks(&path) {
        Ok(document) => {
            json!({ "path": path, "command": desktop_hook::installed_command(&document) })
        }
        Err(error) => json!({ "path": path, "command": null, "error": error }),
    };
    Ok(json!({ "summary": summary, "hook": hook }))
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

/// Specs under `<project>/specs/*/tasks.json`.
#[tauri::command]
async fn spec_files(
    state: tauri::State<'_, AppState>,
    project: String,
) -> Result<Vec<SpecFile>, String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.spec_files(&project_dir(Path::new(&project))?)).await
}

/// Import (or re-import) a spec manifest.
#[tauri::command]
async fn spec_load(state: tauri::State<'_, AppState>, path: String) -> Result<StoredSpec, String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.load_spec(Path::new(&path))).await
}

/// Loaded specs, most recent first.
#[tauri::command]
async fn spec_list(state: tauri::State<'_, AppState>) -> Result<Vec<StoredSpec>, String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.specs()).await
}

/// A spec's tasks with the route each would take now.
#[tauri::command]
async fn task_board(
    state: tauri::State<'_, AppState>,
    spec_id: String,
) -> Result<TaskBoard, String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.board(&spec_id)).await
}

/// Choose a task's tier by hand; `null` returns to the spec's.
#[tauri::command]
async fn task_set_tier(
    state: tauri::State<'_, AppState>,
    task_id: String,
    tier: Option<Tier>,
) -> Result<(), String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.set_tier(&task_id, tier)).await
}

#[tauri::command]
async fn task_decide(
    state: tauri::State<'_, AppState>,
    task_id: String,
    approved: bool,
) -> Result<(), String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.decide(&task_id, approved)).await
}

/// Start a task in the background; its events arrive on `roudex://task`.
#[tauri::command]
fn task_dispatch(state: tauri::State<'_, AppState>, task_id: String) -> Result<(), String> {
    state.tasks.dispatch(&task_id)
}

/// Run every ready task of a spec, one after another, in the background.
#[tauri::command]
fn task_dispatch_ready(state: tauri::State<'_, AppState>, spec_id: String) -> Result<(), String> {
    state.tasks.dispatch_ready(&spec_id)
}

/// Continue an interrupted run on its thread, in the background.
#[tauri::command]
fn task_resume(state: tauri::State<'_, AppState>, run_id: String) -> Result<(), String> {
    state.tasks.resume(&run_id)
}

#[tauri::command]
async fn task_interrupt(state: tauri::State<'_, AppState>, task_id: String) -> Result<(), String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.interrupt(&task_id)).await
}

/// Answer an approval request of a running task.
#[tauri::command]
async fn task_approval(
    state: tauri::State<'_, AppState>,
    task_id: String,
    request_id: Value,
    decision: ApprovalDecision,
) -> Result<(), String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.respond(&task_id, request_id, decision)).await
}

#[tauri::command]
async fn task_retry(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<TaskExecutionReceipt, String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.retry(&task_id)).await
}

/// Mark a task left running by a closed app as interrupted.
#[tauri::command]
async fn task_recover(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<TaskExecutionReceipt, String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.recover(&task_id)).await
}

/// Persisted events of a run, for its transcript and verification output.
#[tauri::command]
async fn task_run_events(
    state: tauri::State<'_, AppState>,
    run_id: String,
) -> Result<Vec<router_core::EventRecord>, String> {
    let tasks = state.tasks.clone();
    blocking(move || tasks.run_events(&run_id)).await
}

/// Data this start copied from the pre-0.9.0 location, to tell the user once.
#[tauri::command]
fn startup_migration(state: tauri::State<'_, AppState>) -> Option<Migration> {
    state.migration.clone()
}

/// What is running now, if anything.
#[tauri::command]
fn task_active(state: tauri::State<'_, AppState>) -> Option<String> {
    state.tasks.active()
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Before anything opens the store: a failed copy must not leave
            // an empty store that would stop the next start from retrying.
            let migration = router_core::paths::migrate_legacy()?;
            let handle = app.handle().clone();
            let store = RouterStore::open(RouterStore::default_path())?;
            let chat = ChatService::new(
                store,
                Box::new(|cwd: &Path| AppServerConfig::local(cwd)),
                Arc::new(move |event| {
                    let _ = handle.emit(SESSION_EVENT, &event);
                }),
            );
            let handle = app.handle().clone();
            let tasks = TaskService::new(
                RouterStore::default_path(),
                Arc::new(|cwd: &Path| AppServerConfig::local(cwd)),
                Arc::new(move |event| {
                    let _ = handle.emit(TASK_EVENT, &event);
                }),
            );
            app.manage(AppState {
                chat: Arc::new(chat),
                tasks: Arc::new(tasks),
                migration,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            router_decide,
            router_escalate,
            router_config,
            settings_set_language,
            settings_set_codex_desktop,
            savings_summary,
            router_models,
            chat_send,
            chat_approval,
            chat_interrupt,
            chat_threads,
            chat_turns,
            usage_summary,
            dashboard_summary,
            desktop_hook_summary,
            spec_files,
            spec_load,
            spec_list,
            task_board,
            task_set_tier,
            task_decide,
            task_dispatch,
            task_dispatch_ready,
            task_resume,
            task_interrupt,
            task_approval,
            task_retry,
            task_recover,
            task_run_events,
            task_active,
            startup_migration,
            thread_list,
            chat_projects,
            thread_rename,
            thread_archive
        ])
        .run(tauri::generate_context!())
        .expect("error while running desktop application");
}
