use router_core::{
    AppServerClient, AppServerConfig, EventRecord, LocalTaskRuntime, RoutePromptInput,
    RouteReceipt, RouteStatus, RouterStore, RuntimeTurn, TaskApproval, TaskCoordinator,
    TaskDefinition, TaskManifest, TaskRuntime, TaskStatus,
};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use uuid::Uuid;

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
    match args.next().as_deref() {
        Some("route-prompt") => route_prompt(args.collect()),
        Some("continue-run") => continue_run(args.collect()),
        Some("task-plan") => task_plan(args.collect()),
        Some("task-approve") => task_approve(args.collect()),
        Some("task-run") => task_run(args.collect()),
        Some("task-recover") => task_recover(args.collect()),
        Some("task-retry") => task_retry(args.collect()),
        Some("task-status") => task_status(args.collect()),
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
    write_json(&json!({
        "specId": manifest.spec_id,
        "taskCount": manifest.tasks.len(),
        "readyTaskIds": manifest.ready_tasks().iter().map(|task| task.id.clone()).collect::<Vec<_>>()
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
    let task = store
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
    if task.definition.status == TaskStatus::AwaitingApproval {
        let next = if decision == "approved" {
            TaskStatus::Ready
        } else {
            TaskStatus::Blocked
        };
        store
            .set_task_status(task_id, next)
            .map_err(|error| error.to_string())?;
    }
    write_json(&json!({ "taskId": task_id, "decision": decision }))
}

fn task_run(args: Vec<String>) -> Result<(), String> {
    let task_id = args
        .first()
        .ok_or_else(|| "task-run requiere un taskId".to_owned())?;
    if env::var_os("ROUDEX_ROUTER_INTERNAL").is_some()
        || env::var_os("ROUDEX_ACTIVE_TASK_ID").is_some()
    {
        return Err("task-run no puede invocarse desde un worker activo".to_owned());
    }
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

fn continue_run(args: Vec<String>) -> Result<(), String> {
    let run_id = args
        .first()
        .ok_or_else(|| "continue-run requiere un runId".to_owned())?;
    let mut prompt = String::new();
    io::stdin()
        .read_to_string(&mut prompt)
        .map_err(|error| format!("could not read continuation: {error}"))?;
    if prompt.trim().is_empty() {
        return write_block("No se puede continuar con un mensaje vacío.");
    }

    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    let run = store
        .get_run(run_id)
        .map_err(|error| format!("could not read run: {error}"))?
        .ok_or_else(|| format!("runId no encontrado: {run_id}"))?;
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
    let record_only = args.iter().any(|arg| arg == "--record-only");
    let mut input_text = String::new();
    io::stdin()
        .read_to_string(&mut input_text)
        .map_err(|error| format!("could not read hook input: {error}"))?;
    let input: RoutePromptInput = serde_json::from_str(&input_text)
        .map_err(|error| format!("invalid hook input: {error}"))?;

    if env::var_os("ROUDEX_ROUTER_INTERNAL").is_some() {
        return write_json(&json!({ "continue": true }));
    }
    if input.prompt.trim().is_empty() {
        return write_block("No se puede enrutar un mensaje vacío.");
    }

    let route = router_core::route_prompt(&input);
    let receipt = RouteReceipt {
        route_id: Uuid::new_v4().to_string(),
        run_id: Uuid::new_v4().to_string(),
        profile: route.profile.to_owned(),
        model: route.model.to_owned(),
        effort: route.effort.to_owned(),
        status: RouteStatus::Accepted,
    };
    let store = RouterStore::open(RouterStore::default_path())
        .map_err(|error| format!("could not open local state: {error}"))?;
    store
        .record_route(&input, &receipt)
        .map_err(|error| format!("could not persist route: {error}"))?;
    store
        .record_event(&EventRecord {
            run_id: receipt.run_id.clone(),
            event_type: "route.accepted".to_owned(),
            payload: json!({
                "sessionId": input.session_id,
                "turnId": input.turn_id,
                "cwd": input.cwd,
                "profile": receipt.profile,
                "model": receipt.model,
                "effort": receipt.effort
            }),
        })
        .map_err(|error| format!("could not persist route event: {error}"))?;

    let worker_context = if record_only {
        "Modo de registro activo; el turno continúa con el recibo local.".to_owned()
    } else {
        let config = AppServerConfig::local(&input.cwd);
        let mut client = AppServerClient::start(&config)
            .map_err(|error| format!("no se pudo iniciar el runtime local: {error}"))?;
        let thread_id = client
            .start_thread(
                receipt.model.as_str(),
                PathBuf::from(&input.cwd).as_path(),
                "never",
                "read-only",
            )
            .map_err(|error| format!("no se pudo iniciar el hilo local: {error}"))?;
        store
            .record_event(&EventRecord {
                run_id: receipt.run_id.clone(),
                event_type: "thread.started".to_owned(),
                payload: json!({ "threadId": thread_id }),
            })
            .map_err(|error| format!("could not persist thread event: {error}"))?;
        let turn = client
            .run_turn(
                &thread_id,
                &input.prompt,
                receipt.model.as_str(),
                receipt.effort.as_str(),
                PathBuf::from(&input.cwd).as_path(),
                "never",
                "read-only",
            )
            .map_err(|error| format!("no se pudo completar el turno local: {error}"))?;
        store
            .record_event(&EventRecord {
                run_id: receipt.run_id.clone(),
                event_type: "turn.completed".to_owned(),
                payload: json!({
                    "threadId": turn.thread_id,
                    "turnId": turn.turn_id,
                    "status": turn.status
                }),
            })
            .map_err(|error| format!("could not persist completion event: {error}"))?;
        format!("Respuesta del runtime local:\n{}", turn.text)
    };

    write_json(&json!({
        "continue": true,
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": format!(
                "Mensaje recibido por el adaptador local. routeId={} runId={} profile={} model={} effort={}\\n{}",
                receipt.route_id,
                receipt.run_id,
                receipt.profile,
                receipt.model,
                receipt.effort,
                worker_context
            )
        }
    }))
}

fn install_hook() -> Result<(), String> {
    let path = hooks_path();
    let mut document = read_hooks(&path)?;
    let command = hook_command();
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
const MANAGED_HOOK_SCRIPT: &str = r#"$router = Join-Path $PSScriptRoot 'router-cli.exe'
$inputText = [Console]::In.ReadToEnd()
$inputText | & $router route-prompt --record-only
exit $LASTEXITCODE
"#;

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

fn hook_command() -> String {
    let executable = env::current_exe().unwrap_or_else(|_| PathBuf::from("router-cli"));
    format!("\"{}\" route-prompt --record-only", executable.display())
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
        "Usage: router-cli <route-prompt [--record-only]|continue-run <runId>|task-plan <manifest>|task-approve <taskId> [approved|denied]|task-run <taskId>|task-recover <taskId> [reason]|task-retry <taskId>|task-status <taskId>|install-hook|install-managed-hook|uninstall-hook|uninstall-managed-hook|status>"
    );
}
