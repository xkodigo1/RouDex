use crate::domain::RoutePromptInput;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const ROUTING_CONFIG_VERSION: u32 = 1;

/// One explicit local assignment. Model identifiers are the repository's supported
/// routing vocabulary, not a claim that the installed runtime offers the model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RoutingOverride {
    pub profile: String,
    pub model: String,
    pub effort: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutingConfigDocument {
    version: u32,
    revision: u64,
    overrides: Vec<RoutingOverride>,
}

/// Validated, versioned local configuration. Unspecified profiles retain their
/// built-in assignment. This contract performs no I/O and starts no execution;
/// persistence and dispatch integration belong to their respective callers.
///
/// Version 1 accepts the three built-in model identifiers and low/medium/high
/// efforts. Unknown fields (including authentication data) are rejected at both
/// document and override level. Private fields prevent mutation after validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "RoutingConfigDocument")]
pub struct RoutingConfig {
    version: u32,
    revision: u64,
    overrides: Vec<RoutingOverride>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RoutingConfigError {
    // Do not echo untrusted configuration values in diagnostics.
    #[error("invalid routing configuration document at line {line}, column {column}")]
    Parse { line: usize, column: usize },
    #[error("unsupported routing configuration version; expected version 1")]
    UnsupportedVersion,
    #[error("routing configuration revision must be greater than zero")]
    InvalidRevision,
    #[error("unknown routing profile in override {0}")]
    InvalidProfile(usize),
    #[error("unsupported model in routing override {0}")]
    InvalidModel(usize),
    #[error("unsupported effort in routing override {0}")]
    InvalidEffort(usize),
    #[error("duplicate profile in routing override {0}")]
    DuplicateProfile(usize),
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            version: ROUTING_CONFIG_VERSION,
            revision: 1,
            overrides: Vec::new(),
        }
    }
}

impl TryFrom<RoutingConfigDocument> for RoutingConfig {
    type Error = RoutingConfigError;

    fn try_from(document: RoutingConfigDocument) -> Result<Self, Self::Error> {
        if document.version != ROUTING_CONFIG_VERSION {
            return Err(RoutingConfigError::UnsupportedVersion);
        }
        if document.revision == 0 {
            return Err(RoutingConfigError::InvalidRevision);
        }
        let mut profiles = HashSet::new();
        for (index, rule) in document.overrides.iter().enumerate() {
            if resolve_profile(&rule.profile).is_none() {
                return Err(RoutingConfigError::InvalidProfile(index));
            }
            if supported_model(&rule.model).is_none() {
                return Err(RoutingConfigError::InvalidModel(index));
            }
            if supported_effort(&rule.effort).is_none() {
                return Err(RoutingConfigError::InvalidEffort(index));
            }
            if !profiles.insert(&rule.profile) {
                return Err(RoutingConfigError::DuplicateProfile(index));
            }
        }
        Ok(Self {
            version: document.version,
            revision: document.revision,
            overrides: document.overrides,
        })
    }
}

impl RoutingConfig {
    pub fn from_json(contents: &str) -> Result<Self, RoutingConfigError> {
        let document =
            serde_json::from_str::<RoutingConfigDocument>(contents).map_err(|error| {
                RoutingConfigError::Parse {
                    line: error.line(),
                    column: error.column(),
                }
            })?;
        document.try_into()
    }

    /// Only absence selects defaults. An existing invalid document fails closed.
    pub fn from_optional_json(contents: Option<&str>) -> Result<Self, RoutingConfigError> {
        contents
            .map(Self::from_json)
            .transpose()
            .map(Option::unwrap_or_default)
    }

    /// Validate the whole candidate before replacing the last valid configuration.
    pub fn replace_from_json(&mut self, contents: &str) -> Result<(), RoutingConfigError> {
        let candidate = Self::from_json(contents)?;
        *self = candidate;
        Ok(())
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn overrides(&self) -> &[RoutingOverride] {
        &self.overrides
    }

    pub fn resolve_profile(&self, profile: &str) -> Option<ModelRoute> {
        let built_in = resolve_profile(profile)?;
        match self.overrides.iter().find(|rule| rule.profile == profile) {
            Some(rule) => Some(ModelRoute {
                profile: built_in.profile,
                model: supported_model(&rule.model).expect("validated model"),
                effort: supported_effort(&rule.effort).expect("validated effort"),
            }),
            None => Some(built_in),
        }
    }
}

fn supported_model(model: &str) -> Option<&'static str> {
    match model {
        "gpt-6-luna" => Some("gpt-6-luna"),
        "gpt-6.1-sol" => Some("gpt-6.1-sol"),
        "gpt-6-astra" => Some("gpt-6-astra"),
        _ => None,
    }
}

fn supported_effort(effort: &str) -> Option<&'static str> {
    match effort {
        "low" => Some("low"),
        "medium" => Some("medium"),
        "high" => Some("high"),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskClass {
    Micro,
    Routine,
    Standard,
    Deep,
    Frontier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRoute {
    pub profile: &'static str,
    pub model: &'static str,
    pub effort: &'static str,
}

pub fn classify_prompt(prompt: &str) -> TaskClass {
    let normalized = prompt.to_ascii_lowercase();
    let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

    if compact.len() <= 220
        && ["color", "copy", "texto", "formato", "botón", "button"]
            .iter()
            .any(|term| compact.contains(term))
    {
        return TaskClass::Micro;
    }

    if [
        "incidente",
        "root cause",
        "causa raíz",
        "arquitectura",
        "threat model",
        "seguridad crítica",
        "pérdida de datos",
    ]
    .iter()
    .any(|term| compact.contains(term))
    {
        return TaskClass::Frontier;
    }

    if [
        "producción",
        "production",
        "credencial",
        "secret",
        "migración",
        "base de datos",
        "database",
        "ledger",
        "saldo",
        "pago",
        "webhook",
        "deploy",
        "desplegar",
        "externo",
        "provider",
    ]
    .iter()
    .any(|term| compact.contains(term))
    {
        return TaskClass::Deep;
    }

    if [
        "interfaz",
        "ui",
        "frontend",
        "contrato",
        "api",
        "integración",
        "browser",
    ]
    .iter()
    .any(|term| compact.contains(term))
    {
        return TaskClass::Standard;
    }

    if ["documentar", "readme", "test", "prueba", "git", "resumen"]
        .iter()
        .any(|term| compact.contains(term))
    {
        return TaskClass::Routine;
    }

    TaskClass::Standard
}

pub fn route_prompt(input: &RoutePromptInput) -> ModelRoute {
    match classify_prompt(&input.prompt) {
        TaskClass::Micro => resolve_profile("micro").expect("built-in profile"),
        TaskClass::Routine => resolve_profile("routine").expect("built-in profile"),
        TaskClass::Standard => resolve_profile("standard").expect("built-in profile"),
        TaskClass::Deep => resolve_profile("deep").expect("built-in profile"),
        TaskClass::Frontier => resolve_profile("frontier").expect("built-in profile"),
    }
}

pub fn resolve_profile(profile: &str) -> Option<ModelRoute> {
    match profile {
        "micro" => Some(ModelRoute {
            profile: "micro",
            model: "gpt-6-luna",
            effort: "low",
        }),
        "routine" => Some(ModelRoute {
            profile: "routine",
            model: "gpt-6-luna",
            effort: "medium",
        }),
        "standard" => Some(ModelRoute {
            profile: "standard",
            model: "gpt-6.1-sol",
            effort: "medium",
        }),
        "deep" => Some(ModelRoute {
            profile: "deep",
            model: "gpt-6.1-sol",
            effort: "high",
        }),
        "frontier" => Some(ModelRoute {
            profile: "frontier",
            model: "gpt-6-astra",
            effort: "high",
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::RoutePromptInput;

    fn input(prompt: &str) -> RoutePromptInput {
        RoutePromptInput {
            prompt: prompt.to_owned(),
            session_id: "session".to_owned(),
            turn_id: "turn".to_owned(),
            cwd: ".".to_owned(),
            permission_mode: None,
        }
    }

    #[test]
    fn classifies_high_risk_prompts_before_generic_terms() {
        assert_eq!(
            classify_prompt("Diagnosticar causa raíz de producción"),
            TaskClass::Frontier
        );
        assert_eq!(
            classify_prompt("Corregir saldo en producción"),
            TaskClass::Deep
        );
    }

    #[test]
    fn selects_expected_route_for_routine_work() {
        let route = route_prompt(&input("Actualizar el README con instrucciones"));
        assert_eq!(route.profile, "routine");
        assert_eq!(route.model, "gpt-6-luna");
    }
}
