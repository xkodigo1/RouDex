//! Which model runs a spec task: the tier the spec declares, `fuerte` when a
//! risk rule matches, or the tier the user chose by hand. `router.toml` turns
//! the tier into a model and effort, as in the chat.

use crate::classifier::{describe, DecisionSource, RouteDecision};
use crate::domain::TaskDefinition;
use crate::risk::RiskRules;
use crate::tiers::{Language, RouterConfig, Tier};

/// Route a task. A manual tier wins; otherwise a risk match forces `fuerte`.
pub fn resolve_task_route(
    task: &TaskDefinition,
    tier_override: Option<Tier>,
    config: &RouterConfig,
    rules: &RiskRules,
) -> RouteDecision {
    let risk = rules.matches(&format!("{}\n{}", task.title, task.prompt));
    let language = config.language();
    let described = describe(&risk, language);
    let (tier, source, reason) = match tier_override {
        Some(tier) if risk.is_empty() => (
            tier,
            DecisionSource::Manual,
            match language {
                Language::En => format!("chosen by hand (the spec asks for {})", task.tier),
                Language::Es => format!("elegido manualmente (la spec pide {})", task.tier),
            },
        ),
        Some(tier) => (
            tier,
            DecisionSource::Manual,
            match language {
                Language::En => format!("chosen by hand despite the risk ({described})"),
                Language::Es => format!("elegido manualmente pese al riesgo ({described})"),
            },
        ),
        None if !risk.is_empty() => (
            Tier::Fuerte,
            DecisionSource::Risk,
            match language {
                Language::En => format!("risk rule: {described}"),
                Language::Es => format!("regla de riesgo: {described}"),
            },
        ),
        None => (
            task.tier,
            DecisionSource::Spec,
            match (&task.profile, language) {
                (Some(profile), Language::En) => format!("spec profile {profile}"),
                (Some(profile), Language::Es) => format!("perfil {profile} de la spec"),
                (None, _) => language.pick("spec tier", "tier de la spec").to_owned(),
            },
        ),
    };
    let target = config.target(tier);
    RouteDecision {
        tier,
        model: target.model.clone(),
        effort: target.effort.clone(),
        source,
        reason,
        risk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{TaskSandbox, TaskStatus};
    use crate::tiers::ModelTarget;

    fn task(tier: Tier, prompt: &str) -> TaskDefinition {
        TaskDefinition {
            id: "T-1".into(),
            title: "Tarea".into(),
            prompt: prompt.into(),
            depends_on: Vec::new(),
            status: TaskStatus::Planned,
            tier,
            profile: None,
            model: String::new(),
            effort: String::new(),
            milestone: None,
            approval: "not_required".into(),
            scope: vec!["src".into()],
            cwd: ".".into(),
            sandbox: TaskSandbox {
                kind: "readOnly".into(),
                writable_roots: Vec::new(),
            },
            acceptance: vec!["ok".into()],
            verify: "true".into(),
        }
    }

    #[test]
    fn the_spec_tier_maps_through_router_toml() {
        let mut config = RouterConfig::default();
        config.tiers.estandar = ModelTarget {
            model: "gpt-x".into(),
            effort: "low".into(),
        };
        let route = resolve_task_route(
            &task(Tier::Estandar, "agrega un test"),
            None,
            &config,
            &RiskRules::default(),
        );
        assert_eq!(
            (route.tier, route.model.as_str(), route.effort.as_str()),
            (Tier::Estandar, "gpt-x", "low")
        );
        assert_eq!(route.source, DecisionSource::Spec);
    }

    #[test]
    fn risk_forces_the_strong_tier_unless_chosen_by_hand() {
        let risky = task(Tier::Economico, "aplica la migración en producción");
        let config = RouterConfig {
            language: Some(Language::Es),
            ..RouterConfig::default()
        };
        let rules = RiskRules::default();
        let route = resolve_task_route(&risky, None, &config, &rules);
        assert_eq!(
            (route.tier, route.source),
            (Tier::Fuerte, DecisionSource::Risk)
        );
        assert!(route.reason.contains("migracion"), "{}", route.reason);
        assert_eq!(route.model, config.tiers.fuerte.model);

        let manual = resolve_task_route(&risky, Some(Tier::Economico), &config, &rules);
        assert_eq!(
            (manual.tier, manual.source),
            (Tier::Economico, DecisionSource::Manual)
        );
        assert!(manual.reason.contains("pese al riesgo"));
        assert!(!manual.risk.is_empty(), "the risk stays on the record");

        let english = resolve_task_route(
            &risky,
            Some(Tier::Economico),
            &RouterConfig::default(),
            &rules,
        );
        assert!(
            english.reason.contains("despite the risk"),
            "{}",
            english.reason
        );
    }

    #[test]
    fn legacy_profiles_explain_the_tier() {
        let mut legacy = task(Tier::Fuerte, "refactor del módulo");
        legacy.profile = Some("deep".into());
        let route = resolve_task_route(
            &legacy,
            None,
            &RouterConfig::default(),
            &RiskRules::default(),
        );
        assert_eq!(route.reason, "spec profile deep");
        let spanish = RouterConfig {
            language: Some(Language::Es),
            ..RouterConfig::default()
        };
        let route = resolve_task_route(&legacy, None, &spanish, &RiskRules::default());
        assert_eq!(route.reason, "perfil deep de la spec");
    }
}
