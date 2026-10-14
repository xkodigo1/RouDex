use crate::app_server::{AppServerClient, AppServerError};
use crate::risk::{words, RiskMatch, RiskRules};
use crate::tiers::{ModelTarget, RouterConfig, Tier};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Where a routing decision came from, so the UI can explain it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionSource {
    /// A fixed risk rule matched; the classifier cannot lower it.
    Risk,
    /// A fast local rule settled an obvious case without a model call.
    Rule,
    /// The model classifier chose the tier.
    Model,
    /// The classifier failed; the configured fallback tier applies.
    Fallback,
    /// The user escalated or overrode the decision.
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDecision {
    pub tier: Tier,
    pub model: String,
    pub effort: String,
    pub source: DecisionSource,
    pub reason: String,
    pub risk: Vec<RiskMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Classification {
    pub tier: Tier,
    pub reason: String,
}

/// A model that classifies a prompt into a tier.
pub trait ModelClassifier {
    fn classify(&mut self, prompt: &str, target: &ModelTarget) -> Result<Classification, String>;
}

/// Intents that are cheap regardless of topic, for short prompts only.
const LIGHT_INTENTS: &[&str] = &[
    "explica",
    "explicame",
    "que es",
    "que significa",
    "resume",
    "resumen",
    "resumir",
    "traduce",
    "traducir",
    "ortografia",
    "typo",
    "renombra",
    "formatea",
    "explain",
    "what is",
    "summarize",
    "translate",
    "rename",
];
const LIGHT_MAX_WORDS: usize = 30;

pub struct Router<'a> {
    config: &'a RouterConfig,
    rules: &'a RiskRules,
}

impl<'a> Router<'a> {
    pub fn new(config: &'a RouterConfig, rules: &'a RiskRules) -> Self {
        Self { config, rules }
    }

    /// Risk rules first, then fast rules, then the model classifier.
    pub fn decide(&self, prompt: &str, classifier: &mut dyn ModelClassifier) -> RouteDecision {
        let risk = self.rules.matches(prompt);
        if !risk.is_empty() {
            let reason = format!("regla de riesgo: {}", describe(&risk));
            return self.decision(Tier::Fuerte, DecisionSource::Risk, reason, risk);
        }
        if let Some((tier, reason)) = fast_rule(prompt) {
            return self.decision(tier, DecisionSource::Rule, reason, risk);
        }
        match classifier.classify(prompt, &self.config.classifier) {
            Ok(result) => self.decision(result.tier, DecisionSource::Model, result.reason, risk),
            Err(error) => self.decision(
                self.config.fallback_tier,
                DecisionSource::Fallback,
                format!("clasificador no disponible: {error}"),
                risk,
            ),
        }
    }

    /// Manual escalation: always the strongest tier, never automatic.
    pub fn escalate(&self, previous: &RouteDecision) -> RouteDecision {
        self.decision(
            Tier::Fuerte,
            DecisionSource::Manual,
            format!("escalado manual desde {}", previous.tier),
            previous.risk.clone(),
        )
    }

    fn decision(
        &self,
        tier: Tier,
        source: DecisionSource,
        reason: String,
        risk: Vec<RiskMatch>,
    ) -> RouteDecision {
        let target = self.config.target(tier);
        RouteDecision {
            tier,
            model: target.model.clone(),
            effort: target.effort.clone(),
            source,
            reason,
            risk,
        }
    }
}

fn describe(risk: &[RiskMatch]) -> String {
    risk.iter()
        .map(|item| match item {
            RiskMatch::Keyword(keyword) => keyword.clone(),
            RiskMatch::Path(path) => format!("ruta {path}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Settle obvious cases locally. `None` means the prompt needs the classifier.
pub fn fast_rule(prompt: &str) -> Option<(Tier, String)> {
    let tokens = words(prompt);
    if tokens.is_empty() {
        return Some((Tier::Economico, "mensaje vacío".into()));
    }
    if tokens.len() > LIGHT_MAX_WORDS || prompt.contains("```") {
        return None;
    }
    LIGHT_INTENTS
        .iter()
        .map(|intent| words(intent))
        .find(|intent| tokens.windows(intent.len()).any(|w| w == intent.as_slice()))
        .map(|intent| {
            (
                Tier::Economico,
                format!("mensaje corto de tipo «{}»", intent.join(" ")),
            )
        })
}

pub const CLASSIFIER_INSTRUCTIONS: &str = "Eres el clasificador de RouDex. Decide qué tier de modelo necesita la tarea del usuario. No ejecutes herramientas ni leas archivos: responde solo con JSON.
- economico: preguntas, explicaciones, texto, formato, cambios pequeños y acotados, tareas mecánicas.
- estandar: implementación o depuración habitual en código existente, cambios en varios archivos, integraciones con contratos conocidos.
- fuerte: arquitectura o diseño de sistemas, incidentes y causa raíz, seguridad, datos o dinero, refactors amplios, o cualquier tarea donde un error sea caro.
Si dudas entre dos tiers, elige el más alto.
Responde exactamente: {\"tier\": \"economico|estandar|fuerte\", \"reason\": \"<una frase en español>\"}";

pub fn classification_input(prompt: &str) -> String {
    format!("Tarea a clasificar:\n<<<\n{prompt}\n>>>")
}

pub fn classification_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "tier": { "type": "string", "enum": ["economico", "estandar", "fuerte"] },
            "reason": { "type": "string" }
        },
        "required": ["tier", "reason"],
        "additionalProperties": false
    })
}

/// Accept the JSON object even when the model wraps it in prose or fences.
pub fn parse_classification(text: &str) -> Result<Classification, String> {
    let start = text.find('{').ok_or("la respuesta no contiene JSON")?;
    let end = text.rfind('}').ok_or("la respuesta no contiene JSON")?;
    if end < start {
        return Err("la respuesta no contiene JSON".into());
    }
    let mut parsed: Classification = serde_json::from_str(&text[start..=end])
        .map_err(|error| format!("JSON de clasificación inválido: {error}"))?;
    parsed.reason = parsed.reason.trim().to_owned();
    if parsed.reason.is_empty() {
        parsed.reason = "sin motivo".into();
    }
    Ok(parsed)
}

/// Classifier backed by `codex app-server`: an ephemeral, read-only thread
/// that never asks for approvals.
pub struct AppServerClassifier {
    client: AppServerClient,
    cwd: PathBuf,
}

impl AppServerClassifier {
    pub fn new(client: AppServerClient, cwd: impl AsRef<Path>) -> Self {
        Self {
            client,
            cwd: cwd.as_ref().to_path_buf(),
        }
    }
}

impl ModelClassifier for AppServerClassifier {
    fn classify(&mut self, prompt: &str, target: &ModelTarget) -> Result<Classification, String> {
        let text = self
            .client
            .classify(
                target,
                &self.cwd,
                CLASSIFIER_INSTRUCTIONS,
                &classification_input(prompt),
                classification_schema(),
            )
            .map_err(|error: AppServerError| error.to_string())?;
        parse_classification(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scripted(Result<Classification, String>, usize);

    impl ModelClassifier for Scripted {
        fn classify(&mut self, _: &str, target: &ModelTarget) -> Result<Classification, String> {
            assert_eq!(target, &RouterConfig::default().classifier);
            self.1 += 1;
            self.0.clone()
        }
    }

    fn model(tier: Tier) -> Scripted {
        Scripted(
            Ok(Classification {
                tier,
                reason: "scripted".into(),
            }),
            0,
        )
    }

    #[test]
    fn risk_rules_force_fuerte_without_calling_the_classifier() {
        let (config, rules) = (RouterConfig::default(), RiskRules::default());
        let mut classifier = model(Tier::Economico);
        let decision =
            Router::new(&config, &rules).decide("Corrige el pago en producción", &mut classifier);
        assert_eq!(decision.tier, Tier::Fuerte);
        assert_eq!(decision.source, DecisionSource::Risk);
        assert_eq!(decision.model, config.tiers.fuerte.model);
        assert_eq!(decision.effort, config.tiers.fuerte.effort);
        assert_eq!(classifier.1, 0);
    }

    #[test]
    fn short_light_intents_stay_economico_without_a_model_call() {
        let (config, rules) = (RouterConfig::default(), RiskRules::default());
        let mut classifier = model(Tier::Fuerte);
        let decision = Router::new(&config, &rules)
            .decide("Explícame qué es un trait en Rust", &mut classifier);
        assert_eq!(decision.tier, Tier::Economico);
        assert_eq!(decision.source, DecisionSource::Rule);
        assert_eq!(classifier.1, 0);
    }

    #[test]
    fn doubtful_prompts_use_the_classifier_and_failures_use_the_fallback() {
        let (config, rules) = (RouterConfig::default(), RiskRules::default());
        let router = Router::new(&config, &rules);
        let prompt = "Agrega paginación al listado de hilos del panel";
        let mut classifier = model(Tier::Estandar);
        let decision = router.decide(prompt, &mut classifier);
        assert_eq!(
            (decision.tier, decision.source),
            (Tier::Estandar, DecisionSource::Model)
        );
        assert_eq!(classifier.1, 1);

        let mut broken = Scripted(Err("timeout".into()), 0);
        let decision = router.decide(prompt, &mut broken);
        assert_eq!(decision.tier, config.fallback_tier);
        assert_eq!(decision.source, DecisionSource::Fallback);
        assert!(decision.reason.contains("timeout"));
    }

    #[test]
    fn long_or_code_prompts_are_not_settled_by_fast_rules() {
        let long = format!("explica {}", "palabra ".repeat(40));
        assert_eq!(fast_rule(&long), None);
        assert_eq!(fast_rule("explica esto\n```rust\nfn main() {}\n```"), None);
        assert_eq!(fast_rule("implementa el endpoint de exportación"), None);
    }

    #[test]
    fn escalation_is_manual_and_keeps_risk_context() {
        let (config, rules) = (RouterConfig::default(), RiskRules::default());
        let router = Router::new(&config, &rules);
        let base = router.decide("resume este archivo", &mut model(Tier::Economico));
        let escalated = router.escalate(&base);
        assert_eq!(escalated.tier, Tier::Fuerte);
        assert_eq!(escalated.source, DecisionSource::Manual);
        assert_eq!(escalated.model, config.tiers.fuerte.model);
    }

    #[test]
    fn classification_parsing_tolerates_fences_and_rejects_garbage() {
        let parsed = parse_classification(
            "```json\n{\"tier\":\"fuerte\",\"reason\":\" toca arquitectura \"}\n```",
        )
        .unwrap();
        assert_eq!(parsed.tier, Tier::Fuerte);
        assert_eq!(parsed.reason, "toca arquitectura");
        assert!(parse_classification("no sé").is_err());
        assert!(parse_classification("{\"tier\":\"premium\",\"reason\":\"x\"}").is_err());
        assert_eq!(
            parse_classification("{\"tier\":\"economico\",\"reason\":\"\"}")
                .unwrap()
                .reason,
            "sin motivo"
        );
    }
}
