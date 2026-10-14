use router_core::{
    AppServerClassifier, AppServerClient, AppServerConfig, DecisionSource, ModelClassifier,
    RiskRules, Router, RouterConfig, Tier,
};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("roudex-router-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create workspace");
        Self(dir)
    }

    fn client(&self, scenario: &str) -> AppServerClient {
        AppServerClient::start(&AppServerConfig {
            program: PathBuf::from(env!("CARGO_BIN_EXE_fake-app-server")),
            arguments: vec![scenario.into()],
            working_directory: self.0.clone(),
            service_name: "router-test".into(),
            active_task_id: None,
        })
        .expect("start fake app server")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn model_catalog_follows_pagination_and_validates_default_tiers() {
    let workspace = Workspace::new();
    let catalog = workspace
        .client("success")
        .list_models()
        .expect("model/list");
    let ids: Vec<_> = catalog.iter().map(|model| model.id.as_str()).collect();
    assert_eq!(ids, ["gpt-5.6-luna", "gpt-6.1-sol"]);
    assert_eq!(catalog[1].efforts, ["low", "medium", "high", "xhigh"]);
    assert!(RouterConfig::default().catalog_issues(&catalog).is_empty());
}

#[test]
fn app_server_classifier_runs_an_ephemeral_read_only_turn() {
    let workspace = Workspace::new();
    let config = RouterConfig::default();
    let mut classifier = AppServerClassifier::new(workspace.client("success"), &workspace.0);
    let result = classifier
        .classify(
            "Diseña la sincronización entre proyectos",
            &config.classifier,
        )
        .expect("classification");
    assert_eq!(result.tier, Tier::Fuerte);
    assert_eq!(result.reason, "fake classification");
}

#[test]
fn router_falls_back_when_the_runtime_classifier_misbehaves() {
    let workspace = Workspace::new();
    let (config, rules) = (RouterConfig::default(), RiskRules::default());
    let router = Router::new(&config, &rules);
    for scenario in ["classify-garbage", "classify-failed"] {
        let mut classifier = AppServerClassifier::new(workspace.client(scenario), &workspace.0);
        let decision = router.decide("Agrega filtros al historial de hilos", &mut classifier);
        assert_eq!(decision.source, DecisionSource::Fallback, "{scenario}");
        assert_eq!(decision.tier, config.fallback_tier, "{scenario}");
        if scenario == "classify-failed" {
            assert!(
                decision.reason.contains("usage limit reached"),
                "{}",
                decision.reason
            );
        }
    }
}

#[test]
fn project_rules_file_forces_fuerte_for_project_specific_terms() {
    let workspace = Workspace::new();
    let rules_path = RiskRules::project_rules_path(&workspace.0);
    fs::create_dir_all(rules_path.parent().unwrap()).unwrap();
    fs::write(&rules_path, "keywords = [\"settlement\"]\n").unwrap();
    let rules = RiskRules::for_project(&workspace.0).expect("project rules");
    let config = RouterConfig::default();
    let mut classifier = AppServerClassifier::new(workspace.client("success"), &workspace.0);
    let decision =
        Router::new(&config, &rules).decide("Revisa el cálculo del settlement", &mut classifier);
    assert_eq!(decision.tier, Tier::Fuerte);
    assert_eq!(decision.source, DecisionSource::Risk);

    fs::write(&rules_path, "keywords = \"no es una lista\"\n").unwrap();
    assert!(RiskRules::for_project(&workspace.0).is_err());
}
