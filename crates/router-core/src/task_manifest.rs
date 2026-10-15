use crate::domain::{TaskDefinition, TaskStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A spec's executable contract (`specs/<slug>/tasks.json`). Fields the
/// coordinator does not use, such as scenarios or evidence, are ignored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskManifest {
    pub spec_id: String,
    pub title: String,
    /// Revision of the spec, not a format version.
    pub version: u32,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    pub tasks: Vec<TaskDefinition>,
    /// Notes from loading, such as legacy profiles translated to tiers.
    #[serde(skip)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Milestone {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub task_ids: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("spec_id cannot be empty")]
    EmptySpecId,
    #[error("manifest must contain at least one task")]
    EmptyTasks,
    #[error("task id cannot be empty")]
    EmptyTaskId,
    #[error("duplicate task id: {0}")]
    DuplicateTaskId(String),
    #[error("task {task_id} depends on missing task {dependency}")]
    MissingDependency { task_id: String, dependency: String },
    #[error("dependency cycle includes task {0}")]
    DependencyCycle(String),
    #[error("task {task_id} requires a title, prompt, cwd, acceptance and verify command")]
    MissingExecutionContract { task_id: String },
    #[error("task {task_id} uses sandbox {kind}; RouDex only runs readOnly or workspaceWrite inside the project")]
    UnsafeSandbox { task_id: String, kind: String },
    #[error("task {task_id} needs writable roots for a workspaceWrite sandbox")]
    MissingWritableRoots { task_id: String },
    #[error("milestone {milestone} of task {task_id} is not in the manifest")]
    UnknownMilestone { task_id: String, milestone: String },
    #[error("milestone {milestone} lists missing task {task_id}")]
    MilestoneTask { milestone: String, task_id: String },
    #[error("could not read manifest: {0}")]
    Io(String),
    #[error("could not parse manifest: {0}")]
    Parse(String),
}

impl TaskManifest {
    pub fn from_json(contents: &str) -> Result<Self, ManifestError> {
        let value: Value = serde_json::from_str(contents)
            .map_err(|error| ManifestError::Parse(error.to_string()))?;
        let warnings = legacy_warnings(&value);
        let mut manifest: Self = serde_json::from_value(value)
            .map_err(|error| ManifestError::Parse(error.to_string()))?;
        // A manifest never chooses the model: router.toml does at dispatch.
        for task in &mut manifest.tasks {
            task.model.clear();
            task.effort.clear();
        }
        manifest.warnings = warnings;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Load a manifest file. A relative task `cwd` refers to the project
    /// root (see [`TaskManifest::project_root`]) and becomes absolute.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        let contents =
            std::fs::read_to_string(path).map_err(|error| ManifestError::Io(error.to_string()))?;
        let mut manifest = Self::from_json(&contents)?;
        let root = Self::project_root(path);
        for task in &mut manifest.tasks {
            if Path::new(&task.cwd).is_relative() {
                let joined = root.join(&task.cwd);
                task.cwd = crate::chat::project_dir(&joined)
                    .unwrap_or(joined)
                    .to_string_lossy()
                    .into_owned();
            }
        }
        Ok(manifest)
    }

    /// The project a manifest belongs to: the folder holding
    /// `specs/<slug>/tasks.json`, or else the manifest's own folder.
    pub fn project_root(path: &Path) -> PathBuf {
        let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let folder = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        match folder.parent() {
            Some(specs) if specs.file_name().is_some_and(|name| name == "specs") => {
                specs.parent().unwrap_or(specs).to_path_buf()
            }
            _ => folder,
        }
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.spec_id.trim().is_empty() {
            return Err(ManifestError::EmptySpecId);
        }
        if self.tasks.is_empty() {
            return Err(ManifestError::EmptyTasks);
        }

        let mut by_id = HashMap::with_capacity(self.tasks.len());
        for task in &self.tasks {
            if task.id.trim().is_empty() {
                return Err(ManifestError::EmptyTaskId);
            }
            if by_id.insert(task.id.clone(), task).is_some() {
                return Err(ManifestError::DuplicateTaskId(task.id.clone()));
            }
            if task.title.trim().is_empty()
                || task.prompt.trim().is_empty()
                || task.cwd.trim().is_empty()
                || task.acceptance.is_empty()
                || task.verify.trim().is_empty()
            {
                return Err(ManifestError::MissingExecutionContract {
                    task_id: task.id.clone(),
                });
            }
            match task.sandbox.kind.as_str() {
                "readOnly" => {}
                "workspaceWrite" if task.sandbox.writable_roots.is_empty() => {
                    return Err(ManifestError::MissingWritableRoots {
                        task_id: task.id.clone(),
                    });
                }
                "workspaceWrite" => {}
                kind => {
                    return Err(ManifestError::UnsafeSandbox {
                        task_id: task.id.clone(),
                        kind: kind.to_owned(),
                    });
                }
            }
        }

        let milestones: HashSet<&str> = self.milestones.iter().map(|m| m.id.as_str()).collect();
        for task in &self.tasks {
            for dependency in &task.depends_on {
                if dependency == &task.id || !by_id.contains_key(dependency) {
                    return Err(ManifestError::MissingDependency {
                        task_id: task.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
            if let Some(milestone) = &task.milestone {
                if !milestones.is_empty() && !milestones.contains(milestone.as_str()) {
                    return Err(ManifestError::UnknownMilestone {
                        task_id: task.id.clone(),
                        milestone: milestone.clone(),
                    });
                }
            }
        }
        for milestone in &self.milestones {
            if let Some(missing) = milestone
                .task_ids
                .iter()
                .find(|id| !by_id.contains_key(*id))
            {
                return Err(ManifestError::MilestoneTask {
                    milestone: milestone.id.clone(),
                    task_id: missing.clone(),
                });
            }
        }

        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        for task in &self.tasks {
            visit_task(task, &by_id, &mut visiting, &mut visited)?;
        }
        Ok(())
    }

    pub fn ready_tasks(&self) -> Vec<&TaskDefinition> {
        self.tasks
            .iter()
            .filter(|task| {
                matches!(task.status, TaskStatus::Planned | TaskStatus::Ready)
                    && task.depends_on.iter().all(|dependency| {
                        self.tasks.iter().any(|candidate| {
                            candidate.id == *dependency && candidate.status == TaskStatus::Completed
                        })
                    })
            })
            .collect()
    }
}

/// Explain what loading a manifest written before tiers changes.
fn legacy_warnings(value: &Value) -> Vec<String> {
    let Some(tasks) = value["tasks"].as_array() else {
        return Vec::new();
    };
    let mut warnings = Vec::new();
    for task in tasks {
        let id = task["id"].as_str().unwrap_or("?");
        if task.get("tier").is_none_or(Value::is_null) {
            if let Some(profile) = task["profile"].as_str() {
                if let Some(tier) = crate::domain::tier_for_profile(profile) {
                    warnings.push(format!(
                        "{id}: el perfil {profile} se traduce al tier {tier}"
                    ));
                }
            }
        }
        let declared = |field: &str| task[field].as_str().is_some_and(|value| !value.is_empty());
        if declared("model") || declared("effort") {
            warnings.push(format!(
                "{id}: se ignoran el modelo y el esfuerzo declarados; los decide router.toml"
            ));
        }
    }
    warnings
}

fn visit_task(
    task: &TaskDefinition,
    by_id: &HashMap<String, &TaskDefinition>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> Result<(), ManifestError> {
    if visited.contains(&task.id) {
        return Ok(());
    }
    if !visiting.insert(task.id.clone()) {
        return Err(ManifestError::DependencyCycle(task.id.clone()));
    }
    for dependency in &task.depends_on {
        visit_task(by_id[dependency], by_id, visiting, visited)?;
    }
    visiting.remove(&task.id);
    visited.insert(task.id.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TaskSandbox;
    use crate::tiers::Tier;
    use serde_json::json;

    fn task(id: &str, depends_on: Vec<&str>) -> TaskDefinition {
        TaskDefinition {
            id: id.to_owned(),
            title: id.to_owned(),
            prompt: format!("Implement {id}"),
            depends_on: depends_on.into_iter().map(ToOwned::to_owned).collect(),
            status: if id == "T-001" {
                TaskStatus::Completed
            } else {
                TaskStatus::Planned
            },
            tier: Tier::Economico,
            profile: None,
            model: String::new(),
            effort: String::new(),
            milestone: None,
            approval: "manual".to_owned(),
            scope: vec!["crates/router-core".to_owned()],
            cwd: ".".to_owned(),
            sandbox: TaskSandbox {
                kind: "workspaceWrite".to_owned(),
                writable_roots: vec!["crates/router-core".to_owned()],
            },
            acceptance: vec!["test passes".to_owned()],
            verify: "cargo test".to_owned(),
        }
    }

    fn manifest(tasks: Vec<TaskDefinition>) -> TaskManifest {
        TaskManifest {
            spec_id: "spec".to_owned(),
            title: "Spec".to_owned(),
            version: 1,
            milestones: Vec::new(),
            tasks,
            warnings: Vec::new(),
        }
    }

    fn task_json(extra: Value) -> Value {
        let mut task = json!({
            "id": "T-1", "title": "Title", "prompt": "Do it", "depends_on": [],
            "status": "planned", "approval": "not_required", "scope": ["src"], "cwd": ".",
            "sandbox": { "type": "workspaceWrite", "writable_roots": ["src"] },
            "acceptance": ["works"], "verify": "cargo test"
        });
        for (key, value) in extra.as_object().unwrap() {
            task[key] = value.clone();
        }
        task
    }

    fn load(tasks: Vec<Value>) -> Result<TaskManifest, ManifestError> {
        TaskManifest::from_json(
            &json!({ "spec_id": "s", "title": "S", "version": 3, "tasks": tasks }).to_string(),
        )
    }

    #[test]
    fn accepts_valid_graph_and_finds_ready_dependents() {
        let value = manifest(vec![task("T-001", vec![]), task("T-002", vec!["T-001"])]);
        value.validate().expect("valid manifest");
        assert_eq!(value.ready_tasks().len(), 1);
        assert_eq!(value.ready_tasks()[0].id, "T-002");
    }

    #[test]
    fn rejects_duplicate_ids_and_cycles() {
        let duplicate = manifest(vec![task("T-001", vec![]), task("T-001", vec![])]);
        assert!(matches!(
            duplicate.validate(),
            Err(ManifestError::DuplicateTaskId(_))
        ));

        let cycle = manifest(vec![
            task("T-001", vec!["T-002"]),
            task("T-002", vec!["T-001"]),
        ]);
        assert!(matches!(
            cycle.validate(),
            Err(ManifestError::DependencyCycle(_))
        ));
    }

    #[test]
    fn tasks_declare_a_tier_and_never_a_model() {
        let loaded = load(vec![task_json(
            json!({ "tier": "fuerte", "model": "gpt-6-astra", "effort": "high" }),
        )])
        .unwrap();
        let task = &loaded.tasks[0];
        assert_eq!(task.tier, Tier::Fuerte);
        assert!(task.model.is_empty() && task.effort.is_empty());
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
        assert!(loaded.warnings[0].contains("router.toml"));
        assert_eq!(loaded.version, 3, "version is the spec revision");
    }

    #[test]
    fn legacy_profiles_map_to_tiers_with_a_warning() {
        let cases = [
            ("micro", Tier::Economico),
            ("routine", Tier::Economico),
            ("standard", Tier::Estandar),
            ("deep", Tier::Fuerte),
            ("frontier", Tier::Fuerte),
        ];
        for (profile, tier) in cases {
            let loaded = load(vec![task_json(json!({ "profile": profile }))]).unwrap();
            assert_eq!(loaded.tasks[0].tier, tier, "{profile}");
            assert_eq!(loaded.tasks[0].profile.as_deref(), Some(profile));
            assert!(loaded.warnings[0].contains(&format!("tier {tier}")));
        }
        // An explicit tier wins over a profile and needs no translation.
        let both = load(vec![task_json(
            json!({ "tier": "economico", "profile": "deep" }),
        )])
        .unwrap();
        assert_eq!(both.tasks[0].tier, Tier::Economico);
        assert!(both.warnings.is_empty());
    }

    #[test]
    fn unknown_tiers_profiles_and_missing_tiers_are_rejected() {
        for extra in [
            json!({ "tier": "premium" }),
            json!({ "profile": "turbo" }),
            json!({}),
        ] {
            let error = load(vec![task_json(extra.clone())]).unwrap_err();
            assert!(matches!(error, ManifestError::Parse(_)), "{extra}: {error}");
        }
    }

    #[test]
    fn sandboxes_stay_inside_the_project() {
        let danger = load(vec![task_json(json!({ "tier": "fuerte",
            "sandbox": { "type": "dangerFullAccess", "writable_roots": [] } }))]);
        assert!(matches!(danger, Err(ManifestError::UnsafeSandbox { .. })));
        let no_roots = load(vec![task_json(json!({ "tier": "fuerte",
            "sandbox": { "type": "workspaceWrite", "writable_roots": [] } }))]);
        assert!(matches!(
            no_roots,
            Err(ManifestError::MissingWritableRoots { .. })
        ));
        let read_only = load(vec![task_json(json!({ "tier": "fuerte",
            "sandbox": { "type": "readOnly", "writable_roots": [] } }))]);
        assert!(read_only.is_ok());
    }

    #[test]
    fn milestones_must_match_their_tasks() {
        let with = |milestones: Value, milestone: &str| {
            TaskManifest::from_json(
                &json!({ "spec_id": "s", "title": "S", "version": 1, "milestones": milestones,
                    "tasks": [task_json(json!({ "tier": "estandar", "milestone": milestone }))] })
                .to_string(),
            )
        };
        let ok = with(json!([{ "id": "H1", "task_ids": ["T-1"] }]), "H1").unwrap();
        assert_eq!(ok.tasks[0].milestone.as_deref(), Some("H1"));
        assert!(matches!(
            with(json!([{ "id": "H1", "task_ids": ["T-1"] }]), "H9"),
            Err(ManifestError::UnknownMilestone { .. })
        ));
        assert!(matches!(
            with(json!([{ "id": "H1", "task_ids": ["T-7"] }]), "H1"),
            Err(ManifestError::MilestoneTask { .. })
        ));
    }

    #[test]
    fn relative_cwds_resolve_against_the_project() {
        let root = std::env::temp_dir().join(format!("roudex-project-{}", uuid::Uuid::new_v4()));
        let spec_dir = root.join("specs").join("001-demo");
        std::fs::create_dir_all(&spec_dir).unwrap();
        let manifest = json!({ "spec_id": "s", "title": "S", "version": 1,
            "tasks": [task_json(json!({ "tier": "estandar" }))] });
        let in_specs = spec_dir.join("tasks.json");
        std::fs::write(&in_specs, manifest.to_string()).unwrap();
        let elsewhere = root.join("tasks.json");
        std::fs::write(&elsewhere, manifest.to_string()).unwrap();

        let project = crate::chat::project_dir(&root).unwrap();
        assert_eq!(
            TaskManifest::project_root(&in_specs),
            std::fs::canonicalize(&root).unwrap()
        );
        let loaded = TaskManifest::load(&in_specs).unwrap();
        assert_eq!(Path::new(&loaded.tasks[0].cwd), project);
        let loaded = TaskManifest::load(&elsewhere).unwrap();
        assert_eq!(Path::new(&loaded.tasks[0].cwd), project);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_repository_specs_load() {
        let hook =
            TaskManifest::from_json(include_str!("../../../specs/003-desktop-hook/tasks.json"))
                .unwrap();
        assert_eq!(hook.tasks.len(), 5);
        assert!(hook.tasks.iter().all(|task| task.tier == Tier::Estandar));
        assert!(!hook.warnings.is_empty());
        let orchestrator = TaskManifest::from_json(include_str!(
            "../../../specs/004-sdd-orchestrator/tasks.json"
        ))
        .unwrap();
        assert_eq!(orchestrator.tasks.len(), 7);
        assert_eq!(orchestrator.milestones[0].task_ids.len(), 7);
        assert!(orchestrator.warnings.is_empty());
        let example = TaskManifest::from_json(include_str!(
            "../../../examples/hello-orchestrator/specs/001-notes/tasks.json"
        ))
        .unwrap();
        assert_eq!(example.tasks.len(), 2);
        assert!(example
            .tasks
            .iter()
            .all(|task| task.tier == Tier::Economico));
        assert!(example.warnings.is_empty());
    }
}
