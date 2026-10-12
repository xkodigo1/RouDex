use router_core::{
    AppServerClient, AppServerConfig, EventRecord, RoutePromptInput, RouteReceipt, RouteStatus,
    RouterStore,
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

    let command = format!("\"{}\" route-prompt", managed_executable.display());
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
    format!("\"{}\" route-prompt", executable.display())
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
        "Usage: router-cli <route-prompt [--record-only]|continue-run <runId>|install-hook|install-managed-hook|uninstall-hook|uninstall-managed-hook|status>"
    );
}
