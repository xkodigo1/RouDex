use crate::app_server::{AppServerClient, AppServerConfig, AppServerTurn};
use crate::domain::{
    EventRecord, RoutePromptInput, RouteReceipt, RouteStatus, TaskDefinition, TaskStatus,
};
use crate::persistence::{RouterStore, StoreError};
use crate::routing::resolve_profile;
use crate::task_manifest::{ManifestError, TaskManifest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskExecutionReceipt {
    pub task_id: String,
    pub route_id: Option<String>,
    pub run_id: Option<String>,
    pub thread_id: Option<String>,
    pub status: TaskStatus,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTurn {
    pub thread_id: String,
    pub turn_id: String,
    pub status: String,
    pub text: String,
}

#[derive(Debug, Error)]
pub enum CoordinatorError {
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error("task not found: {0}")]
    TaskNotFound(String),
}

pub trait TaskRuntime {
    fn start_thread(&mut self, task: &TaskDefinition) -> Result<String, String>;
    fn run_turn(&mut self, task: &TaskDefinition, thread_id: &str) -> Result<RuntimeTurn, String>;
}

pub struct LocalTaskRuntime {
    client: AppServerClient,
}

impl LocalTaskRuntime {
    pub fn start(cwd: impl AsRef<Path>) -> Result<Self, CoordinatorError> {
        let client = AppServerClient::start(&AppServerConfig::local(cwd))
            .map_err(|error| CoordinatorError::Runtime(error.to_string()))?;
        Ok(Self { client })
    }
}

impl TaskRuntime for LocalTaskRuntime {
    fn start_thread(&mut self, task: &TaskDefinition) -> Result<String, String> {
        self.client
            .start_thread(
                &task.model,
                Path::new(&task.cwd),
                "never",
                sandbox_name(&task.sandbox.kind),
            )
            .map_err(|error| error.to_string())
    }

    fn run_turn(&mut self, task: &TaskDefinition, thread_id: &str) -> Result<RuntimeTurn, String> {
        let turn = self
            .client
            .run_turn(
                thread_id,
                &task.prompt,
                &task.model,
                &task.effort,
                Path::new(&task.cwd),
                "never",
                sandbox_name(&task.sandbox.kind),
            )
            .map_err(|error| error.to_string())?;
        Ok(runtime_turn(turn))
    }
}

pub struct TaskCoordinator<'a> {
    store: &'a RouterStore,
}

impl<'a> TaskCoordinator<'a> {
    pub fn new(store: &'a RouterStore) -> Self {
        Self { store }
    }

    pub fn import_manifest(&self, manifest: &TaskManifest) -> Result<(), CoordinatorError> {
        manifest.validate()?;
        for task in &manifest.tasks {
            self.store.upsert_task(&manifest.spec_id, task)?;
        }
        Ok(())
    }

    pub fn dispatch_task<R: TaskRuntime>(
        &self,
        task_id: &str,
        runtime: &mut R,
    ) -> Result<TaskExecutionReceipt, CoordinatorError> {
        let stored = self
            .store
            .get_task(task_id)?
            .ok_or_else(|| CoordinatorError::TaskNotFound(task_id.to_owned()))?;
        let task = stored.definition;

        if !self.store.dependencies_completed(task_id)? {
            if matches!(task.status, TaskStatus::Planned | TaskStatus::Ready) {
                self.store.set_task_status(task_id, TaskStatus::Blocked)?;
            }
            return Ok(TaskExecutionReceipt {
                task_id: task.id,
                route_id: None,
                run_id: None,
                thread_id: None,
                status: TaskStatus::Blocked,
                text: Some("task has incomplete dependencies".to_owned()),
            });
        }

        let current = task.status.clone();
        if current == TaskStatus::Planned {
            self.store.set_task_status(task_id, TaskStatus::Ready)?;
        } else if current != TaskStatus::Ready {
            return Ok(TaskExecutionReceipt {
                task_id: task.id,
                route_id: None,
                run_id: None,
                thread_id: None,
                status: current,
                text: Some("task is not ready for dispatch".to_owned()),
            });
        }

        if task.approval != "not_required"
            && self
                .store
                .latest_task_approval(&task.id)?
                .as_ref()
                .map(|approval| approval.decision.as_str())
                != Some("approved")
        {
            self.store
                .set_task_status(&task.id, TaskStatus::AwaitingApproval)?;
            return Ok(TaskExecutionReceipt {
                task_id: task.id,
                route_id: None,
                run_id: None,
                thread_id: None,
                status: TaskStatus::AwaitingApproval,
                text: Some("explicit approval required".to_owned()),
            });
        }

        let route = resolve_profile(&task.profile).ok_or_else(|| {
            CoordinatorError::Runtime(format!("unknown profile: {}", task.profile))
        })?;
        if route.model != task.model || route.effort != task.effort {
            return Err(CoordinatorError::Runtime(format!(
                "profile {} resolves to {}/{}, manifest has {}/{}",
                task.profile, route.model, route.effort, task.model, task.effort
            )));
        }

        let route_id = Uuid::new_v4().to_string();
        let run_id = Uuid::new_v4().to_string();
        self.store.record_route(
            &RoutePromptInput {
                prompt: task.prompt.clone(),
                session_id: stored.spec_id.clone(),
                turn_id: task.id.clone(),
                cwd: task.cwd.clone(),
                permission_mode: Some(task.approval.clone()),
            },
            &RouteReceipt {
                route_id: route_id.clone(),
                run_id: run_id.clone(),
                profile: route.profile.to_owned(),
                model: route.model.to_owned(),
                effort: route.effort.to_owned(),
                status: RouteStatus::Accepted,
            },
        )?;
        self.store.set_task_status(&task.id, TaskStatus::Running)?;
        self.store.record_event(&EventRecord {
            run_id: run_id.clone(),
            event_type: "task.started".to_owned(),
            payload: json!({ "taskId": task.id, "profile": route.profile, "model": route.model, "effort": route.effort }),
        })?;

        let thread_id = match runtime.start_thread(&task) {
            Ok(thread_id) => thread_id,
            Err(error) => {
                self.fail_task(&task.id, &run_id, "thread.start.failed", &error)?;
                return Ok(failed_receipt(task.id, route_id, run_id, error));
            }
        };
        self.store.record_event(&EventRecord {
            run_id: run_id.clone(),
            event_type: "thread.started".to_owned(),
            payload: json!({ "taskId": task.id, "threadId": thread_id }),
        })?;

        let turn = match runtime.run_turn(&task, &thread_id) {
            Ok(turn) => turn,
            Err(error) => {
                self.fail_task(&task.id, &run_id, "turn.failed", &error)?;
                return Ok(failed_receipt_with_thread(
                    task.id, route_id, run_id, thread_id, error,
                ));
            }
        };
        self.store.record_event(&EventRecord {
            run_id: run_id.clone(),
            event_type: "turn.completed".to_owned(),
            payload: json!({ "taskId": task.id, "threadId": turn.thread_id, "turnId": turn.turn_id, "status": turn.status }),
        })?;
        if turn.status == "completed" {
            self.store.update_run_status(&run_id, "completed")?;
            self.store
                .set_task_status(&task.id, TaskStatus::Completed)?;
            Ok(TaskExecutionReceipt {
                task_id: task.id,
                route_id: Some(route_id),
                run_id: Some(run_id),
                thread_id: Some(turn.thread_id),
                status: TaskStatus::Completed,
                text: Some(turn.text),
            })
        } else {
            self.fail_task(&task.id, &run_id, "turn.incomplete", &turn.status)?;
            Ok(failed_receipt_with_thread(
                task.id,
                route_id,
                run_id,
                turn.thread_id,
                turn.status,
            ))
        }
    }

    fn fail_task(
        &self,
        task_id: &str,
        run_id: &str,
        event_type: &str,
        reason: &str,
    ) -> Result<(), CoordinatorError> {
        self.store.update_run_status(run_id, "failed")?;
        self.store.set_task_status(task_id, TaskStatus::Failed)?;
        self.store.record_event(&EventRecord {
            run_id: run_id.to_owned(),
            event_type: event_type.to_owned(),
            payload: json!({ "taskId": task_id, "reason": reason }),
        })?;
        Ok(())
    }
}

fn sandbox_name(kind: &str) -> &str {
    match kind {
        "readOnly" => "read-only",
        "dangerFullAccess" => "danger-full-access",
        _ => "workspace-write",
    }
}

fn runtime_turn(turn: AppServerTurn) -> RuntimeTurn {
    RuntimeTurn {
        thread_id: turn.thread_id,
        turn_id: turn.turn_id,
        status: turn.status,
        text: turn.text,
    }
}

fn failed_receipt(
    task_id: String,
    route_id: String,
    run_id: String,
    error: String,
) -> TaskExecutionReceipt {
    TaskExecutionReceipt {
        task_id,
        route_id: Some(route_id),
        run_id: Some(run_id),
        thread_id: None,
        status: TaskStatus::Failed,
        text: Some(error),
    }
}

fn failed_receipt_with_thread(
    task_id: String,
    route_id: String,
    run_id: String,
    thread_id: String,
    error: String,
) -> TaskExecutionReceipt {
    TaskExecutionReceipt {
        task_id,
        route_id: Some(route_id),
        run_id: Some(run_id),
        thread_id: Some(thread_id),
        status: TaskStatus::Failed,
        text: Some(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{TaskSandbox, TaskStatus};

    struct FakeRuntime {
        starts: usize,
        turns: usize,
    }

    impl TaskRuntime for FakeRuntime {
        fn start_thread(&mut self, _task: &TaskDefinition) -> Result<String, String> {
            self.starts += 1;
            Ok("thread-fake".to_owned())
        }

        fn run_turn(
            &mut self,
            _task: &TaskDefinition,
            thread_id: &str,
        ) -> Result<RuntimeTurn, String> {
            self.turns += 1;
            Ok(RuntimeTurn {
                thread_id: thread_id.to_owned(),
                turn_id: "turn-fake".to_owned(),
                status: "completed".to_owned(),
                text: "done".to_owned(),
            })
        }
    }

    fn task(approval: &str) -> TaskDefinition {
        TaskDefinition {
            id: "T-004".to_owned(),
            title: "Dispatch".to_owned(),
            prompt: "Implement coordinator".to_owned(),
            depends_on: vec![],
            status: TaskStatus::Ready,
            profile: "standard".to_owned(),
            model: "gpt-6.1-sol".to_owned(),
            effort: "medium".to_owned(),
            approval: approval.to_owned(),
            scope: vec!["crates/router-core".to_owned()],
            cwd: ".".to_owned(),
            sandbox: TaskSandbox {
                kind: "workspaceWrite".to_owned(),
                writable_roots: vec!["crates/router-core".to_owned()],
            },
            acceptance: vec!["done".to_owned()],
            verify: "cargo test".to_owned(),
        }
    }

    #[test]
    fn dispatches_with_assigned_route_and_persists_completion() {
        let store = RouterStore::in_memory().expect("store");
        store
            .upsert_task("spec", &task("not_required"))
            .expect("task");
        let mut runtime = FakeRuntime {
            starts: 0,
            turns: 0,
        };
        let receipt = TaskCoordinator::new(&store)
            .dispatch_task("T-004", &mut runtime)
            .expect("dispatch");
        assert_eq!(receipt.status, TaskStatus::Completed);
        assert_eq!(runtime.starts, 1);
        assert_eq!(runtime.turns, 1);
        assert_eq!(store.count_runs().expect("runs"), 1);
    }

    #[test]
    fn manual_task_waits_for_approval_without_runtime_call() {
        let store = RouterStore::in_memory().expect("store");
        store.upsert_task("spec", &task("manual")).expect("task");
        let mut runtime = FakeRuntime {
            starts: 0,
            turns: 0,
        };
        let receipt = TaskCoordinator::new(&store)
            .dispatch_task("T-004", &mut runtime)
            .expect("dispatch");
        assert_eq!(receipt.status, TaskStatus::AwaitingApproval);
        assert_eq!(runtime.starts, 0);
        assert_eq!(store.count_runs().expect("runs"), 0);
    }
}
