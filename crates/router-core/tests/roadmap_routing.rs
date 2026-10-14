use router_core::{ModelRoute, RoutingConfig, RoutingConfigError, ROUTING_CONFIG_VERSION};
use serde_json::json;

fn candidate(profile: &str, model: &str, effort: &str) -> serde_json::Value {
    json!({
        "version": 1,
        "revision": 2,
        "overrides": [{ "profile": profile, "model": model, "effort": effort }]
    })
}

fn configured() -> RoutingConfig {
    RoutingConfig::from_json(&candidate("deep", "gpt-6-astra", "medium").to_string()).unwrap()
}

#[test]
fn absent_configuration_preserves_every_exact_builtin_assignment() {
    let config = RoutingConfig::from_optional_json(None).unwrap();
    assert_eq!(config, RoutingConfig::default());
    assert_eq!(config.version(), ROUTING_CONFIG_VERSION);
    assert_eq!(config.revision(), 1);
    assert!(config.overrides().is_empty());
    for (profile, model, effort) in [
        ("micro", "gpt-6-luna", "low"),
        ("routine", "gpt-6-luna", "medium"),
        ("standard", "gpt-6.1-sol", "medium"),
        ("deep", "gpt-6.1-sol", "high"),
        ("frontier", "gpt-6-astra", "high"),
    ] {
        assert_eq!(
            config.resolve_profile(profile),
            Some(ModelRoute {
                profile,
                model,
                effort
            })
        );
    }
    assert_eq!(config.resolve_profile("unknown"), None);
}

#[test]
fn partial_override_changes_only_the_explicit_profile() {
    let config = configured();
    let defaults = RoutingConfig::default();
    assert_eq!(config.revision(), 2);
    assert_eq!(config.overrides().len(), 1);
    assert_eq!(
        config.resolve_profile("deep"),
        Some(ModelRoute {
            profile: "deep",
            model: "gpt-6-astra",
            effort: "medium"
        })
    );
    for profile in ["micro", "routine", "standard", "frontier"] {
        assert_eq!(
            config.resolve_profile(profile),
            defaults.resolve_profile(profile)
        );
    }
    // Loading a contract must not change the existing global/default resolver.
    assert_eq!(
        router_core::routing::resolve_profile("deep"),
        defaults.resolve_profile("deep")
    );
}

#[test]
fn invalid_profile_model_and_effort_preserve_the_entire_last_valid_revision() {
    let mut config = configured();
    let original = config.clone();
    for (field, value, expected) in [
        ("profile", "unknown", RoutingConfigError::InvalidProfile(1)),
        ("model", "unknown", RoutingConfigError::InvalidModel(1)),
        ("effort", "unknown", RoutingConfigError::InvalidEffort(1)),
    ] {
        let mut next = candidate("standard", "gpt-6-luna", "low");
        next["revision"] = json!(3);
        let mut invalid = json!({ "profile": "routine", "model": "gpt-6-astra", "effort": "high" });
        invalid[field] = json!(value);
        next["overrides"].as_array_mut().unwrap().push(invalid);
        assert_eq!(config.replace_from_json(&next.to_string()), Err(expected));
        assert_eq!(config, original, "partial candidate must not be applied");
        assert!(RoutingConfig::from_optional_json(Some(&next.to_string())).is_err());
    }
}

#[test]
fn invalid_versions_revisions_duplicates_and_documents_do_not_replace_configuration() {
    let mut config = configured();
    let original = config.clone();
    let mut version = candidate("standard", "gpt-6-luna", "low");
    version["version"] = json!(2);
    assert_eq!(
        config.replace_from_json(&version.to_string()),
        Err(RoutingConfigError::UnsupportedVersion)
    );
    let mut revision = version.clone();
    revision["version"] = json!(1);
    revision["revision"] = json!(0);
    assert_eq!(
        config.replace_from_json(&revision.to_string()),
        Err(RoutingConfigError::InvalidRevision)
    );
    let mut duplicate = candidate("standard", "gpt-6-luna", "low");
    duplicate["overrides"].as_array_mut().unwrap().push(json!({
        "profile": "standard", "model": "gpt-6-astra", "effort": "high"
    }));
    assert_eq!(
        config.replace_from_json(&duplicate.to_string()),
        Err(RoutingConfigError::DuplicateProfile(1))
    );
    for document in ["", "null", "{}", "{", r#"{"version":1,"revision":1}"#] {
        assert!(config.replace_from_json(document).is_err());
        assert_eq!(config, original);
    }
    assert_eq!(config, original);
}

#[test]
fn authentication_and_unknown_fields_are_rejected_at_every_level_without_echoing_values() {
    let mut config = configured();
    let original = config.clone();
    for field in [
        "api_key",
        "token",
        "credentials",
        "authentication",
        "endpoint",
        "typo",
    ] {
        for nested in [false, true] {
            let mut document = candidate("standard", "gpt-6-luna", "low");
            let object = if nested {
                &mut document["overrides"][0]
            } else {
                &mut document
            };
            object[field] = json!("SYNTHETIC_REJECTED_VALUE");
            let error = config.replace_from_json(&document.to_string()).unwrap_err();
            assert!(!error.to_string().contains("SYNTHETIC_REJECTED_VALUE"));
            assert_eq!(config, original);
        }
    }
    let serialized = serde_json::to_value(&config).unwrap();
    let top = serialized.as_object().unwrap();
    assert_eq!(top.len(), 3);
    for key in ["version", "revision", "overrides"] {
        assert!(top.contains_key(key));
    }
    let rule = serialized["overrides"][0].as_object().unwrap();
    assert_eq!(rule.len(), 3);
    for key in ["profile", "model", "effort"] {
        assert!(rule.contains_key(key));
    }
}

#[test]
fn validated_configuration_round_trips_and_can_be_replaced_explicitly() {
    let mut config = configured();
    let serialized = serde_json::to_string(&config).unwrap();
    assert_eq!(RoutingConfig::from_json(&serialized).unwrap(), config);
    assert_eq!(
        serde_json::from_str::<RoutingConfig>(&serialized).unwrap(),
        config
    );
    let mut next = candidate("standard", "gpt-6-luna", "low");
    next["revision"] = json!(3);
    config.replace_from_json(&next.to_string()).unwrap();
    assert_eq!(config.revision(), 3);
    assert_eq!(
        config.resolve_profile("standard").unwrap().model,
        "gpt-6-luna"
    );
    // Replacement removes overrides absent from the new revision.
    assert_eq!(
        config.resolve_profile("deep"),
        RoutingConfig::default().resolve_profile("deep")
    );
    config
        .replace_from_json(r#"{"version":1,"revision":4,"overrides":[]}"#)
        .unwrap();
    assert!(config.overrides().is_empty());
    for profile in ["micro", "routine", "standard", "deep", "frontier"] {
        assert_eq!(
            config.resolve_profile(profile),
            RoutingConfig::default().resolve_profile(profile)
        );
    }
}

#[test]
fn serde_deserialization_cannot_bypass_contract_validation() {
    for document in [
        candidate("unknown", "gpt-6-luna", "low"),
        candidate("deep", "unknown", "low"),
        candidate("deep", "gpt-6-luna", "unknown"),
        json!({"version": 99, "revision": 1, "overrides": []}),
        json!({"version": 1, "revision": 0, "overrides": []}),
    ] {
        assert!(serde_json::from_value::<RoutingConfig>(document).is_err());
    }
}
