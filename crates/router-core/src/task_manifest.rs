use crate::domain::{TaskDefinition, TaskStatus};
use crate::routing::resolve_profile;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskManifest {
    pub spec_id: String,
    pub title: String,
    pub version: u32,
    pub tasks: Vec<TaskDefinition>,
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
    #[error("task {task_id} uses unknown profile {profile}")]
    UnknownProfile { task_id: String, profile: String },
    #[error("task {task_id} profile {profile} resolves to {expected_model}/{expected_effort}, not {model}/{effort}")]
    RouteConflict {
        task_id: String,
        profile: String,
        expected_model: String,
        expected_effort: String,
        model: String,
        effort: String,
    },
    #[error("task {task_id} requires a title, prompt, cwd, acceptance and verify command")]
    MissingExecutionContract { task_id: String },
    #[error("could not read manifest: {0}")]
    Io(String),
    #[error("could not parse manifest: {0}")]
    Parse(String),
}

impl TaskManifest {
    pub fn from_json(contents: &str) -> Result<Self, ManifestError> {
        let manifest: Self = serde_json::from_str(contents)
            .map_err(|error| ManifestError::Parse(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let contents =
            std::fs::read_to_string(path).map_err(|error| ManifestError::Io(error.to_string()))?;
        Self::from_json(&contents)
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
            let route =
                resolve_profile(&task.profile).ok_or_else(|| ManifestError::UnknownProfile {
                    task_id: task.id.clone(),
                    profile: task.profile.clone(),
                })?;
            if route.model != task.model || route.effort != task.effort {
                return Err(ManifestError::RouteConflict {
                    task_id: task.id.clone(),
                    profile: task.profile.clone(),
                    expected_model: route.model.to_owned(),
                    expected_effort: route.effort.to_owned(),
                    model: task.model.clone(),
                    effort: task.effort.clone(),
                });
            }
        }

        for task in &self.tasks {
            for dependency in &task.depends_on {
                if dependency == &task.id || !by_id.contains_key(dependency) {
                    return Err(ManifestError::MissingDependency {
                        task_id: task.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
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
            profile: "routine".to_owned(),
            model: "gpt-6-luna".to_owned(),
            effort: "medium".to_owned(),
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
            tasks,
        }
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
    fn rejects_route_conflicts() {
        let mut invalid = task("T-001", vec![]);
        invalid.model = "gpt-6.1-sol".to_owned();
        assert!(matches!(
            manifest(vec![invalid]).validate(),
            Err(ManifestError::RouteConflict { .. })
        ));
    }
}
