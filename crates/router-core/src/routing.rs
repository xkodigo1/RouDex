use crate::domain::RoutePromptInput;

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
        TaskClass::Micro => ModelRoute {
            profile: "micro",
            model: "gpt-6-luna",
            effort: "low",
        },
        TaskClass::Routine => ModelRoute {
            profile: "routine",
            model: "gpt-6-luna",
            effort: "medium",
        },
        TaskClass::Standard => ModelRoute {
            profile: "standard",
            model: "gpt-6.1-sol",
            effort: "medium",
        },
        TaskClass::Deep => ModelRoute {
            profile: "deep",
            model: "gpt-6.1-sol",
            effort: "high",
        },
        TaskClass::Frontier => ModelRoute {
            profile: "frontier",
            model: "gpt-6-astra",
            effort: "high",
        },
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
