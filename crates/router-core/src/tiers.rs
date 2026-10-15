use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

/// Language of what RouDex writes: the app, router reasons and the Desktop
/// hook's context and warning (spec 005, D6). English when unset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Es,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Es => "es",
        }
    }

    /// The text for this language.
    pub fn pick<T>(self, en: T, es: T) -> T {
        match self {
            Language::En => en,
            Language::Es => es,
        }
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

/// What a model costs, in USD per million tokens, for the savings estimate
/// (spec 005, D5). RouDex ships no prices: the user sets them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPrice {
    pub input: f64,
    pub cached_input: f64,
    pub output: f64,
}

/// Router configuration (`router.toml`). Absence selects the defaults, and a
/// file may set only some keys; an invalid document is an error, never a
/// silent fallback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
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
    /// The Codex executable, when RouDex cannot find it on its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_path: Option<PathBuf>,
    /// Chosen by the user in the app; unset means English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    /// Prices by model id, for the savings estimate.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub prices: BTreeMap<String, ModelPrice>,
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
            codex_path: None,
            language: None,
            prices: BTreeMap::new(),
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
    #[error("the price of {0} must be zero or more, in USD per million tokens")]
    Price(String),
}

impl RouterConfig {
    pub fn language(&self) -> Language {
        self.language.unwrap_or_default()
    }

    /// Set one key of `router.toml` (`section` is a table such as
    /// `savings`, or `None` for the top level) and keep every other line,
    /// comments included. The result must still be a valid configuration;
    /// otherwise nothing is written.
    pub fn write_setting(
        path: &Path,
        section: Option<&str>,
        key: &str,
        value: &str,
    ) -> Result<Self, RouterConfigError> {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        let updated = set_line(&contents, section, key, value);
        let config = Self::from_toml(&updated)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let partial = path.with_extension("toml.partial");
        std::fs::write(&partial, updated)?;
        std::fs::rename(&partial, path)?;
        Ok(config)
    }

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

    /// `~/.roudex/router.toml`, next to the local store (see [`crate::paths`]).
    pub fn default_path() -> PathBuf {
        crate::paths::config_path()
    }

    pub fn target(&self, tier: Tier) -> &ModelTarget {
        match tier {
            Tier::Economico => &self.tiers.economico,
            Tier::Estandar => &self.tiers.estandar,
            Tier::Fuerte => &self.tiers.fuerte,
        }
    }

    fn validate(&self) -> Result<(), RouterConfigError> {
        for (model, price) in &self.prices {
            if [price.input, price.cached_input, price.output]
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
            {
                return Err(RouterConfigError::Price(model.clone()));
            }
        }
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

/// `contents` with `key = value` in `section`: the existing line replaced,
/// or a new line after the section's last setting, or a new section at the
/// end. Top-level keys go before the first table.
fn set_line(contents: &str, section: Option<&str>, key: &str, value: &str) -> String {
    let line = format!("{key} = {value}");
    let mut lines: Vec<String> = contents.lines().map(str::to_owned).collect();
    let header = |text: &str| {
        let text = text.trim();
        (text.starts_with('[') && !text.starts_with("[[")).then(|| {
            text.trim_start_matches('[')
                .split(']')
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned()
        })
    };
    let is_key = |text: &str| {
        text.trim_start()
            .strip_prefix(key)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    };
    // The section's lines: from its header (or the top) to the next header.
    let start = match section {
        None => Some(0),
        Some(name) => lines
            .iter()
            .position(|text| header(text).as_deref() == Some(name))
            .map(|index| index + 1),
    };
    let Some(start) = start else {
        if lines.last().is_some_and(|last| !last.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(format!("[{}]", section.unwrap_or_default()));
        lines.push(line);
        return lines.join(
            "
",
        ) + "
";
    };
    let end = lines[start..]
        .iter()
        .position(|text| header(text).is_some())
        .map_or(lines.len(), |offset| start + offset);
    if let Some(index) = (start..end).find(|&index| is_key(&lines[index])) {
        lines[index] = line;
    } else {
        let after = (start..end)
            .rev()
            .find(|&index| {
                !lines[index].trim().is_empty() && !lines[index].trim_start().starts_with('#')
            })
            .map_or(start, |index| index + 1);
        lines.insert(after, line);
    }
    lines.join(
        "
",
    ) + "
"
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMENTED: &str = "# My router
version = 1 # keep
fallback_tier = \"estandar\"

[tiers.economico]
model = \"gpt-6-luna\"
effort = \"high\"

[tiers.estandar]
model = \"gpt-6.1-sol\"
effort = \"medium\"

[tiers.fuerte]
model = \"gpt-6.1-sol\"
effort = \"high\"

[classifier]
model = \"gpt-6-luna\"
effort = \"low\"
";

    #[test]
    fn a_setting_changes_one_line_and_keeps_the_rest() {
        let added = set_line(COMMENTED, None, "language", "\"es\"");
        let expected = COMMENTED.replacen(
            "fallback_tier = \"estandar\"
",
            "fallback_tier = \"estandar\"
language = \"es\"
",
            1,
        );
        assert_eq!(added, expected, "added after the last top-level setting");
        let changed = set_line(&added, None, "language", "\"en\"");
        assert_eq!(
            changed,
            expected.replace("language = \"es\"", "language = \"en\"")
        );
        assert_eq!(
            RouterConfig::from_toml(&changed).unwrap().language(),
            Language::En
        );

        let section = set_line(&changed, Some("savings"), "include_codex_desktop", "true");
        assert!(section.starts_with(&changed), "a new table goes at the end");
        assert!(section.ends_with(
            "
[savings]
include_codex_desktop = true
"
        ));
        let toggled = set_line(&section, Some("savings"), "include_codex_desktop", "false");
        assert_eq!(toggled, section.replace("= true", "= false"));
    }

    #[test]
    fn a_setting_in_an_empty_or_missing_file_is_all_the_file_holds() {
        assert_eq!(
            set_line("", None, "language", "\"es\""),
            "language = \"es\"
"
        );
        let dir = std::env::temp_dir().join(format!("roudex-config-{}", uuid::Uuid::new_v4()));
        let path = dir.join("router.toml");
        let config = RouterConfig::write_setting(&path, None, "language", "\"es\"").unwrap();
        assert_eq!(config.language(), Language::Es);
        assert_eq!(
            config.tiers,
            RouterConfig::default().tiers,
            "the rest stays default"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "language = \"es\"
"
        );
        assert!(RouterConfig::write_setting(&path, None, "language", "\"fr\"").is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "language = \"es\"
",
            "an invalid result is never written"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn prices_are_read_by_model_and_must_not_be_negative() {
        let config = RouterConfig::from_toml(
            "[prices.\"gpt-6.1-sol\"]\ninput = 2.0\ncached_input = 0.5\noutput = 10\n",
        )
        .unwrap();
        assert_eq!(
            config.prices["gpt-6.1-sol"],
            ModelPrice {
                input: 2.0,
                cached_input: 0.5,
                output: 10.0
            }
        );
        let negative = "[prices.x]\ninput = -1.0\ncached_input = 0\noutput = 0\n";
        assert!(matches!(
            RouterConfig::from_toml(negative),
            Err(RouterConfigError::Price(model)) if model == "x"
        ));
        assert!(
            RouterConfig::from_toml("[prices.x]\ninput = 1.0\n").is_err(),
            "all three prices"
        );
    }

    #[test]
    fn unset_language_means_english() {
        assert_eq!(RouterConfig::default().language(), Language::En);
        assert_eq!(Language::Es.pick("hi", "hola"), "hola");
    }

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
