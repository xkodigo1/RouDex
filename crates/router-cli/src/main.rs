use router_core::{
    AppServerClient, AppServerConfig, DesktopPromptInput, EventRecord, LocalTaskRuntime,
    RouterStore, RoutingConfig, RuntimeTurn, TaskApproval, TaskCoordinator, TaskDefinition,
    TaskManifest, TaskRuntime,
};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

fn main() {
    let command = env::args().nth(1);
    if let Err(error) = run() {
        if matches!(
            command.as_deref(),
            Some("route-prompt") | Some("continue-run")
        ) {
            let _ = write_block(&format!("Enrutamiento rechazado: {error}"));
            return;
        } else {
            eprintln!("{error}");
        }
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
        Some("route-preview") => route_preview(args.collect()),
        Some("route-prompt") => route_prompt(args.collect()),
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
        Some("install-hook") => install_hook(),
        Some("install-managed-hook") => install_managed_hook(),
        Some("uninstall-hook") => uninstall_hook(),
        Some("uninstall-managed-hook") => uninstall_managed_hook(),
        Some("status") => status(),
        _ => {
            print_usage();
            Ok(())
        }
    }
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

// Pure preview: never open the operational store or construct a runtime. Explicit
// configuration is a candidate supplied on stdin, not an active dispatch revision.
fn route_preview(args: Vec<String>) -> Result<(), String> {
    const USAGE: &str = "Usage: router-cli route-preview <profile> [--config-stdin] [--model <model>] [--effort <effort>]";
    let profile = args
        .first()
        .filter(|arg| !arg.trim().is_empty())
        .ok_or(USAGE)?;
    let mut config_stdin = false;
    let mut model = None;
    let mut effort = None;
    let mut remaining = args.iter().skip(1);
    while let Some(flag) = remaining.next() {
        match flag.as_str() {
            "--config-stdin" if !config_stdin => config_stdin = true,
            "--model" if model.is_none() => {
                model = Some(
                    remaining
                        .next()
                        .filter(|v| !v.starts_with("--") && !v.trim().is_empty())
                        .ok_or(USAGE)?,
                );
            }
            "--effort" if effort.is_none() => {
                effort = Some(
                    remaining
                        .next()
                        .filter(|v| !v.starts_with("--") && !v.trim().is_empty())
                        .ok_or(USAGE)?,
                );
            }
            _ => return Err(USAGE.to_owned()),
        }
    }
    let config = if config_stdin {
        let mut contents = String::new();
        io::stdin()
            .read_to_string(&mut contents)
            .map_err(|_| "could not read routing configuration from stdin")?;
        RoutingConfig::from_json(&contents).map_err(|error| error.to_string())?
    } else {
        RoutingConfig::default()
    };
    // Dispatch uses this same built-in resolver. The validated candidate resolver
    // delegates to it for every profile without an explicit configuration rule.
    let route = if config_stdin {
        config.resolve_profile(profile)
    } else {
        router_core::routing::resolve_profile(profile)
    }
    .ok_or("unknown routing profile")?;
    if model.is_some_and(|value| value != route.model)
        || effort.is_some_and(|value| value != route.effort)
    {
        return Err(
            "task model/effort overrides conflict with the resolved routing rule".to_owned(),
        );
    }
    let explicit = config
        .overrides()
        .iter()
        .any(|rule| rule.profile == *profile);
    write_json(&json!({
        "profile": route.profile,
        "model": route.model,
        "effort": route.effort,
        "reason": if explicit { "explicit" } else { "built-in" },
        "revision": config.revision()
    }))
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
        "readyTaskIds": persisted.ready_tasks().iter().map(|task| task.id.clone()).collect::<Vec<_>>()
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
    let mut prompt = String::new();
    io::stdin()
        .read_to_string(&mut prompt)
        .map_err(|error| format!("could not read continuation: {error}"))?;
    if prompt.trim().is_empty() {
        return write_block("No se puede continuar con un mensaje vacío.");
    }
    let thread_id = store
        .last_thread_id(run_id)
        .map_err(|error| format!("could not read thread: {error}"))?
        .ok_or_else(|| format!("el runId no tiene un hilo reanudable: {run_id}"))?;
    let mut client = AppServerClient::start(&AppServerConfig::local(&run.cwd))
        .map_err(|error| format!("no se pudo iniciar el runtime local: {error}"))?;
    client
        .resume_thread(&thread_id)
        .map_err(|error| format!("no se pudo reanudar el hilo local: {error}"))?;
    let turn = client
        .run_turn(
            &thread_id,
            prompt.trim(),
            &run.model,
            &run.effort,
            PathBuf::from(&run.cwd).as_path(),
            "never",
            "read-only",
        )
        .map_err(|error| format!("no se pudo completar la continuación: {error}"))?;
    store
        .record_event(&EventRecord {
            run_id: run.run_id.clone(),
            event_type: "turn.continued".to_owned(),
            payload: json!({ "threadId": thread_id, "turnId": turn.turn_id, "status": turn.status }),
        })
        .map_err(|error| format!("could not persist continuation event: {error}"))?;
    write_json(&json!({
        "runId": run.run_id,
        "routeId": run.route_id,
        "threadId": turn.thread_id,
        "status": turn.status,
        "text": turn.text
    }))
}

fn route_prompt(args: Vec<String>) -> Result<(), String> {
    // Only workers launched by the coordinator can continue the original turn.
    // Check before parsing stdin so worker hooks cannot recurse on bad input.
    if env::var_os("ROUDEX_ROUTER_INTERNAL").is_some()
        || env::var_os("ROUDEX_ACTIVE_TASK_ID").is_some()
    {
        return write_json(&json!({ "continue": true }));
    }
    // Retain the old flag as an ingress-only alias for already-installed hooks.
    if !args.is_empty() && args != ["--record-only"] {
        return Err("Usage: router-cli route-prompt [--record-only]".into());
    }
    let mut input_text = String::new();
    io::stdin()
        .read_to_string(&mut input_text)
        .map_err(|error| format!("could not read hook input: {error}"))?;
    let input: DesktopPromptInput = serde_json::from_str(&input_text)
        .map_err(|error| format!("invalid hook input: {error}"))?;
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    let receipt = store
        .ingest_desktop_prompt(&input)
        .map_err(|error| error.to_string())?;
    let task = &receipt.task.definition;
    write_json(&json!({
        "continue": false,
        "decision": "block",
        "reason": format!(
            "RouDex recorded task {} ({}). Review its execution contract and approval, then use task-run {} or task-dispatch {}. The Desktop turn is stopped; only the coordinator executes this task.",
            task.id, if receipt.replayed { "replayed delivery" } else { "new delivery" },
            task.id, receipt.task.spec_id
        ),
        "taskId": task.id,
        "specId": receipt.task.spec_id,
        "status": task.status,
        "replayed": receipt.replayed
    }))
}

fn install_hook() -> Result<(), String> {
    let path = hooks_path();
    let mut document = read_hooks(&path)?;
    let hook_dir = hooks_path()
        .parent()
        .ok_or("hook directory missing")?
        .join("roudex");
    fs::create_dir_all(&hook_dir).map_err(|error| error.to_string())?;
    let script = hook_dir.join("route-prompt.ps1");
    fs::write(&script, MANAGED_HOOK_SCRIPT).map_err(|error| error.to_string())?;
    let command = format!(
        "{} -NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\" -RouterPath \"{}\"",
        WINDOWS_POWERSHELL,
        script.display(),
        env::current_exe()
            .map_err(|error| error.to_string())?
            .display()
    );
    let hooks = document
        .as_object_mut()
        .ok_or_else(|| "hooks.json debe contener un objeto JSON".to_owned())?
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let event_hooks = hooks
        .as_object_mut()
        .ok_or_else(|| "la propiedad hooks debe ser un objeto".to_owned())?
        .entry("UserPromptSubmit")
        .or_insert_with(|| json!([]));
    let entries = event_hooks
        .as_array_mut()
        .ok_or_else(|| "UserPromptSubmit debe ser un arreglo".to_owned())?;
    entries.retain(|entry| {
        entry["hooks"]
            .as_array()
            .map(|handlers| {
                !handlers.iter().any(|handler| {
                    handler["command"]
                        .as_str()
                        .is_some_and(|value| value.contains("route-prompt"))
                })
            })
            .unwrap_or(true)
    });
    entries.push(json!({
        "hooks": [{
            "type": "command",
            "command": command,
            "commandWindows": command,
            "timeout": 600,
            "statusMessage": "Routing desktop message"
        }]
    }));
    write_hooks(&path, &document)
}

fn install_managed_hook() -> Result<(), String> {
    let requirements = managed_requirements_path();
    let managed_dir = managed_hook_dir();
    let executable = env::current_exe()
        .map_err(|error| format!("could not locate router executable: {error}"))?;
    let managed_executable = managed_dir.join("router-cli.exe");

    if requirements.exists() {
        let contents = fs::read_to_string(&requirements)
            .map_err(|error| format!("could not read managed requirements: {error}"))?;
        if !contents.contains(MANAGED_MARKER) {
            return Err(format!(
                "managed requirements already exist at {}; preserve that policy and add the RouDex hook manually",
                requirements.display()
            ));
        }
    }

    uninstall_hook()?;
    fs::create_dir_all(&managed_dir)
        .map_err(|error| format!("could not create managed hook directory: {error}"))?;
    fs::copy(&executable, &managed_executable)
        .map_err(|error| format!("could not install managed router executable: {error}"))?;

    let managed_script = managed_dir.join("route-prompt.ps1");
    fs::write(&managed_script, MANAGED_HOOK_SCRIPT)
        .map_err(|error| format!("could not install managed hook entrypoint: {error}"))?;
    let command = format!(
        "{} -NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\"",
        WINDOWS_POWERSHELL,
        managed_script.display()
    );
    let document = format!(
        "{MANAGED_MARKER}\n\n[features]\nhooks = true\n\n[hooks]\nwindows_managed_dir = '{}'\n\n[[hooks.UserPromptSubmit]]\n[[hooks.UserPromptSubmit.hooks]]\ntype = \"command\"\ncommand = '{}'\ncommand_windows = '{}'\ntimeout = 600\nstatusMessage = \"Routing desktop message\"\n",
        managed_dir.display(),
        command,
        command
    );
    if let Some(parent) = requirements.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create requirements directory: {error}"))?;
    }
    fs::write(&requirements, document)
        .map_err(|error| format!("could not write managed requirements: {error}"))?;
    eprintln!(
        "Managed hook installed at {} using {}",
        requirements.display(),
        managed_executable.display()
    );
    Ok(())
}

fn uninstall_hook() -> Result<(), String> {
    let path = hooks_path();
    if !path.exists() {
        return Ok(());
    }
    let mut document = read_hooks(&path)?;
    if let Some(entries) = document["hooks"]["UserPromptSubmit"].as_array_mut() {
        entries.retain(|entry| {
            !entry["hooks"]
                .as_array()
                .map(|handlers| {
                    handlers.iter().any(|handler| {
                        ["command", "commandWindows"].iter().any(|field| {
                            handler[*field]
                                .as_str()
                                .is_some_and(|value| value.contains("route-prompt"))
                        })
                    })
                })
                .unwrap_or(false)
        });
    }
    write_hooks(&path, &document)
}

fn uninstall_managed_hook() -> Result<(), String> {
    let requirements = managed_requirements_path();
    if requirements.exists() {
        let contents = fs::read_to_string(&requirements)
            .map_err(|error| format!("could not read managed requirements: {error}"))?;
        if !contents.contains(MANAGED_MARKER) {
            return Err(format!(
                "managed requirements at {} are not owned by RouDex",
                requirements.display()
            ));
        }
        fs::remove_file(&requirements)
            .map_err(|error| format!("could not remove managed requirements: {error}"))?;
    }
    let managed_executable = managed_hook_dir().join("router-cli.exe");
    if managed_executable.exists() {
        fs::remove_file(&managed_executable)
            .map_err(|error| format!("could not remove managed router executable: {error}"))?;
    }
    let managed_script = managed_hook_dir().join("route-prompt.ps1");
    if managed_script.exists() {
        fs::remove_file(&managed_script)
            .map_err(|error| format!("could not remove managed hook entrypoint: {error}"))?;
    }
    Ok(())
}

fn status() -> Result<(), String> {
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    write_json(&json!({
        "statePath": store.path(),
        "runCount": store.count_runs().map_err(|error| error.to_string())?,
        "hookPath": hooks_path()
    }))
}

fn write_block(reason: &str) -> Result<(), String> {
    write_json(&json!({ "continue": false, "decision": "block", "reason": reason }))
}

fn write_json(value: &Value) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string(value)
            .map_err(|error| format!("could not encode hook output: {error}"))?
    );
    Ok(())
}

fn hooks_path() -> PathBuf {
    env::var_os("CODEX_HOME")
        .or_else(|| {
            env::var_os("USERPROFILE")
                .map(|value| PathBuf::from(value).join(".codex").into_os_string())
        })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".codex"))
        .join("hooks.json")
}

const MANAGED_MARKER: &str = "# RouDex managed desktop routing hook";
const WINDOWS_POWERSHELL: &str = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";
const MANAGED_HOOK_SCRIPT: &str = include_str!("../../../scripts/route-prompt.ps1");

fn managed_requirements_path() -> PathBuf {
    if let Some(path) = env::var_os("ROUDEX_CODEX_REQUIREMENTS_PATH") {
        return PathBuf::from(path);
    }
    if cfg!(windows) {
        env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
            .join("OpenAI")
            .join("Codex")
            .join("requirements.toml")
    } else {
        PathBuf::from("/etc/codex/requirements.toml")
    }
}

fn managed_hook_dir() -> PathBuf {
    if let Some(path) = env::var_os("ROUDEX_MANAGED_HOOK_DIR") {
        return PathBuf::from(path);
    }
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".roudex"))
        .join("RouDex")
        .join("hooks")
}

fn read_hooks(path: &PathBuf) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({ "description": "Local desktop routing hooks", "hooks": {} }));
    }
    let contents =
        fs::read_to_string(path).map_err(|error| format!("could not read hooks file: {error}"))?;
    serde_json::from_str(&contents).map_err(|error| format!("invalid hooks file: {error}"))
}

fn write_hooks(path: &PathBuf, document: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create hooks directory: {error}"))?;
    }
    let contents = serde_json::to_string_pretty(document)
        .map_err(|error| format!("could not encode hooks file: {error}"))?;
    fs::write(path, format!("{contents}\n"))
        .map_err(|error| format!("could not write hooks file: {error}"))
}

fn print_usage() {
    eprintln!(
        "Usage: router-cli <route-preview <profile> [--config-stdin] [--model <model>] [--effort <effort>]|spec-load <manifest.json>|task-list <specId>|task-approve <taskId> [approved|denied]|task-dispatch <specId>|continue-run <runId>|task-plan <manifest>|task-run <taskId>|task-recover <taskId> [reason]|task-retry <taskId>|task-status <taskId>|run-events <runId>|route-prompt [--record-only]|install-hook|install-managed-hook|uninstall-hook|uninstall-managed-hook|status>"
    );
}
