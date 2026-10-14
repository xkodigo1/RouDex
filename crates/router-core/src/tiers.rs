use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const ROUTER_CONFIG_VERSION: u32 = 1;

/// Cost/capability tier chosen by the router. Concrete models live in
/// configuration so a new model release never requires a code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Economico,
    Estandar,
    Fuerte,
}

impl Tier {
    pub const ALL: [Tier; 3] = [Tier::Economico, Tier::Estandar, Tier::Fuerte];

    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Economico => "economico",
            Tier::Estandar => "estandar",
            Tier::Fuerte => "fuerte",
        }
    }

    pub fn parse(value: &str) -> Option<Tier> {
        Tier::ALL
            .into_iter()
            .find(|tier| tier.as_str() == value.trim().to_lowercase())
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTarget {
    pub model: String,
    pub effort: String,
}

impl ModelTarget {
    fn new(model: &str, effort: &str) -> Self {
        Self {
            model: model.into(),
            effort: effort.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TierTargets {
    pub economico: ModelTarget,
    pub estandar: ModelTarget,
    pub fuerte: ModelTarget,
}

/// Router configuration (`router.toml`). Absence selects the defaults; an
/// existing invalid document is an error, never a silent fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterConfig {
    pub version: u32,
    /// Tier used when the model classifier is unavailable or answers badly.
    pub fallback_tier: Tier,
    pub tiers: TierTargets,
    /// Model that classifies prompts the fast rules cannot settle.
    pub classifier: ModelTarget,
    /// Codex Desktop hook options; optional so older files keep loading.
    #[serde(default)]
    pub desktop_hook: DesktopHookConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DesktopHookConfig {
    /// Block risky Codex Desktop prompts sent with a model other than the
    /// `fuerte` one, instead of only warning.
    pub block_high_risk: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            version: ROUTER_CONFIG_VERSION,
            fallback_tier: Tier::Estandar,
            tiers: TierTargets {
                economico: ModelTarget::new("gpt-6-luna", "high"),
                estandar: ModelTarget::new("gpt-6.1-sol", "medium"),
                fuerte: ModelTarget::new("gpt-6.1-sol", "high"),
            },
            classifier: ModelTarget::new("gpt-6-luna", "low"),
            desktop_hook: DesktopHookConfig::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum RouterConfigError {
    #[error("could not read router configuration: {0}")]
    Read(#[from] std::io::Error),
    #[error("invalid router configuration: {0}")]
    Parse(String),
    #[error("unsupported router configuration version {0}; expected {ROUTER_CONFIG_VERSION}")]
    Version(u32),
    #[error("router configuration has an empty model or effort for {0}")]
    Empty(&'static str),
}

impl RouterConfig {
    pub fn from_toml(contents: &str) -> Result<Self, RouterConfigError> {
        let config: RouterConfig = toml::from_str(contents)
            .map_err(|error| RouterConfigError::Parse(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_or_default(path: &Path) -> Result<Self, RouterConfigError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Self::from_toml(&contents),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    /// `%LOCALAPPDATA%\RouDex\router.toml`, next to the local store.
    pub fn default_path() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("RouDex")
            .join("router.toml")
    }

    pub fn target(&self, tier: Tier) -> &ModelTarget {
        match tier {
            Tier::Economico => &self.tiers.economico,
            Tier::Estandar => &self.tiers.estandar,
            Tier::Fuerte => &self.tiers.fuerte,
        }
    }

    fn validate(&self) -> Result<(), RouterConfigError> {
        if self.version != ROUTER_CONFIG_VERSION {
            return Err(RouterConfigError::Version(self.version));
        }
        let targets = Tier::ALL
            .into_iter()
            .map(|tier| (tier.as_str(), self.target(tier)))
            .chain([("classifier", &self.classifier)]);
        for (name, target) in targets {
            if target.model.trim().is_empty() || target.effort.trim().is_empty() {
                return Err(RouterConfigError::Empty(name));
            }
        }
        Ok(())
    }

    /// Check every configured target against the runtime catalog
    /// (`model/list`). Returns one message per problem; empty means valid.
    pub fn catalog_issues(&self, catalog: &[CatalogModel]) -> Vec<String> {
        let targets = Tier::ALL
            .into_iter()
            .map(|tier| (tier.as_str(), self.target(tier)))
            .chain([("classifier", &self.classifier)]);
        let mut issues = Vec::new();
        for (name, target) in targets {
            match catalog.iter().find(|model| model.matches(&target.model)) {
                None => issues.push(format!(
                    "{name}: el modelo {} no está en el catálogo del runtime",
                    target.model
                )),
                Some(model) if !model.efforts.contains(&target.effort) => issues.push(format!(
                    "{name}: {} no admite el esfuerzo {} (admite: {})",
                    target.model,
                    target.effort,
                    model.efforts.join(", ")
                )),
                Some(_) => {}
            }
        }
        issues
    }
}

/// One entry of the runtime model catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogModel {
    pub id: String,
    pub model: String,
    pub display_name: String,
    pub hidden: bool,
    pub default_effort: String,
    pub efforts: Vec<String>,
}

impl CatalogModel {
    fn matches(&self, name: &str) -> bool {
        self.id == name || self.model == name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Vec<CatalogModel> {
        let entry = |id: &str, efforts: &[&str]| CatalogModel {
            id: id.into(),
            model: id.into(),
            display_name: id.into(),
            hidden: false,
            default_effort: "medium".into(),
            efforts: efforts.iter().map(|e| e.to_string()).collect(),
        };
        vec![
            entry("gpt-6-luna", &["low", "medium", "high"]),
            entry("gpt-6.1-sol", &["low", "medium", "high", "xhigh"]),
        ]
    }

    #[test]
    fn defaults_are_valid_against_a_matching_catalog() {
        assert!(RouterConfig::default()
            .catalog_issues(&catalog())
            .is_empty());
    }

    #[test]
    fn catalog_reports_missing_models_and_unsupported_efforts() {
        let mut config = RouterConfig::default();
        config.tiers.fuerte = ModelTarget::new("gpt-6-astra", "high");
        config.classifier = ModelTarget::new("gpt-6-luna", "ultra");
        let issues = config.catalog_issues(&catalog());
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert!(issues[0].starts_with("fuerte:"));
        assert!(issues[1].starts_with("classifier:"));
    }

    #[test]
    fn toml_round_trip_and_strict_parsing() {
        let text = toml::to_string(&RouterConfig::default()).unwrap();
        assert_eq!(
            RouterConfig::from_toml(&text).unwrap(),
            RouterConfig::default()
        );
        let unknown = format!("{text}\nsurprise = 1\n");
        assert!(RouterConfig::from_toml(&unknown).is_err());
        let wrong_version = text.replace("version = 1", "version = 2");
        assert!(matches!(
            RouterConfig::from_toml(&wrong_version),
            Err(RouterConfigError::Version(2))
        ));
        let empty = text.replacen("effort = \"medium\"", "effort = \"\"", 1);
        assert!(matches!(
            RouterConfig::from_toml(&empty),
            Err(RouterConfigError::Empty(_))
        ));
    }

    #[test]
    fn desktop_hook_section_is_optional_and_strict() {
        let text = toml::to_string(&RouterConfig::default()).unwrap();
        let without: String = text.split("[desktop_hook]").next().unwrap().to_owned();
        assert!(
            !RouterConfig::from_toml(&without)
                .unwrap()
                .desktop_hook
                .block_high_risk
        );
        let blocking = format!("{without}\n[desktop_hook]\nblock_high_risk = true\n");
        assert!(
            RouterConfig::from_toml(&blocking)
                .unwrap()
                .desktop_hook
                .block_high_risk
        );
        let typo = format!("{without}\n[desktop_hook]\nblock_risk = true\n");
        assert!(RouterConfig::from_toml(&typo).is_err());
    }

    #[test]
    fn missing_file_selects_defaults() {
        let path =
            std::env::temp_dir().join(format!("roudex-missing-{}.toml", uuid::Uuid::new_v4()));
        assert_eq!(
            RouterConfig::load_or_default(&path).unwrap(),
            RouterConfig::default()
        );
    }

    #[test]
    fn tier_parsing_is_case_insensitive() {
        assert_eq!(Tier::parse(" Fuerte "), Some(Tier::Fuerte));
        assert_eq!(Tier::parse("premium"), None);
    }
}
