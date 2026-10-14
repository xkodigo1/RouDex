//! Codex Desktop `UserPromptSubmit` hook: routing context from the fast rules,
//! a warning or an optional block for risky prompts, a local record of each
//! decision, and the entry RouDex keeps in Codex's `hooks.json`.

use crate::classifier::{describe, DecisionSource, RouteDecision, Router};
use crate::persistence::{now_string, RouterStore, StoreError};
use crate::risk::RiskRules;
use crate::tiers::{RouterConfig, Tier};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Marks the handlers RouDex owns in `hooks.json`.
pub const HOOK_STATUS_MESSAGE: &str = "RouDex: contexto de routing";
/// Days the summary covers.
pub const SUMMARY_DAYS: u32 = 30;

/// The fields RouDex reads from Codex's `UserPromptSubmit` input.
#[derive(Debug, Clone, Deserialize)]
pub struct PromptSubmit {
    pub session_id: String,
    #[serde(default)]
    pub turn_id: Option<String>,
    pub cwd: PathBuf,
    /// Model of the chat; Codex does not send its effort.
    #[serde(default)]
    pub model: Option<String>,
    pub prompt: String,
}

/// Field names and JSON types of a hook input, never its values, so a failure
/// can be diagnosed without storing the prompt.
pub fn input_shape(text: &str) -> String {
    let kind = |value: &Value| match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(fields)) => fields
            .iter()
            .map(|(name, value)| format!("{name}:{}", kind(value)))
            .collect::<Vec<_>>()
            .join(", "),
        Ok(other) => format!("JSON {}", kind(&other)),
        Err(_) => format!("no JSON ({} bytes)", text.len()),
    }
}

/// Where the hook keeps its binary and data: `$ROUDEX_HOME`, by default
/// `~/.roudex`. Codex Desktop is a packaged (MSIX) app and Windows redirects
/// what its child processes read and write under AppData to a private copy,
/// so nothing the hook needs may live under `%LOCALAPPDATA%` or `%APPDATA%`.
pub fn home() -> PathBuf {
    std::env::var_os("ROUDEX_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(|home| PathBuf::from(home).join(".roudex"))
        })
        .unwrap_or_else(|| PathBuf::from(".roudex"))
}

/// The copy of `router-cli` that Codex runs.
pub fn bin_path() -> PathBuf {
    let name = if cfg!(windows) {
        "router-cli.exe"
    } else {
        "router-cli"
    };
    home().join("bin").join(name)
}

/// Decisions recorded by the hook, read by the desktop app.
pub fn store_path() -> PathBuf {
    home().join("desktop-hook.sqlite3")
}

/// Failures Codex Desktop does not show.
pub fn log_path() -> PathBuf {
    home().join("hook.log")
}

/// Refuse locations that packaged apps see through a private copy.
pub fn check_outside_app_data(path: &Path) -> Result<(), String> {
    let normalize = |path: &Path| path.to_string_lossy().replace('/', "\\").to_lowercase();
    let target = normalize(path);
    for variable in ["LOCALAPPDATA", "APPDATA"] {
        if let Some(root) = std::env::var_os(variable).filter(|root| !root.is_empty()) {
            let root = normalize(Path::new(&root));
            if target.starts_with(&format!("{}\\", root.trim_end_matches('\\'))) {
                return Err(format!(
                    "{} está dentro de %{variable}%: Codex Desktop (app empaquetada) vería una \
                     copia privada de esa carpeta. Usa una ruta fuera de AppData (ROUDEX_HOME).",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

/// The managed `requirements.toml` that `v0.5.0` installed, if it is still
/// there. Removing it needs administrator rights, so RouDex only reports it.
pub fn legacy_managed_hook() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("ProgramData")?)
        .join("OpenAI")
        .join("Codex")
        .join("requirements.toml");
    std::fs::read_to_string(&path)
        .ok()
        .filter(|text| text.contains("route-prompt") || text.contains("RouDex"))
        .map(|_| path)
}

/// Append one line to the hook log, starting over past 64 KiB.
pub fn log_failure(path: &Path, message: &str) {
    use std::io::Write;
    const LIMIT: u64 = 64 * 1024;
    let restart = std::fs::metadata(path).is_ok_and(|meta| meta.len() > LIMIT);
    let _ = path.parent().map(std::fs::create_dir_all);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(!restart)
        .write(true)
        .truncate(restart)
        .open(path);
    if let Ok(mut file) = file {
        // One line per failure, even for multi-line parser errors.
        let message = message.split_whitespace().collect::<Vec<_>>().join(" ");
        let _ = writeln!(file, "{} {message}", now_string());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookAction {
    /// Only routing context for the model.
    Context,
    /// Risky prompt on a model other than the `fuerte` one: visible warning.
    Warn,
    /// Same, with `block_high_risk = true`: the prompt is not sent.
    Block,
}

impl HookAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Warn => "warn",
            Self::Block => "block",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HookOutcome {
    pub decision: RouteDecision,
    pub action: HookAction,
    /// What the hook prints on stdout for Codex.
    pub response: Value,
}

/// Decide with risk and fast rules only and build Codex's hook response.
pub fn evaluate(input: &PromptSubmit, config: &RouterConfig, rules: &RiskRules) -> HookOutcome {
    let decision = Router::new(config, rules).decide_by_rules(&input.prompt);
    let strong = config.target(Tier::Fuerte);
    let below_strong = input.model.as_deref() != Some(strong.model.as_str());
    let action = match (decision.source, below_strong) {
        (DecisionSource::Risk, true) if config.desktop_hook.block_high_risk => HookAction::Block,
        (DecisionSource::Risk, true) => HookAction::Warn,
        _ => HookAction::Context,
    };
    let chat_model = input.model.as_deref().unwrap_or("un modelo desconocido");
    let warning = format!(
        "RouDex: mensaje de riesgo ({}). Se recomienda el tier fuerte ({} {}); este chat usa {chat_model}.",
        describe(&decision.risk),
        strong.model,
        strong.effort
    );
    let mut additional = context(&decision, config);
    if action == HookAction::Warn {
        // Codex Desktop does not show a hook's systemMessage, so the model
        // repeats the warning where the user reads it.
        additional.push_str(&format!(
            " Empieza tu respuesta con esta línea, tal cual, y luego atiende el mensaje con normalidad: «⚠ {warning}»"
        ));
    }
    let context = json!({
        "hookEventName": "UserPromptSubmit",
        "additionalContext": additional,
    });
    let response = match action {
        HookAction::Context => json!({ "hookSpecificOutput": context }),
        HookAction::Warn => json!({
            "systemMessage": warning,
            "hookSpecificOutput": context,
        }),
        HookAction::Block => json!({
            "decision": "block",
            "reason": format!(
                "RouDex bloqueó este mensaje por riesgo ({}). Cambia el chat a {} con esfuerzo {} y vuelve a enviarlo; este chat usa {chat_model}. Para solo avisar, pon block_high_risk = false en router.toml.",
                describe(&decision.risk), strong.model, strong.effort
            ),
        }),
    };
    HookOutcome {
        decision,
        action,
        response,
    }
}

fn context(decision: &RouteDecision, config: &RouterConfig) -> String {
    match decision.source {
        DecisionSource::Fallback => format!(
            "RouDex: ninguna regla de riesgo ni regla rápida aplica a este mensaje; sin recomendación de tier (por defecto: {}).",
            config.fallback_tier
        ),
        _ => format!(
            "RouDex recomienda el tier {} ({} {}) para este mensaje ({}).",
            decision.tier, decision.model, decision.effort, decision.reason
        ),
    }
}

pub(crate) fn migrate(connection: &Connection) -> rusqlite::Result<()> {
    // One row per Codex turn; the prompt text is never stored.
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS desktop_prompts (
             session_id TEXT NOT NULL,
             turn_id TEXT NOT NULL,
             cwd TEXT NOT NULL,
             chat_model TEXT,
             tier TEXT NOT NULL,
             model TEXT NOT NULL,
             effort TEXT NOT NULL,
             source TEXT NOT NULL,
             risk TEXT NOT NULL,
             action TEXT NOT NULL,
             created_at TEXT NOT NULL,
             PRIMARY KEY (session_id, turn_id)
         );",
    )
}

/// Codex Desktop decisions of the last [`SUMMARY_DAYS`] days.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DesktopSummary {
    pub days: u32,
    pub prompts: i64,
    pub today: i64,
    pub economico: i64,
    pub estandar: i64,
    pub fuerte: i64,
    /// Prompts no rule settled.
    pub unruled: i64,
    pub warned: i64,
    pub blocked: i64,
    /// Prompts whose chat model was the recommended one.
    pub on_recommended_model: i64,
    pub last_at: Option<String>,
}

impl RouterStore {
    /// Store the decision for a Codex turn; a repeated turn replaces its row.
    pub fn record_desktop_prompt(
        &self,
        input: &PromptSubmit,
        outcome: &HookOutcome,
    ) -> Result<(), StoreError> {
        let decision = &outcome.decision;
        let source = serde_json::to_value(decision.source)?;
        self.connection.execute(
            "INSERT INTO desktop_prompts (session_id, turn_id, cwd, chat_model, tier, model,
                 effort, source, risk, action, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(session_id, turn_id) DO UPDATE SET cwd = excluded.cwd,
                 chat_model = excluded.chat_model, tier = excluded.tier, model = excluded.model,
                 effort = excluded.effort, source = excluded.source, risk = excluded.risk,
                 action = excluded.action",
            params![
                input.session_id,
                input.turn_id.as_deref().unwrap_or(""),
                input.cwd.to_string_lossy(),
                input.model,
                decision.tier.as_str(),
                decision.model,
                decision.effort,
                source.as_str().unwrap_or_default(),
                serde_json::to_string(&decision.risk)?,
                outcome.action.as_str(),
                now_string(),
            ],
        )?;
        Ok(())
    }

    pub fn desktop_summary(&self) -> Result<DesktopSummary, StoreError> {
        const DAY: &str = "date(CAST(created_at AS INTEGER), 'unixepoch', 'localtime')";
        let summary = self.connection.query_row(
            &format!(
                "SELECT count(*),
                     coalesce(sum({DAY} = date('now', 'localtime')), 0),
                     coalesce(sum(tier = 'economico'), 0),
                     coalesce(sum(tier = 'estandar'), 0),
                     coalesce(sum(tier = 'fuerte'), 0),
                     coalesce(sum(source = 'fallback'), 0),
                     coalesce(sum(action = 'warn'), 0),
                     coalesce(sum(action = 'block'), 0),
                     coalesce(sum(chat_model = model), 0),
                     max(CAST(created_at AS INTEGER))
                 FROM desktop_prompts
                 WHERE {DAY} > date('now', 'localtime', ?1)"
            ),
            [format!("-{SUMMARY_DAYS} days")],
            |row| {
                Ok(DesktopSummary {
                    days: SUMMARY_DAYS,
                    prompts: row.get(0)?,
                    today: row.get(1)?,
                    economico: row.get(2)?,
                    estandar: row.get(3)?,
                    fuerte: row.get(4)?,
                    unruled: row.get(5)?,
                    warned: row.get(6)?,
                    blocked: row.get(7)?,
                    on_recommended_model: row.get(8)?,
                    last_at: row.get::<_, Option<i64>>(9)?.map(|at| at.to_string()),
                })
            },
        )?;
        Ok(summary)
    }
}

/// `$CODEX_HOME/hooks.json`, by default `~/.codex/hooks.json`.
pub fn hooks_path() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(|home| PathBuf::from(home).join(".codex"))
        })
        .unwrap_or_else(|| PathBuf::from(".codex"))
        .join("hooks.json")
}

/// The command Codex runs: `<router-cli> hook`. Codex does not document which
/// shell runs it, so the path goes unquoted with forward slashes, which cmd,
/// PowerShell and sh all read the same way; paths that would need quoting are
/// refused.
pub fn hook_command(router_cli: &Path) -> Result<String, String> {
    let path = router_cli.to_string_lossy().replace('\\', "/");
    let path = path.strip_prefix("//?/").unwrap_or(&path);
    let safe = |c: char| c.is_ascii_alphanumeric() || "/:._-+~".contains(c);
    if !path.chars().all(safe) {
        return Err(format!(
            "la ruta de router-cli ({path}) tiene espacios o caracteres especiales; \
             cópialo a una carpeta sin ellos y vuelve a ejecutar hook-install"
        ));
    }
    Ok(format!("{path} hook"))
}

/// What an install or uninstall changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct HookChange {
    /// RouDex handlers removed, including the earlier one.
    pub removed: usize,
    /// Handlers of the `route-prompt` hook from `v0.3.x`–`v0.5.0` among them.
    pub removed_legacy: usize,
}

/// Replace every RouDex `UserPromptSubmit` handler with one that runs
/// `command`. Other hooks are kept as they are.
pub fn install_into(document: &mut Value, command: &str) -> Result<HookChange, String> {
    let change = uninstall_from(document)?;
    let root = document
        .as_object_mut()
        .ok_or("hooks.json debe contener un objeto JSON")?;
    let events = root
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("la propiedad hooks debe ser un objeto")?;
    events
        .entry("UserPromptSubmit")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or("hooks.UserPromptSubmit debe ser un arreglo")?
        .push(json!({
            "hooks": [{
                "type": "command",
                "command": command,
                "timeout": 30,
                "statusMessage": HOOK_STATUS_MESSAGE
            }]
        }));
    Ok(change)
}

/// Remove RouDex handlers (current and legacy) and any group they leave empty.
pub fn uninstall_from(document: &mut Value) -> Result<HookChange, String> {
    let mut change = HookChange::default();
    let Some(groups) = document
        .get_mut("hooks")
        .and_then(|hooks| hooks.get_mut("UserPromptSubmit"))
    else {
        return Ok(change);
    };
    let groups = groups
        .as_array_mut()
        .ok_or("hooks.UserPromptSubmit debe ser un arreglo")?;
    for group in groups.iter_mut() {
        if let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) {
            handlers.retain(|handler| {
                let legacy = is_legacy(handler);
                let ours = legacy || is_ours(handler);
                change.removed += usize::from(ours);
                change.removed_legacy += usize::from(legacy);
                !ours
            });
        }
    }
    groups.retain(|group| {
        group
            .get("hooks")
            .and_then(Value::as_array)
            .is_none_or(|handlers| !handlers.is_empty())
    });
    Ok(change)
}

/// The command of the installed RouDex handler, if any.
pub fn installed_command(document: &Value) -> Option<String> {
    document["hooks"]["UserPromptSubmit"]
        .as_array()?
        .iter()
        .filter_map(|group| group["hooks"].as_array())
        .flatten()
        .find(|handler| is_ours(handler))
        .and_then(|handler| handler["command"].as_str().map(str::to_owned))
}

fn is_ours(handler: &Value) -> bool {
    handler["statusMessage"].as_str() == Some(HOOK_STATUS_MESSAGE)
}

fn is_legacy(handler: &Value) -> bool {
    ["command", "commandWindows"].iter().any(|field| {
        handler[*field]
            .as_str()
            .is_some_and(|value| value.contains("route-prompt"))
    })
}

/// Read `hooks.json`; a missing file is an empty document.
pub fn read_hooks(path: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(json!({})),
        Ok(text) => serde_json::from_str(&text)
            .map_err(|error| format!("{} no es JSON válido: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(error) => Err(format!("no se pudo leer {}: {error}", path.display())),
    }
}

pub fn write_hooks(path: &Path, document: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(document).map_err(|error| error.to_string())?;
    std::fs::write(path, text + "\n")
        .map_err(|error| format!("no se pudo escribir {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RiskMatch;

    fn input(prompt: &str, model: Option<&str>) -> PromptSubmit {
        PromptSubmit {
            session_id: "s1".into(),
            turn_id: Some("t1".into()),
            cwd: PathBuf::from("C:/p/api"),
            model: model.map(str::to_owned),
            prompt: prompt.into(),
        }
    }

    fn blocking() -> RouterConfig {
        let mut config = RouterConfig::default();
        config.desktop_hook.block_high_risk = true;
        config
    }

    const RISKY: &str = "aplica la migración de la base de datos en producción";

    #[test]
    fn risky_prompt_on_a_lower_model_warns_without_blocking() {
        let outcome = evaluate(
            &input(RISKY, Some("gpt-6-luna")),
            &RouterConfig::default(),
            &RiskRules::default(),
        );
        assert_eq!(outcome.action, HookAction::Warn);
        assert_eq!(outcome.decision.tier, Tier::Fuerte);
        let message = outcome.response["systemMessage"].as_str().unwrap();
        assert!(message.contains("gpt-6.1-sol high"), "{message}");
        assert!(message.contains("este chat usa gpt-6-luna"), "{message}");
        assert!(outcome.response.get("decision").is_none());
        let context = outcome.response["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap();
        assert!(context.contains("tier fuerte"), "{context}");
        // Codex Desktop hides systemMessage: the model must repeat the warning.
        assert!(context.contains(&format!("«⚠ {message}»")), "{context}");
        assert_eq!(
            outcome.response["hookSpecificOutput"]["hookEventName"],
            "UserPromptSubmit"
        );
    }

    #[test]
    fn blocking_option_blocks_risky_prompts_on_a_lower_model() {
        let outcome = evaluate(
            &input(RISKY, Some("gpt-6-luna")),
            &blocking(),
            &RiskRules::default(),
        );
        assert_eq!(outcome.action, HookAction::Block);
        assert_eq!(outcome.response["decision"], "block");
        let reason = outcome.response["reason"].as_str().unwrap();
        assert!(reason.contains("gpt-6.1-sol con esfuerzo high"), "{reason}");
        // An unknown chat model cannot prove it is the strong one.
        let unknown = evaluate(&input(RISKY, None), &blocking(), &RiskRules::default());
        assert_eq!(unknown.action, HookAction::Block);
    }

    #[test]
    fn risky_prompt_on_the_strong_model_only_gets_context() {
        let outcome = evaluate(
            &input(RISKY, Some("gpt-6.1-sol")),
            &blocking(),
            &RiskRules::default(),
        );
        assert_eq!(outcome.action, HookAction::Context);
        assert!(outcome.response.get("systemMessage").is_none());
        assert!(outcome.response.get("decision").is_none());
    }

    #[test]
    fn fast_rules_recommend_the_economy_tier_quietly() {
        let outcome = evaluate(
            &input("explica qué hace este módulo", Some("gpt-6.1-sol")),
            &blocking(),
            &RiskRules::default(),
        );
        assert_eq!(outcome.action, HookAction::Context);
        assert_eq!(outcome.decision.tier, Tier::Economico);
        assert_eq!(outcome.decision.source, DecisionSource::Rule);
        assert!(outcome.response.get("systemMessage").is_none());
        let context = outcome.response["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap();
        assert!(
            context.contains("tier economico (gpt-6-luna high)"),
            "{context}"
        );
        assert!(
            !context.contains('⚠'),
            "only warnings ask for a visible line"
        );
    }

    #[test]
    fn prompts_no_rule_settles_get_no_recommendation() {
        let outcome = evaluate(
            &input(
                "agrega paginación por cursor al listado de hilos y su test",
                Some("gpt-6-luna"),
            ),
            &blocking(),
            &RiskRules::default(),
        );
        assert_eq!(outcome.action, HookAction::Context);
        assert_eq!(outcome.decision.source, DecisionSource::Fallback);
        assert_eq!(outcome.decision.tier, Tier::Estandar);
        assert_eq!(
            outcome.decision.reason,
            "ninguna regla aplica; sin clasificador"
        );
        let context = outcome.response["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap();
        assert!(context.contains("sin recomendación"), "{context}");
        assert!(context.contains("estandar"), "{context}");
    }

    #[test]
    fn project_rules_also_trigger_the_warning() {
        let rules = RiskRules::from_project_toml("keywords = [\"ledger\"]").unwrap();
        let outcome = evaluate(
            &input("ajusta el ledger", Some("gpt-6-luna")),
            &RouterConfig::default(),
            &rules,
        );
        assert_eq!(outcome.action, HookAction::Warn);
        assert_eq!(
            outcome.decision.risk,
            vec![RiskMatch::Keyword("ledger".into())]
        );
    }

    #[test]
    fn a_repeated_turn_updates_one_row_and_the_summary_counts_it_once() {
        let store = RouterStore::in_memory().unwrap();
        let config = RouterConfig::default();
        let rules = RiskRules::default();
        let first = input(RISKY, Some("gpt-6-luna"));
        store
            .record_desktop_prompt(&first, &evaluate(&first, &config, &rules))
            .unwrap();
        let again = input(RISKY, Some("gpt-6.1-sol"));
        store
            .record_desktop_prompt(&again, &evaluate(&again, &config, &rules))
            .unwrap();
        let mut other = input("explica el README", Some("gpt-6-luna"));
        other.turn_id = Some("t2".into());
        store
            .record_desktop_prompt(&other, &evaluate(&other, &config, &rules))
            .unwrap();
        let mut blocked = input(RISKY, Some("gpt-6-luna"));
        blocked.turn_id = Some("t3".into());
        store
            .record_desktop_prompt(&blocked, &evaluate(&blocked, &blocking(), &rules))
            .unwrap();

        let summary = store.desktop_summary().unwrap();
        assert_eq!(
            (summary.prompts, summary.today, summary.days),
            (3, 3, SUMMARY_DAYS)
        );
        assert_eq!(
            (summary.economico, summary.estandar, summary.fuerte),
            (1, 0, 2)
        );
        assert_eq!(
            (summary.warned, summary.blocked, summary.unruled),
            (0, 1, 0)
        );
        // t1 now runs on the strong model and t2 on the economy one.
        assert_eq!(summary.on_recommended_model, 2);
        assert!(summary.last_at.is_some());
        let stored: String = store
            .connection
            .query_row(
                "SELECT group_concat(cwd || chat_model || risk) FROM desktop_prompts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!stored.contains("migración"), "the prompt is never stored");
    }

    #[test]
    fn an_empty_store_has_an_empty_summary() {
        let summary = RouterStore::in_memory().unwrap().desktop_summary().unwrap();
        assert_eq!(summary.prompts, 0);
        assert_eq!(summary.last_at, None);
    }

    fn legacy_and_other_hooks() -> Value {
        json!({
            "hooks": {
                "UserPromptSubmit": [
                    { "hooks": [{ "type": "command", "command": "powershell -File C:/x/route-prompt.ps1",
                        "statusMessage": "Routing desktop message" }] },
                    { "hooks": [{ "type": "command", "command": "notify.exe" }] }
                ],
                "Stop": [{ "hooks": [{ "type": "command", "command": "beep.exe" }] }]
            }
        })
    }

    #[test]
    fn install_is_idempotent_and_retires_the_legacy_hook() {
        let mut document = legacy_and_other_hooks();
        let first = install_into(&mut document, "\"C:/bin/router-cli.exe\" hook").unwrap();
        assert_eq!(
            first,
            HookChange {
                removed: 1,
                removed_legacy: 1
            }
        );
        let second = install_into(&mut document, "\"C:/bin/router-cli.exe\" hook").unwrap();
        assert_eq!(
            second,
            HookChange {
                removed: 1,
                removed_legacy: 0
            }
        );
        let groups = document["hooks"]["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0]["hooks"][0]["command"], "notify.exe");
        assert_eq!(
            installed_command(&document).as_deref(),
            Some("\"C:/bin/router-cli.exe\" hook")
        );
        assert_eq!(
            document["hooks"]["Stop"][0]["hooks"][0]["command"],
            "beep.exe"
        );
    }

    #[test]
    fn uninstall_keeps_every_other_hook() {
        let mut document = legacy_and_other_hooks();
        install_into(&mut document, "router-cli hook").unwrap();
        let change = uninstall_from(&mut document).unwrap();
        assert_eq!(change.removed, 1);
        assert_eq!(installed_command(&document), None);
        let groups = document["hooks"]["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["hooks"][0]["command"], "notify.exe");
        assert_eq!(
            uninstall_from(&mut json!({})).unwrap(),
            HookChange::default()
        );
    }

    #[test]
    fn input_shape_names_fields_without_values() {
        let shape = input_shape(r#"{"session_id":"s","prompt":"secreto","input":[1],"n":2}"#);
        assert_eq!(
            shape,
            "input:array, n:number, prompt:string, session_id:string"
        );
        assert!(!shape.contains("secreto"));
        assert_eq!(input_shape("[]"), "JSON array");
        assert_eq!(input_shape("hola"), "no JSON (4 bytes)");
    }

    #[test]
    fn the_failure_log_appends_and_starts_over_when_large() {
        let path = std::env::temp_dir().join(format!("roudex-hook-{}.log", uuid::Uuid::new_v4()));
        log_failure(&path, "uno");
        log_failure(&path, "dos");
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.lines().last().unwrap().ends_with(" dos"));
        std::fs::write(&path, "x".repeat(70 * 1024)).unwrap();
        log_failure(&path, "tres");
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn app_data_locations_are_refused() {
        // Reads the real variables; skip where they are not set.
        let Some(local) = std::env::var_os("LOCALAPPDATA") else {
            return;
        };
        let inside = PathBuf::from(&local)
            .join("RouDex")
            .join("bin")
            .join("router-cli.exe");
        let error = check_outside_app_data(&inside).unwrap_err();
        assert!(error.contains("LOCALAPPDATA"), "{error}");
        let lower =
            PathBuf::from(local.to_string_lossy().to_lowercase().replace('\\', "/")).join("x.exe");
        assert!(check_outside_app_data(&lower).is_err());
        let sibling = PathBuf::from(format!("{}Other", local.to_string_lossy())).join("x.exe");
        assert!(check_outside_app_data(&sibling).is_ok());
        assert!(
            check_outside_app_data(Path::new(r"C:\Users\dev\.roudex\bin\router-cli.exe")).is_ok()
        );
    }

    #[test]
    fn hook_commands_need_no_quoting_in_any_shell() {
        assert_eq!(
            hook_command(Path::new(
                r"C:\Users\dev\RouDex\target\release\router-cli.exe"
            ))
            .unwrap(),
            "C:/Users/dev/RouDex/target/release/router-cli.exe hook"
        );
        assert_eq!(
            hook_command(Path::new(r"\\?\C:\tools\router-cli.exe")).unwrap(),
            "C:/tools/router-cli.exe hook"
        );
        assert!(hook_command(Path::new(r"C:\Program Files\RouDex\router-cli.exe")).is_err());
        assert!(hook_command(Path::new(r"C:\a&b\router-cli.exe")).is_err());
    }

    #[test]
    fn install_rejects_documents_it_cannot_edit_safely() {
        assert!(install_into(&mut json!([]), "x").is_err());
        assert!(install_into(&mut json!({ "hooks": [] }), "x").is_err());
        assert!(install_into(&mut json!({ "hooks": { "UserPromptSubmit": {} } }), "x").is_err());
    }
}
