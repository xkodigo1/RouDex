use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Generic high-risk vocabulary. Projects add their own terms and paths in
/// `.roudex/rules.toml`; matching is accent- and case-insensitive and works on
/// whole words, so "quiero" never matches "ui".
const DEFAULT_KEYWORDS: &[&str] = &[
    "produccion",
    "production",
    "prod",
    "migracion",
    "migraciones",
    "migration",
    "migrations",
    "credencial",
    "credenciales",
    "credential",
    "credentials",
    "secret",
    "secrets",
    "secreto",
    "secretos",
    "password",
    "passwords",
    "contrasena",
    "api key",
    "dinero",
    "money",
    "pago",
    "pagos",
    "payment",
    "payments",
    "deploy",
    "desplegar",
    "despliegue",
    "rollback",
    "borrado masivo",
    "eliminar datos",
    "delete data",
    "drop table",
    "truncate",
    "rm rf",
    "force push",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "kind", content = "value")]
pub enum RiskMatch {
    Keyword(String),
    Path(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct RulesFile {
    extend_defaults: Option<bool>,
    keywords: Vec<String>,
    paths: Vec<String>,
}

#[derive(Debug, Error)]
pub enum RiskRulesError {
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid risk rules in {path}: {message}")]
    Parse { path: PathBuf, message: String },
}

/// Fixed risk rule set: any match forces the strongest tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskRules {
    keywords: Vec<Vec<String>>,
    paths: Vec<String>,
}

impl Default for RiskRules {
    fn default() -> Self {
        Self {
            keywords: DEFAULT_KEYWORDS.iter().map(|k| words(k)).collect(),
            paths: Vec::new(),
        }
    }
}

impl RiskRules {
    pub fn project_rules_path(project_dir: &Path) -> PathBuf {
        project_dir.join(".roudex").join("rules.toml")
    }

    /// Built-in defaults plus the project's `.roudex/rules.toml`, if present.
    pub fn for_project(project_dir: &Path) -> Result<Self, RiskRulesError> {
        let path = Self::project_rules_path(project_dir);
        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                Self::from_project_toml(&contents).map_err(|message| RiskRulesError::Parse {
                    path: path.clone(),
                    message,
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(RiskRulesError::Read { path, source }),
        }
    }

    pub fn from_project_toml(contents: &str) -> Result<Self, String> {
        let file: RulesFile = toml::from_str(contents).map_err(|error| error.to_string())?;
        let mut rules = if file.extend_defaults.unwrap_or(true) {
            Self::default()
        } else {
            Self {
                keywords: Vec::new(),
                paths: Vec::new(),
            }
        };
        for keyword in &file.keywords {
            let phrase = words(keyword);
            if phrase.is_empty() {
                return Err(format!("empty keyword: {keyword:?}"));
            }
            rules.keywords.push(phrase);
        }
        for path in &file.paths {
            let path = normalize_path(path);
            if path.is_empty() {
                return Err("empty path rule".into());
            }
            rules.paths.push(path);
        }
        Ok(rules)
    }

    pub fn matches(&self, prompt: &str) -> Vec<RiskMatch> {
        let tokens = words(prompt);
        let path_text = normalize_path(prompt);
        let mut found = Vec::new();
        for phrase in &self.keywords {
            if tokens
                .windows(phrase.len())
                .any(|window| window == phrase.as_slice())
            {
                found.push(RiskMatch::Keyword(phrase.join(" ")));
            }
        }
        for path in &self.paths {
            if path_text.contains(path.as_str()) {
                found.push(RiskMatch::Path(path.clone()));
            }
        }
        found
    }
}

/// Lowercase, strip Spanish accents and split on anything that is not a
/// letter or digit.
pub(crate) fn words(text: &str) -> Vec<String> {
    fold(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

fn normalize_path(text: &str) -> String {
    fold(text).replace('\\', "/").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keywords(matches: Vec<RiskMatch>) -> Vec<String> {
        matches
            .into_iter()
            .map(|m| match m {
                RiskMatch::Keyword(k) | RiskMatch::Path(k) => k,
            })
            .collect()
    }

    #[test]
    fn matches_whole_words_ignoring_case_and_accents() {
        let rules = RiskRules::default();
        assert_eq!(
            keywords(rules.matches("Revisa la MIGRACIÓN antes de ir a Producción")),
            ["produccion", "migracion"]
        );
        assert!(rules.matches("quiero construir la guía digital").is_empty());
        assert!(rules.matches("productivity tips and prodding").is_empty());
    }

    #[test]
    fn multi_word_phrases_need_contiguous_words() {
        let rules = RiskRules::default();
        assert_eq!(
            keywords(rules.matches("ejecuta DROP TABLE users;")),
            ["drop table"]
        );
        assert!(rules.matches("drop the table idea").is_empty());
        assert_eq!(keywords(rules.matches("corre rm -rf ./build")), ["rm rf"]);
    }

    #[test]
    fn project_rules_extend_defaults_with_keywords_and_paths() {
        let rules = RiskRules::from_project_toml(
            "keywords = [\"ledger\", \"settlement fee\"]\npaths = [\"src\\\\billing\\\\\"]",
        )
        .unwrap();
        assert_eq!(
            keywords(rules.matches("Ajusta el Ledger en src/Billing/fees.rs")),
            ["ledger", "src/billing/"]
        );
        assert_eq!(keywords(rules.matches("sube a producción")), ["produccion"]);
    }

    #[test]
    fn project_rules_can_replace_defaults_and_reject_bad_input() {
        let rules =
            RiskRules::from_project_toml("extend_defaults = false\nkeywords = [\"ledger\"]")
                .unwrap();
        assert!(rules.matches("deploy a producción").is_empty());
        assert!(RiskRules::from_project_toml("keywords = [\"  \"]").is_err());
        assert!(RiskRules::from_project_toml("unknown = true").is_err());
    }

    #[test]
    fn missing_project_file_uses_defaults() {
        let dir = std::env::temp_dir().join(format!("roudex-risk-{}", uuid::Uuid::new_v4()));
        assert_eq!(RiskRules::for_project(&dir).unwrap(), RiskRules::default());
    }
}
