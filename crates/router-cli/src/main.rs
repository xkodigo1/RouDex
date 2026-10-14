use router_core::{
    desktop_hook, AccessMode, ApprovalDecision, ChatService, RiskRules, SessionEvent,
};
use router_core::{
    route_local, AppServerClient, AppServerConfig, LocalTaskRuntime, RouterConfig, RouterStore,
    RuntimeTurn, TaskApproval, TaskCoordinator, TaskDefinition, TaskManifest, TaskRuntime,
};
use serde_json::{json, Value};
use std::env;
use std::io::{self, Read};
use std::path::Path;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next();
    if matches!(
        command.as_deref(),
        Some(
            "task-plan"
                | "spec-load"
                | "task-dispatch"
                | "task-approve"
                | "task-run"
                | "task-recover"
                | "task-retry"
                | "continue-run"
        )
    ) && (env::var_os("ROUDEX_ROUTER_INTERNAL").is_some()
        || env::var_os("ROUDEX_ACTIVE_TASK_ID").is_some())
    {
        return Err("un worker activo no puede invocar el ciclo del coordinador".to_owned());
    }
    match command.as_deref() {
        Some("hook") => {
            hook();
            Ok(())
        }
        Some("hook-install") => hook_install(),
        Some("hook-uninstall") => hook_uninstall(),
        Some("chat") => chat(checked_args(args.collect(), 1, 1, "chat <projectDir>")?),
        Some("route") => route(checked_args(args.collect(), 1, 1, "route <projectDir>")?),
        Some("models") => models(checked_args(args.collect(), 0, 1, "models [projectDir]")?),
        Some("continue-run") => {
            continue_run(checked_args(args.collect(), 1, 1, "continue-run <runId>")?)
        }
        Some("spec-load" | "task-plan") => task_plan(checked_args(
            args.collect(),
            1,
            1,
            "spec-load <manifest.json>",
        )?),
        Some("task-list") => task_list(checked_args(args.collect(), 1, 1, "task-list <specId>")?),
        Some("task-dispatch") => task_dispatch(checked_args(
            args.collect(),
            1,
            1,
            "task-dispatch <specId>",
        )?),
        Some("task-approve") => task_approve(checked_args(
            args.collect(),
            1,
            2,
            "task-approve <taskId> [approved|denied]",
        )?),
        Some("task-run") => task_run(args.collect()),
        Some("task-recover") => task_recover(args.collect()),
        Some("task-retry") => task_retry(args.collect()),
        Some("task-status") => task_status(args.collect()),
        Some("run-events") => run_events(args.collect()),
        Some("status") => status(),
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn project_dir(arg: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(arg);
    if !path.is_dir() {
        return Err(format!("no existe la carpeta del proyecto: {arg}"));
    }
    Ok(path)
}

fn router_config() -> Result<RouterConfig, String> {
    RouterConfig::load_or_default(&RouterConfig::default_path()).map_err(|error| error.to_string())
}

/// Route a prompt read from stdin and print the decision; nothing is executed.
fn route(args: Vec<String>) -> Result<(), String> {
    let cwd = project_dir(&args[0])?;
    let mut prompt = String::new();
    io::stdin()
        .read_to_string(&mut prompt)
        .map_err(|error| format!("could not read prompt: {error}"))?;
    let decision = route_local(&cwd, &prompt)?;
    write_json(&serde_json::to_value(decision).map_err(|error| error.to_string())?)
}

/// Route a prompt from stdin, send it in a new thread and print every session
/// event as JSON until the turn ends. Approval requests are declined.
fn chat(args: Vec<String>) -> Result<(), String> {
    let project = router_core::project_dir(Path::new(&args[0]))?;
    let mut prompt = String::new();
    io::stdin()
        .read_to_string(&mut prompt)
        .map_err(|error| format!("could not read prompt: {error}"))?;
    let decision = route_local(&project, &prompt)?;
    write_json(&json!({ "decision": decision }))?;
    let (events, received) = std::sync::mpsc::channel();
    let service = ChatService::new(
        RouterStore::open(RouterStore::default_path()).map_err(|error| error.to_string())?,
        Box::new(|cwd: &Path| AppServerConfig::local(cwd)),
        std::sync::Arc::new(move |event: SessionEvent| {
            let _ = events.send(event);
        }),
    );
    let sent = service.send(&project, None, prompt.trim(), &decision, AccessMode::Write)?;
    write_json(&json!({ "sent": sent }))?;
    for event in received {
        write_json(&serde_json::to_value(&event).map_err(|error| error.to_string())?)?;
        match event {
            SessionEvent::ApprovalRequested { request_id, .. } => {
                service.respond_approval(&project, request_id, ApprovalDecision::Decline)?;
            }
            SessionEvent::TurnCompleted { turn_id, .. } if turn_id == sent.turn_id => break,
            SessionEvent::Closed { reason } => return Err(reason),
            _ => {}
        }
    }
    Ok(())
}

/// Runtime model catalog and validation of the configured tiers.
fn models(args: Vec<String>) -> Result<(), String> {
    let cwd = match args.first() {
        Some(arg) => project_dir(arg)?,
        None => env::current_dir().map_err(|error| error.to_string())?,
    };
    let config = router_config()?;
    let catalog = AppServerClient::start(&AppServerConfig::local(&cwd))
        .and_then(|mut client| client.list_models())
        .map_err(|error| format!("no se pudo leer el catálogo del runtime: {error}"))?;
    write_json(&json!({
        "configPath": RouterConfig::default_path(),
        "config": config,
        "issues": config.catalog_issues(&catalog),
        "catalog": catalog,
    }))
}

fn checked_args(
    args: Vec<String>,
    min: usize,
    max: usize,
    usage: &str,
) -> Result<Vec<String>, String> {
    if !(min..=max).contains(&args.len()) || args.iter().any(|arg| arg.trim().is_empty()) {
        return Err(format!("Usage: router-cli {usage}"));
    }
    Ok(args)
}

fn task_list(args: Vec<String>) -> Result<(), String> {
    let spec_id = &args[0];
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    let tasks = store
        .tasks_for_spec(spec_id)
        .map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    for task in tasks {
        let approval = store
            .latest_task_approval(&task.id)
            .map_err(|error| error.to_string())?;
        let run = store
            .latest_run_for_task(&task.id)
            .map_err(|error| error.to_string())?;
        let dependencies_completed = store
            .dependencies_completed(&task.id)
            .map_err(|error| error.to_string())?;
        entries.push(json!({
            "task": task,
            "approval": approval,
            "latestRun": run,
            "dependenciesCompleted": dependencies_completed
        }));
    }
    write_json(&json!({ "specId": spec_id, "tasks": entries }))
}

fn task_dispatch(args: Vec<String>) -> Result<(), String> {
    let spec_id = &args[0];
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    if store
        .tasks_for_spec(spec_id)
        .map_err(|error| error.to_string())?
        .is_empty()
    {
        return Err(format!("spec not found: {spec_id}"));
    }
    let receipts = TaskCoordinator::new(&store)
        .dispatch_ready_tasks(spec_id)
        .map_err(|error| error.to_string())?;
    write_json(&json!({ "specId": spec_id, "receipts": receipts }))
}

fn task_plan(args: Vec<String>) -> Result<(), String> {
    let manifest_path = args
        .first()
        .ok_or_else(|| "task-plan requiere la ruta de un manifest JSON".to_owned())?;
    let manifest = TaskManifest::load(manifest_path).map_err(|error| error.to_string())?;
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    TaskCoordinator::new(&store)
        .import_manifest(&manifest)
        .map_err(|error| error.to_string())?;
    // Reimports preserve lifecycle state; report readiness from that same state.
    let persisted = TaskManifest {
        tasks: store
            .tasks_for_spec(&manifest.spec_id)
            .map_err(|error| error.to_string())?,
        ..manifest
    };
    write_json(&json!({
        "specId": persisted.spec_id,
        "taskCount": persisted.tasks.len(),
        "readyTaskIds": persisted.ready_tasks().iter().map(|task| task.id.clone()).collect::<Vec<_>>(),
        "warnings": persisted.warnings
    }))
}

fn task_approve(args: Vec<String>) -> Result<(), String> {
    let task_id = args
        .first()
        .ok_or_else(|| "task-approve requiere un taskId".to_owned())?;
    let decision = args.get(1).map(String::as_str).unwrap_or("approved");
    if !matches!(decision, "approved" | "denied") {
        return Err("la decisión debe ser approved o denied".to_owned());
    }
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    store
        .get_task(task_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("task no encontrada: {task_id}"))?;
    store
        .record_task_approval(&TaskApproval {
            task_id: task_id.to_owned(),
            decision: decision.to_owned(),
            reason: Some("explicit CLI decision".to_owned()),
        })
        .map_err(|error| error.to_string())?;
    write_json(&json!({ "taskId": task_id, "decision": decision }))
}

fn task_run(args: Vec<String>) -> Result<(), String> {
    let task_id = args
        .first()
        .ok_or_else(|| "task-run requiere un taskId".to_owned())?;
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    let task = store
        .get_task(task_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("task no encontrada: {task_id}"))?;
    let needs_approval = task.definition.approval != "not_required"
        && store
            .latest_task_approval(task_id)
            .map_err(|error| error.to_string())?
            .as_ref()
            .map(|approval| approval.decision.as_str())
            != Some("approved");
    if needs_approval {
        let mut probe = ApprovalProbe;
        let receipt = TaskCoordinator::new(&store)
            .dispatch_task(task_id, &mut probe)
            .map_err(|error| error.to_string())?;
        return write_json(&serde_json::to_value(receipt).map_err(|error| error.to_string())?);
    }
    let mut runtime =
        LocalTaskRuntime::start(&task.definition).map_err(|error| error.to_string())?;
    let receipt = TaskCoordinator::new(&store)
        .dispatch_task(task_id, &mut runtime)
        .map_err(|error| error.to_string())?;
    write_json(&serde_json::to_value(receipt).map_err(|error| error.to_string())?)
}

fn task_recover(args: Vec<String>) -> Result<(), String> {
    let task_id = args
        .first()
        .ok_or_else(|| "task-recover requiere un taskId".to_owned())?;
    let reason = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("worker process interrupted");
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    let receipt = TaskCoordinator::new(&store)
        .recover_running_task(task_id, reason)
        .map_err(|error| error.to_string())?;
    write_json(&serde_json::to_value(receipt).map_err(|error| error.to_string())?)
}

fn task_retry(args: Vec<String>) -> Result<(), String> {
    let task_id = args
        .first()
        .ok_or_else(|| "task-retry requiere un taskId".to_owned())?;
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    let receipt = TaskCoordinator::new(&store)
        .retry_interrupted_task(task_id)
        .map_err(|error| error.to_string())?;
    write_json(&serde_json::to_value(receipt).map_err(|error| error.to_string())?)
}

struct ApprovalProbe;

impl TaskRuntime for ApprovalProbe {
    fn start_thread(&mut self, _task: &TaskDefinition) -> Result<String, String> {
        Err("approval probe must not start a thread".to_owned())
    }

    fn run_turn(
        &mut self,
        _task: &TaskDefinition,
        _thread_id: &str,
    ) -> Result<RuntimeTurn, String> {
        Err("approval probe must not run a turn".to_owned())
    }
}

fn task_status(args: Vec<String>) -> Result<(), String> {
    let task_id = args
        .first()
        .ok_or_else(|| "task-status requiere un taskId".to_owned())?;
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    let task = store
        .get_task(task_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("task no encontrada: {task_id}"))?;
    write_json(&serde_json::to_value(task).map_err(|error| error.to_string())?)
}

fn run_events(args: Vec<String>) -> Result<(), String> {
    let run_id = args
        .first()
        .ok_or_else(|| "run-events requiere un runId".to_owned())?;
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    let events = store
        .events_for_run(run_id)
        .map_err(|error| error.to_string())?;
    write_json(&serde_json::to_value(events).map_err(|error| error.to_string())?)
}

fn continue_run(args: Vec<String>) -> Result<(), String> {
    let run_id = args
        .first()
        .ok_or_else(|| "continue-run requiere un runId".to_owned())?;
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    let run = store
        .get_run(run_id)
        .map_err(|error| format!("could not read run: {error}"))?
        .ok_or_else(|| format!("runId no encontrado: {run_id}"))?;
    // Task runs must retain the coordinator's immutable assignment, approval
    // gates and terminal replay protection. User text cannot replace that task.
    if store
        .events_for_run(run_id)
        .map_err(|error| error.to_string())?
        .iter()
        .any(|event| event.event_type == "task.started")
        || store
            .get_task(&run.turn_id)
            .map_err(|error| error.to_string())?
            .is_some_and(|task| task.spec_id == run.session_id)
    {
        let mut runtime = LocalTaskRuntime::with_config(
            AppServerConfig::local(&run.cwd).with_active_task(&run.turn_id),
        );
        let receipt = TaskCoordinator::new(&store)
            .resume_run(run_id, &mut runtime)
            .map_err(|error| error.to_string())?;
        return write_json(&serde_json::to_value(receipt).map_err(|error| error.to_string())?);
    }
    Err(format!(
        "el runId no pertenece a una task del coordinador: {run_id}"
    ))
}

fn status() -> Result<(), String> {
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    write_json(&json!({
        "statePath": store.path(),
        "runCount": store.count_runs().map_err(|error| error.to_string())?
    }))
}

/// Codex `UserPromptSubmit` hook. It never fails: any problem lets the prompt
/// through and is reported on stderr. RouDex's own app-server turns are skipped.
fn hook() {
    if env::var_os("ROUDEX_ROUTER_INTERNAL").is_some()
        || env::var_os("ROUDEX_ACTIVE_TASK_ID").is_some()
    {
        return;
    }
    let mut text = String::new();
    let response = io::stdin()
        .read_to_string(&mut text)
        .map_err(|error| format!("no se pudo leer la entrada: {error}"))
        .and_then(|_| hook_response(&text));
    match response {
        Ok(response) => println!("{response}"),
        Err(error) => {
            eprintln!("RouDex hook: {error}; el mensaje sigue sin contexto");
            // Codex Desktop does not show hook stderr; keep the reason locally.
            desktop_hook::log_failure(
                &desktop_hook::log_path(),
                &format!("{error} | campos: {}", desktop_hook::input_shape(&text)),
            );
        }
    }
}

fn hook_response(text: &str) -> Result<String, String> {
    let input: desktop_hook::PromptSubmit =
        serde_json::from_str(text).map_err(|error| format!("entrada inválida: {error}"))?;
    let config = router_config()?;
    let rules = RiskRules::for_project(&input.cwd).map_err(|error| error.to_string())?;
    let outcome = desktop_hook::evaluate(&input, &config, &rules);
    // A store problem must not hide the decision from Codex.
    if let Err(error) = RouterStore::open(desktop_hook::store_path())
        .and_then(|store| store.record_desktop_prompt(&input, &outcome))
    {
        let message = format!("no se registró la decisión: {error}");
        eprintln!("RouDex hook: {message}");
        desktop_hook::log_failure(&desktop_hook::log_path(), &message);
    }
    serde_json::to_string(&outcome.response).map_err(|error| error.to_string())
}

/// Copy this binary to the RouDex home and point Codex's user `hooks.json`
/// at that copy, replacing any earlier RouDex hook.
fn hook_install() -> Result<(), String> {
    let target = desktop_hook::bin_path();
    desktop_hook::check_outside_app_data(&target)?;
    let command = desktop_hook::hook_command(&target)?;
    let exe = env::current_exe().map_err(|error| error.to_string())?;
    let same = |a: &Path, b: &Path| match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if !same(&exe, &target) {
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        }
        std::fs::copy(&exe, &target).map_err(|error| {
            format!(
                "no se pudo copiar router-cli a {}: {error}",
                target.display()
            )
        })?;
    }
    let path = desktop_hook::hooks_path();
    let mut document = desktop_hook::read_hooks(&path)?;
    let change = desktop_hook::install_into(&mut document, &command)?;
    desktop_hook::write_hooks(&path, &document)?;
    eprintln!("Codex pedirá revisar y confiar en el hook antes de ejecutarlo.");
    let legacy_managed = desktop_hook::legacy_managed_hook();
    if let Some(managed) = &legacy_managed {
        eprintln!(
            "Aviso: {} contiene el hook administrado de RouDex v0.5.0. Renómbralo como administrador; \
             consulta docs/desktop-hook.md.",
            managed.display()
        );
    }
    write_json(&json!({
        "hooksPath": path,
        "command": command,
        "binary": target,
        "replaced": change.removed - change.removed_legacy,
        "removedLegacy": change.removed_legacy,
        "legacyManagedHook": legacy_managed
    }))
}

/// Remove RouDex hooks from Codex's user `hooks.json`; other hooks stay.
fn hook_uninstall() -> Result<(), String> {
    let path = desktop_hook::hooks_path();
    let mut document = desktop_hook::read_hooks(&path)?;
    let change = desktop_hook::uninstall_from(&mut document)?;
    if change.removed > 0 {
        desktop_hook::write_hooks(&path, &document)?;
    }
    write_json(&json!({ "hooksPath": path, "removed": change.removed }))
}

fn write_json(value: &Value) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string(value)
            .map_err(|error| format!("could not encode output: {error}"))?
    );
    Ok(())
}

fn print_usage() {
    eprintln!(
        "Usage: router-cli <hook (Codex UserPromptSubmit input on stdin)|hook-install|hook-uninstall|chat <projectDir> (prompt on stdin)|route <projectDir> (prompt on stdin)|models [projectDir]|spec-load <manifest.json>|task-list <specId>|task-approve <taskId> [approved|denied]|task-dispatch <specId>|continue-run <runId>|task-plan <manifest>|task-run <taskId>|task-recover <taskId> [reason]|task-retry <taskId>|task-status <taskId>|run-events <runId>|status>"
    );
}
