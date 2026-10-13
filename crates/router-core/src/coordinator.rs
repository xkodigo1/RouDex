use crate::app_server::{AppServerClient, AppServerConfig, AppServerTurn};
use crate::domain::{
    EventRecord, RoutePromptInput, RouteReceipt, RouteStatus, TaskDefinition, TaskStatus,
};
use crate::persistence::{RouterStore, StoreError};
use crate::routing::resolve_profile;
use crate::task_manifest::{ManifestError, TaskManifest};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    fn resume_thread(
        &mut self,
        _task: &TaskDefinition,
        _thread_id: &str,
    ) -> Result<Option<RuntimeTurn>, String> {
        Err("runtime does not support thread resume".into())
    }
    /// Resume only when history confirms the persisted turn identity. Runtimes
    /// that return no completed turn must override this to validate interrupted history.
    fn resume_thread_for_turn(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
        expected_turn_id: Option<&str>,
    ) -> Result<Option<RuntimeTurn>, String> {
        let turn = self.resume_thread(task, thread_id)?;
        if expected_turn_id.is_some_and(|expected| {
            turn.as_ref().map(|turn| turn.turn_id.as_str()) != Some(expected)
        }) {
            return Err("runtime cannot confirm the persisted turn identity".into());
        }
        Ok(turn)
    }
    fn run_turn_with_evidence(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
        _evidence: &mut dyn FnMut(&Value) -> Result<(), String>,
    ) -> Result<RuntimeTurn, String> {
        self.run_turn(task, thread_id)
    }
    fn verify(&mut self, _task: &TaskDefinition) -> Result<Value, String> {
        Err("runtime did not supply independent verification evidence".into())
    }
}

pub struct LocalTaskRuntime {
    config: AppServerConfig,
    client: Option<AppServerClient>,
}

impl LocalTaskRuntime {
    pub fn start(task: &TaskDefinition) -> Result<Self, CoordinatorError> {
        let config = AppServerConfig::local(&task.cwd).with_active_task(&task.id);
        Ok(Self::with_config(config))
    }

    pub fn with_config(config: AppServerConfig) -> Self {
        Self {
            config,
            client: None,
        }
    }

    fn client(&mut self) -> Result<&mut AppServerClient, String> {
        if self.client.is_none() {
            self.client =
                Some(AppServerClient::start(&self.config).map_err(|error| error.to_string())?);
        }
        Ok(self.client.as_mut().expect("client initialized"))
    }
}

impl TaskRuntime for LocalTaskRuntime {
    fn start_thread(&mut self, task: &TaskDefinition) -> Result<String, String> {
        self.client()?
            .start_task_thread(task)
            .map_err(|error| error.to_string())
    }

    fn run_turn(&mut self, task: &TaskDefinition, thread_id: &str) -> Result<RuntimeTurn, String> {
        self.run_turn_with_evidence(task, thread_id, &mut |_| Ok(()))
    }

    fn resume_thread(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
    ) -> Result<Option<RuntimeTurn>, String> {
        self.resume_thread_for_turn(task, thread_id, None)
    }

    fn resume_thread_for_turn(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
        expected_turn_id: Option<&str>,
    ) -> Result<Option<RuntimeTurn>, String> {
        self.client()?
            .resume_task_thread_for_turn(task, thread_id, expected_turn_id)
            .map(|turn| turn.map(runtime_turn))
            .map_err(|error| error.to_string())
    }

    fn run_turn_with_evidence(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
        evidence: &mut dyn FnMut(&Value) -> Result<(), String>,
    ) -> Result<RuntimeTurn, String> {
        self.client()?
            .run_task_turn(task, thread_id, evidence)
            .map(runtime_turn)
            .map_err(|error| error.to_string())
    }

    fn verify(&mut self, task: &TaskDefinition) -> Result<Value, String> {
        self.client()?
            .verify_task(task)
            .map_err(|error| error.to_string())
    }
}

pub struct TaskCoordinator<'a> {
    store: &'a RouterStore,
}

// Batch dispatch must check persisted gates before even constructing a runtime.
// Keep the factory lazy so an implementation that spawns in its constructor is safe.
struct DeferredRuntime<'a, R, F> {
    factory: &'a mut F,
    runtime: Option<R>,
}

impl<R: TaskRuntime, F: FnMut(&TaskDefinition) -> Result<R, CoordinatorError>>
    DeferredRuntime<'_, R, F>
{
    fn runtime(&mut self, task: &TaskDefinition) -> Result<&mut R, String> {
        if self.runtime.is_none() {
            self.runtime = Some((self.factory)(task).map_err(|error| error.to_string())?);
        }
        Ok(self.runtime.as_mut().expect("runtime initialized"))
    }
}

impl<R: TaskRuntime, F: FnMut(&TaskDefinition) -> Result<R, CoordinatorError>> TaskRuntime
    for DeferredRuntime<'_, R, F>
{
    fn start_thread(&mut self, task: &TaskDefinition) -> Result<String, String> {
        self.runtime(task)?.start_thread(task)
    }

    fn run_turn(&mut self, task: &TaskDefinition, thread_id: &str) -> Result<RuntimeTurn, String> {
        self.runtime(task)?.run_turn(task, thread_id)
    }

    fn run_turn_with_evidence(
        &mut self,
        task: &TaskDefinition,
        thread_id: &str,
        evidence: &mut dyn FnMut(&Value) -> Result<(), String>,
    ) -> Result<RuntimeTurn, String> {
        self.runtime(task)?
            .run_turn_with_evidence(task, thread_id, evidence)
    }

    fn verify(&mut self, task: &TaskDefinition) -> Result<Value, String> {
        self.runtime(task)?.verify(task)
    }
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

        // Preserve terminal and active tasks even if their dependencies have changed.
        if !matches!(
            task.status,
            TaskStatus::Planned
                | TaskStatus::Ready
                | TaskStatus::Blocked
                | TaskStatus::AwaitingApproval
        ) {
            return self.existing_receipt(&task, "task is not ready for dispatch");
        }
        // Desktop drafts deliberately omit the execution contract. Approval
        // alone cannot make such a draft executable.
        if task.verify.trim().is_empty() || task.acceptance.is_empty() || task.scope.is_empty() {
            if task.status != TaskStatus::Blocked {
                self.store.set_task_status(task_id, TaskStatus::Blocked)?;
            }
            let mut draft = task.clone();
            draft.status = TaskStatus::Blocked;
            return self.existing_receipt(
                &draft,
                "task requires scope, acceptance and verification before dispatch",
            );
        }
        if task.status == TaskStatus::Blocked {
            if let Some(run) = self.store.latest_run_for_task(task_id)? {
                if self
                    .store
                    .events_for_run(&run.run_id)?
                    .iter()
                    .any(|event| event.event_type == "thread.resume.failed")
                {
                    return self.existing_receipt(
                        &task,
                        "thread recovery is blocked; explicit recovery required",
                    );
                }
            }
        }
        if !self.store.dependencies_completed(task_id)? {
            if matches!(
                task.status,
                TaskStatus::Planned | TaskStatus::Ready | TaskStatus::AwaitingApproval
            ) {
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

        if task.approval != "not_required"
            && self
                .store
                .latest_task_approval(task_id)?
                .is_some_and(|approval| approval.decision == "denied")
        {
            if task.status != TaskStatus::Blocked {
                self.store.set_task_status(task_id, TaskStatus::Blocked)?;
            }
            let mut blocked = task.clone();
            blocked.status = TaskStatus::Blocked;
            return self.existing_receipt(&blocked, "manual approval denied");
        }

        let current = task.status.clone();
        if matches!(current, TaskStatus::Planned | TaskStatus::Blocked) {
            self.store.set_task_status(task_id, TaskStatus::Ready)?;
        } else if current == TaskStatus::AwaitingApproval {
            if self
                .store
                .latest_task_approval(task_id)?
                .is_some_and(|approval| approval.decision == "approved")
            {
                self.store.set_task_status(task_id, TaskStatus::Ready)?;
            } else {
                return self.existing_receipt(&task, "explicit approval required");
            }
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

        let Some(route) = resolve_profile(&task.profile) else {
            return self.block_invalid_route(&task, &stored.spec_id, "unknown routing profile");
        };
        if route.model != task.model || route.effort != task.effort {
            return self.block_invalid_route(
                &task,
                &stored.spec_id,
                &format!(
                    "profile {} resolves to {}/{}, manifest has {}/{}",
                    task.profile, route.model, route.effort, task.model, task.effort
                ),
            );
        }

        let route_id = Uuid::new_v4().to_string();
        let run_id = Uuid::new_v4().to_string();
        if !self.store.begin_task_run(
            &task,
            &stored.spec_id,
            &RouteReceipt {
                route_id: route_id.clone(),
                run_id: run_id.clone(),
                profile: route.profile.to_owned(),
                model: route.model.to_owned(),
                effort: route.effort.to_owned(),
                status: RouteStatus::Accepted,
            },
        )? {
            let task = self
                .store
                .get_task(task_id)?
                .expect("task exists")
                .definition;
            return self.existing_receipt(&task, "dispatch was claimed or a gate changed");
        }

        let thread_id = match runtime.start_thread(&task) {
            Ok(thread_id) if !thread_id.is_empty() => thread_id,
            Ok(_) => {
                let error = "runtime returned an empty thread id".to_owned();
                self.fail_task(&task.id, &run_id, "thread.start.failed", &error)?;
                return Ok(failed_receipt(task.id, route_id, run_id, error));
            }
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

        self.execute_turn(&task, &route_id, &run_id, &thread_id, runtime, None)
    }

    fn execute_turn<R: TaskRuntime>(
        &self,
        task: &TaskDefinition,
        route_id: &str,
        run_id: &str,
        thread_id: &str,
        runtime: &mut R,
        recovered: Option<RuntimeTurn>,
    ) -> Result<TaskExecutionReceipt, CoordinatorError> {
        let mut evidence = |message: &Value| {
            self.store
                .record_event(&EventRecord {
                    run_id: run_id.to_owned(),
                    event_type: message["method"]
                        .as_str()
                        .unwrap_or("runtime.event")
                        .to_owned(),
                    payload: message["params"].clone(),
                })
                .map_err(|error| error.to_string())
        };
        let turn = match recovered
            .map(Ok)
            .unwrap_or_else(|| runtime.run_turn_with_evidence(task, thread_id, &mut evidence))
        {
            Ok(turn) => turn,
            Err(error) => {
                self.store.finish_task_run(
                    &task.id,
                    run_id,
                    TaskStatus::Interrupted,
                    Some(&EventRecord {
                        run_id: run_id.into(),
                        event_type: "turn.error".into(),
                        payload: json!({ "threadId": thread_id, "reason": error }),
                    }),
                )?;
                return Ok(TaskExecutionReceipt {
                    task_id: task.id.clone(),
                    route_id: Some(route_id.into()),
                    run_id: Some(run_id.into()),
                    thread_id: Some(thread_id.into()),
                    status: TaskStatus::Interrupted,
                    text: Some(error),
                });
            }
        };
        if turn.thread_id != thread_id
            || turn.turn_id.is_empty()
            || !matches!(turn.status.as_str(), "completed" | "failed" | "interrupted")
        {
            let error = "runtime returned a different thread, empty turn id or non-terminal status";
            self.fail_task(&task.id, run_id, "turn.invalid", error)?;
            return Ok(failed_receipt_with_thread(
                task.id.clone(),
                route_id.into(),
                run_id.into(),
                thread_id.into(),
                error.into(),
            ));
        }
        self.store.record_terminal_event(&EventRecord {
            run_id: run_id.to_owned(),
            event_type: "turn.completed".to_owned(),
            payload: json!({ "taskId": task.id, "threadId": turn.thread_id, "turnId": turn.turn_id, "status": turn.status, "text": turn.text }),
        })?;
        if turn.status == "completed" {
            let verification = match runtime.verify(task) {
                Ok(result) => result,
                Err(error) => {
                    self.fail_task(&task.id, run_id, "verification.failed", &error)?;
                    return Ok(failed_receipt_with_thread(
                        task.id.clone(),
                        route_id.into(),
                        run_id.into(),
                        thread_id.into(),
                        error,
                    ));
                }
            };
            if verification["exitCode"].as_i64() != Some(0) {
                let reason = "task verification failed or returned no exit code";
                self.store.finish_task_run(
                    &task.id,
                    run_id,
                    TaskStatus::Failed,
                    Some(&EventRecord {
                        run_id: run_id.into(),
                        event_type: "verification.failed".into(),
                        payload: json!({ "taskId": task.id, "reason": reason,
                        "command": task.verify, "result": verification }),
                    }),
                )?;
                return Ok(failed_receipt_with_thread(
                    task.id.clone(),
                    route_id.into(),
                    run_id.into(),
                    thread_id.into(),
                    reason.into(),
                ));
            }
            self.store.finish_task_run(
                &task.id,
                run_id,
                TaskStatus::Completed,
                Some(&EventRecord {
                    run_id: run_id.into(),
                    event_type: "task.verified".into(),
                    payload: json!({ "command": task.verify, "result": verification }),
                }),
            )?;
            Ok(TaskExecutionReceipt {
                task_id: task.id.clone(),
                route_id: Some(route_id.into()),
                run_id: Some(run_id.into()),
                thread_id: Some(turn.thread_id),
                status: TaskStatus::Completed,
                text: Some(turn.text),
            })
        } else if turn.status == "interrupted" {
            self.store
                .finish_task_run(&task.id, run_id, TaskStatus::Interrupted, None)?;
            Ok(TaskExecutionReceipt {
                task_id: task.id.clone(),
                route_id: Some(route_id.into()),
                run_id: Some(run_id.into()),
                thread_id: Some(thread_id.into()),
                status: TaskStatus::Interrupted,
                text: Some(turn.text),
            })
        } else {
            self.fail_task(&task.id, run_id, "turn.incomplete", &turn.status)?;
            Ok(failed_receipt_with_thread(
                task.id.clone(),
                route_id.into(),
                run_id.into(),
                turn.thread_id,
                turn.status,
            ))
        }
    }

    fn existing_receipt(
        &self,
        task: &TaskDefinition,
        reason: &str,
    ) -> Result<TaskExecutionReceipt, CoordinatorError> {
        let run = self.store.latest_run_for_task(&task.id)?;
        let thread_id = match &run {
            Some(run) => self.store.last_thread_id(&run.run_id)?,
            None => None,
        };
        Ok(TaskExecutionReceipt {
            task_id: task.id.clone(),
            route_id: run.as_ref().map(|run| run.route_id.clone()),
            run_id: run.map(|run| run.run_id),
            thread_id,
            status: task.status.clone(),
            text: Some(reason.into()),
        })
    }

    fn block_invalid_route(
        &self,
        task: &TaskDefinition,
        spec_id: &str,
        reason: &str,
    ) -> Result<TaskExecutionReceipt, CoordinatorError> {
        let route_id = Uuid::new_v4().to_string();
        let run_id = Uuid::new_v4().to_string();
        self.store.record_route(
            &RoutePromptInput {
                prompt: task.prompt.clone(),
                session_id: spec_id.into(),
                turn_id: task.id.clone(),
                cwd: task.cwd.clone(),
                permission_mode: Some(task.approval.clone()),
            },
            &RouteReceipt {
                route_id: route_id.clone(),
                run_id: run_id.clone(),
                profile: task.profile.clone(),
                model: task.model.clone(),
                effort: task.effort.clone(),
                status: RouteStatus::Blocked,
            },
        )?;
        self.store.set_task_status(&task.id, TaskStatus::Blocked)?;
        self.store.record_event(&EventRecord {
            run_id: run_id.clone(),
            event_type: "task.blocked".into(),
            payload: json!({ "reason": reason }),
        })?;
        Ok(TaskExecutionReceipt {
            task_id: task.id.clone(),
            route_id: Some(route_id),
            run_id: Some(run_id),
            thread_id: None,
            status: TaskStatus::Blocked,
            text: Some(reason.into()),
        })
    }

    /// Execute eligible tasks in dependency order, refreshing persisted readiness after each run.
    pub fn dispatch_ready_tasks(
        &self,
        spec_id: &str,
    ) -> Result<Vec<TaskExecutionReceipt>, CoordinatorError> {
        self.dispatch_ready_tasks_with_runtime(spec_id, LocalTaskRuntime::start)
    }

    /// Supply a fresh runtime for each eligible task; gates are checked before it starts.
    pub fn dispatch_ready_tasks_with_runtime<R, F>(
        &self,
        spec_id: &str,
        mut runtime_for: F,
    ) -> Result<Vec<TaskExecutionReceipt>, CoordinatorError>
    where
        R: TaskRuntime,
        F: FnMut(&TaskDefinition) -> Result<R, CoordinatorError>,
    {
        let mut receipts = Vec::new();
        let mut visited = std::collections::HashSet::new();
        loop {
            let mut next = None;
            for task in self.store.tasks_for_spec(spec_id)? {
                if !visited.contains(&task.id)
                    && matches!(
                        task.status,
                        TaskStatus::Planned
                            | TaskStatus::Ready
                            | TaskStatus::Blocked
                            | TaskStatus::AwaitingApproval
                    )
                    && self.store.dependencies_completed(&task.id)?
                {
                    next = Some(task);
                    break;
                }
            }
            let Some(task) = next else {
                break;
            };
            visited.insert(task.id.clone());
            let mut runtime = DeferredRuntime {
                factory: &mut runtime_for,
                runtime: None,
            };
            receipts.push(self.dispatch_task(&task.id, &mut runtime)?);
        }
        Ok(receipts)
    }

    pub fn resume_run<R: TaskRuntime>(
        &self,
        run_id: &str,
        runtime: &mut R,
    ) -> Result<TaskExecutionReceipt, CoordinatorError> {
        let run = self
            .store
            .get_run(run_id)?
            .ok_or_else(|| CoordinatorError::Runtime(format!("run not found: {run_id}")))?;
        let stored = self
            .store
            .get_task(&run.turn_id)?
            .ok_or_else(|| CoordinatorError::TaskNotFound(run.turn_id.clone()))?;
        if self
            .store
            .latest_run_for_task(&stored.definition.id)?
            .is_some_and(|latest| latest.run_id != run_id)
        {
            return Err(CoordinatorError::Runtime(
                "run was superseded by a later dispatch; resume the latest run instead".into(),
            ));
        }
        if run.status != "interrupted" || stored.definition.status != TaskStatus::Interrupted {
            return self.existing_receipt(&stored.definition, "run is not interrupted");
        }
        let events = self.store.events_for_run(run_id)?;
        let task: Option<TaskDefinition> = events
            .iter()
            .find(|event| event.event_type == "task.started")
            .and_then(|event| event.payload.get("definition"))
            .and_then(|definition| serde_json::from_value(definition.clone()).ok());
        let Some(task) = task.filter(|task| task.id == stored.definition.id) else {
            return self.block_resume(
                &stored.definition,
                run_id,
                "run has no valid immutable dispatch definition; explicit recovery required",
            );
        };
        let Some(thread_id) = self
            .store
            .last_thread_id(run_id)?
            .filter(|id| !id.is_empty())
        else {
            return self.block_resume(
                &task,
                run_id,
                "run has no persisted thread; explicit recovery required",
            );
        };
        let latest_turn_id = events.iter().rev().find_map(|event| {
            if event.event_type == "turn/started"
                && event.payload["threadId"].as_str() == Some(thread_id.as_str())
            {
                event.payload["turn"]["id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
            } else {
                None
            }
        });
        let Some(latest_turn_id) = latest_turn_id else {
            return self.block_resume(
                &task,
                run_id,
                "run has no persisted turn identity; explicit recovery required",
            );
        };
        if task.approval != "not_required"
            && !self
                .store
                .latest_task_approval(&task.id)?
                .is_some_and(|approval| approval.decision == "approved")
        {
            return self.existing_receipt(&stored.definition, "resume requires approval");
        }
        if !self.store.dependencies_completed(&task.id)? {
            return self.block_resume(
                &task,
                run_id,
                "resume has incomplete dependencies; explicit recovery required",
            );
        }
        if !self.store.begin_resume(&task.id, run_id)? {
            let task = self
                .store
                .get_task(&task.id)?
                .expect("task exists")
                .definition;
            return self.existing_receipt(&task, "resume was already claimed");
        }
        let recovered =
            match runtime.resume_thread_for_turn(&task, &thread_id, Some(latest_turn_id)) {
                Ok(turn) => turn,
                Err(error) => {
                    return self.block_resume(&task, run_id, &error);
                }
            };
        self.store.record_event(&EventRecord {
            run_id: run_id.into(),
            event_type: "thread.resumed".into(),
            payload: json!({ "threadId": thread_id }),
        })?;
        // A crash can happen after the terminal event is durable but before the
        // verification/status update. Never replay that turn, even if the server
        // history is incomplete. Only use evidence for the latest dispatched turn.
        let persisted = events.iter().rev().find_map(|event| {
            let payload = &event.payload;
            let (turn_id, status, text) = match event.event_type.as_str() {
                "turn.completed" => (
                    payload["turnId"].as_str()?,
                    payload["status"].as_str()?,
                    payload["text"].as_str().unwrap_or_default(),
                ),
                "turn/completed" => (
                    payload["turn"]["id"].as_str()?,
                    payload["turn"]["status"].as_str()?,
                    "",
                ),
                _ => return None,
            };
            if payload["threadId"].as_str() != Some(thread_id.as_str())
                || latest_turn_id != turn_id
                || !matches!(status, "completed" | "failed")
            {
                return None;
            }
            Some(RuntimeTurn {
                thread_id: thread_id.clone(),
                turn_id: turn_id.into(),
                status: status.into(),
                text: text.into(),
            })
        });
        let recovered = persisted.or(recovered);
        if recovered
            .as_ref()
            .is_some_and(|turn| latest_turn_id != turn.turn_id)
        {
            let reason =
                "resumed history does not match the persisted turn; explicit recovery required";
            return self.block_resume(&task, run_id, reason);
        }
        self.execute_turn(&task, &run.route_id, run_id, &thread_id, runtime, recovered)
    }

    fn block_resume(
        &self,
        task: &TaskDefinition,
        run_id: &str,
        reason: &str,
    ) -> Result<TaskExecutionReceipt, CoordinatorError> {
        self.store.finish_task_run(
            &task.id,
            run_id,
            TaskStatus::Blocked,
            Some(&EventRecord {
                run_id: run_id.into(),
                event_type: "thread.resume.failed".into(),
                payload: json!({ "taskId": task.id, "reason": reason }),
            }),
        )?;
        let mut blocked = task.clone();
        blocked.status = TaskStatus::Blocked;
        self.existing_receipt(&blocked, reason)
    }

    fn fail_task(
        &self,
        task_id: &str,
        run_id: &str,
        event_type: &str,
        reason: &str,
    ) -> Result<(), CoordinatorError> {
        self.store.finish_task_run(
            task_id,
            run_id,
            TaskStatus::Failed,
            Some(&EventRecord {
                run_id: run_id.to_owned(),
                event_type: event_type.to_owned(),
                payload: json!({ "taskId": task_id, "reason": reason }),
            }),
        )?;
        Ok(())
    }

    pub fn recover_running_task(
        &self,
        task_id: &str,
        reason: &str,
    ) -> Result<TaskExecutionReceipt, CoordinatorError> {
        let task = self
            .store
            .get_task(task_id)?
            .ok_or_else(|| CoordinatorError::TaskNotFound(task_id.to_owned()))?;
        if task.definition.status != TaskStatus::Running {
            return Ok(TaskExecutionReceipt {
                task_id: task_id.to_owned(),
                route_id: None,
                run_id: None,
                thread_id: None,
                status: task.definition.status,
                text: Some("task is not running".to_owned()),
            });
        }
        let run = self.store.latest_run_for_task(task_id)?;
        if let Some(run) = &run {
            self.store.finish_task_run(
                task_id,
                &run.run_id,
                TaskStatus::Interrupted,
                Some(&EventRecord {
                    run_id: run.run_id.clone(),
                    event_type: "task.recovered".to_owned(),
                    payload: json!({ "taskId": task_id, "reason": reason }),
                }),
            )?;
        } else {
            self.store
                .set_task_status(task_id, TaskStatus::Interrupted)?;
        }
        let thread_id = match &run {
            Some(run) => self.store.last_thread_id(&run.run_id)?,
            None => None,
        };
        Ok(TaskExecutionReceipt {
            task_id: task_id.to_owned(),
            route_id: run.as_ref().map(|run| run.route_id.clone()),
            run_id: run.as_ref().map(|run| run.run_id.clone()),
            thread_id,
            status: TaskStatus::Interrupted,
            text: Some(reason.to_owned()),
        })
    }

    pub fn retry_interrupted_task(
        &self,
        task_id: &str,
    ) -> Result<TaskExecutionReceipt, CoordinatorError> {
        let task = self
            .store
            .get_task(task_id)?
            .ok_or_else(|| CoordinatorError::TaskNotFound(task_id.to_owned()))?;
        if !matches!(
            task.definition.status,
            TaskStatus::Interrupted | TaskStatus::Failed
        ) {
            return Ok(TaskExecutionReceipt {
                task_id: task_id.to_owned(),
                route_id: None,
                run_id: None,
                thread_id: None,
                status: task.definition.status,
                text: Some("task is not retryable".to_owned()),
            });
        }
        if task.definition.status == TaskStatus::Failed {
            self.store.set_task_status(task_id, TaskStatus::Blocked)?;
        }
        self.store.set_task_status(task_id, TaskStatus::Ready)?;
        Ok(TaskExecutionReceipt {
            task_id: task_id.to_owned(),
            route_id: None,
            run_id: None,
            thread_id: None,
            status: TaskStatus::Ready,
            text: Some("task ready for retry".to_owned()),
        })
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
        fn verify(&mut self, _task: &TaskDefinition) -> Result<Value, String> {
            Ok(json!({ "exitCode": 0, "stdout": "verified", "stderr": "" }))
        }
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
