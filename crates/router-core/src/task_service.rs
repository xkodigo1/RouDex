//! What the app does with spec tasks: find a project's specs, load them, show
//! each task with the route it would take, and run tasks in the background
//! while their turns stream to the UI. One run at a time across the app.

use crate::app_server::AppServerConfig;
use crate::classifier::RouteDecision;
use crate::coordinator::{TaskCoordinator, TaskExecutionReceipt};
use crate::domain::{EventRecord, StoredRun, TaskApproval, TaskDefinition};
use crate::persistence::{RouterStore, StoredSpec};
use crate::risk::RiskRules;
use crate::session::{ApprovalDecision, EventSink, SessionEvent};
use crate::task_manifest::TaskManifest;
use crate::task_route::resolve_task_route;
use crate::task_runtime::{LocalTaskRuntime, TaskRuns};
use crate::tiers::{RouterConfig, Tier};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Receives what happens to running tasks.
pub type TaskSink = Arc<dyn Fn(TaskEvent) + Send + Sync>;
/// App-server configuration for a task's working directory.
pub type TaskConfigFactory = Arc<dyn Fn(&Path) -> AppServerConfig + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TaskEvent {
    /// A session event of a running task's turn.
    Session {
        task_id: String,
        event: SessionEvent,
    },
    /// A dispatch or resume ended; the receipt says how.
    Finished {
        task_id: String,
        receipt: TaskExecutionReceipt,
    },
    /// The run could not proceed (store or coordinator error).
    Failed { task_id: String, error: String },
}

/// A `specs/<slug>/tasks.json` found in a project.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecFile {
    pub path: PathBuf,
    pub spec_id: Option<String>,
    pub title: Option<String>,
    pub tasks: usize,
    pub loaded: bool,
    /// Why the manifest cannot be loaded, if it cannot.
    pub error: Option<String>,
}

/// A loaded spec with every task and what would happen if it ran now.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskBoard {
    pub spec: StoredSpec,
    pub tasks: Vec<TaskCard>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCard {
    pub task: TaskDefinition,
    pub tier_override: Option<Tier>,
    /// The route a dispatch would take now.
    pub route: Option<RouteDecision>,
    /// Why no route can be computed (invalid `router.toml` or project rules).
    pub route_error: Option<String>,
    pub dependencies_completed: bool,
    pub approval: Option<TaskApproval>,
    pub latest_run: Option<StoredRun>,
    pub running: bool,
}

pub struct TaskService {
    store_path: PathBuf,
    config_for: TaskConfigFactory,
    router: Option<RouterConfig>,
    runs: Arc<TaskRuns>,
    sink: TaskSink,
    /// What is running now, if anything.
    active: Arc<Mutex<Option<String>>>,
}

impl TaskService {
    pub fn new(
        store_path: impl Into<PathBuf>,
        config_for: TaskConfigFactory,
        sink: TaskSink,
    ) -> Self {
        Self {
            store_path: store_path.into(),
            config_for,
            router: None,
            runs: Arc::new(TaskRuns::default()),
            sink,
            active: Arc::new(Mutex::new(None)),
        }
    }

    /// Use these tier targets instead of reading `router.toml`.
    pub fn with_router(mut self, config: RouterConfig) -> Self {
        self.router = Some(config);
        self
    }

    /// Specs found under `<project>/specs/*/tasks.json`, by folder name.
    pub fn spec_files(&self, project: &Path) -> Result<Vec<SpecFile>, String> {
        let folder = project.join("specs");
        if !folder.is_dir() {
            return Ok(Vec::new());
        }
        let loaded = self
            .store()?
            .specs()
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|spec| spec.spec_id)
            .collect::<Vec<_>>();
        let mut paths = std::fs::read_dir(&folder)
            .map_err(|error| error.to_string())?
            .filter_map(|entry| entry.ok().map(|entry| entry.path().join("tasks.json")))
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        paths.sort();
        Ok(paths
            .into_iter()
            .map(|path| match TaskManifest::load(&path) {
                Ok(manifest) => SpecFile {
                    loaded: loaded.contains(&manifest.spec_id),
                    spec_id: Some(manifest.spec_id),
                    title: Some(manifest.title),
                    tasks: manifest.tasks.len(),
                    error: None,
                    path,
                },
                Err(error) => SpecFile {
                    path,
                    spec_id: None,
                    title: None,
                    tasks: 0,
                    loaded: false,
                    error: Some(error.to_string()),
                },
            })
            .collect())
    }

    /// Import a manifest (again); lifecycle state of known tasks is kept.
    pub fn load_spec(&self, path: &Path) -> Result<StoredSpec, String> {
        let manifest = TaskManifest::load(path).map_err(|error| error.to_string())?;
        let store = self.store()?;
        TaskCoordinator::new(&store)
            .import_manifest(&manifest)
            .map_err(|error| error.to_string())?;
        store
            .upsert_spec(&manifest, Some(path))
            .map_err(|error| error.to_string())?;
        self.spec(&store, &manifest.spec_id)
    }

    pub fn specs(&self) -> Result<Vec<StoredSpec>, String> {
        self.store()?.specs().map_err(|error| error.to_string())
    }

    pub fn board(&self, spec_id: &str) -> Result<TaskBoard, String> {
        let store = self.store()?;
        let spec = self.spec(&store, spec_id)?;
        let config = self.router_config();
        let tasks = store
            .tasks_for_spec(spec_id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|task| {
                let tier_override = store
                    .get_task(&task.id)
                    .map_err(|error| error.to_string())?
                    .and_then(|stored| stored.tier_override);
                let route = config.as_ref().map_err(Clone::clone).and_then(|config| {
                    let rules = RiskRules::for_project(Path::new(&task.cwd))
                        .map_err(|error| error.to_string())?;
                    Ok(resolve_task_route(&task, tier_override, config, &rules))
                });
                Ok(TaskCard {
                    tier_override,
                    route_error: route.as_ref().err().cloned(),
                    route: route.ok(),
                    dependencies_completed: store
                        .dependencies_completed(&task.id)
                        .map_err(|error| error.to_string())?,
                    approval: store
                        .latest_task_approval(&task.id)
                        .map_err(|error| error.to_string())?,
                    latest_run: store
                        .latest_run_for_task(&task.id)
                        .map_err(|error| error.to_string())?,
                    running: self.runs.is_running(&task.id),
                    task,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(TaskBoard { spec, tasks })
    }

    /// Choose a task's tier by hand; `None` returns to the spec's.
    pub fn set_tier(&self, task_id: &str, tier: Option<Tier>) -> Result<(), String> {
        let changed = self
            .store()?
            .set_task_tier_override(task_id, tier)
            .map_err(|error| error.to_string())?;
        if changed {
            Ok(())
        } else {
            Err(format!(
                "no se puede cambiar el tier de {task_id}: no existe, está en curso o ya terminó"
            ))
        }
    }

    pub fn decide(&self, task_id: &str, approved: bool) -> Result<(), String> {
        self.store()?
            .record_task_approval(&TaskApproval {
                task_id: task_id.to_owned(),
                decision: if approved { "approved" } else { "denied" }.to_owned(),
                reason: Some("decidido en la app".to_owned()),
            })
            .map_err(|error| error.to_string())
    }

    pub fn retry(&self, task_id: &str) -> Result<TaskExecutionReceipt, String> {
        let store = self.store()?;
        TaskCoordinator::new(&store)
            .retry_interrupted_task(task_id)
            .map_err(|error| error.to_string())
    }

    /// Mark a task left running by a closed app as interrupted.
    pub fn recover(&self, task_id: &str) -> Result<TaskExecutionReceipt, String> {
        if self.runs.is_running(task_id) {
            return Err(format!("{task_id} se está ejecutando; detenla en su lugar"));
        }
        let store = self.store()?;
        TaskCoordinator::new(&store)
            .recover_running_task(task_id, "recuperada desde la app")
            .map_err(|error| error.to_string())
    }

    pub fn interrupt(&self, task_id: &str) -> Result<(), String> {
        self.runs.interrupt(task_id)
    }

    pub fn respond(
        &self,
        task_id: &str,
        request_id: Value,
        decision: ApprovalDecision,
    ) -> Result<(), String> {
        self.runs.respond(task_id, request_id, decision)
    }

    pub fn run_events(&self, run_id: &str) -> Result<Vec<EventRecord>, String> {
        self.store()?
            .events_for_run(run_id)
            .map_err(|error| error.to_string())
    }

    /// What is running now, if anything.
    pub fn active(&self) -> Option<String> {
        self.active.lock().expect("active run").clone()
    }

    /// Run one task in the background.
    pub fn dispatch(&self, task_id: &str) -> Result<(), String> {
        let work = self.work(task_id)?;
        let task_id = task_id.to_owned();
        std::thread::spawn(move || {
            let result = work.store().and_then(|store| {
                let task = store
                    .get_task(&task_id)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| format!("la tarea {task_id} no existe"))?
                    .definition;
                let mut runtime = work.runtime(&task);
                work.coordinator(&store)
                    .dispatch_task(&task_id, &mut runtime)
                    .map_err(|error| error.to_string())
            });
            work.finish(&task_id, result);
        });
        Ok(())
    }

    /// Run every ready task of a spec in dependency order, in the background.
    pub fn dispatch_ready(&self, spec_id: &str) -> Result<(), String> {
        let work = self.work(&format!("spec {spec_id}"))?;
        let spec_id = spec_id.to_owned();
        std::thread::spawn(move || {
            let result = work.store().and_then(|store| {
                work.coordinator(&store)
                    .dispatch_ready_tasks_with_runtime(&spec_id, |task| Ok(work.runtime(task)))
                    .map_err(|error| error.to_string())
            });
            match result {
                Ok(receipts) => {
                    // As in `finish`: listeners that ask what is running must
                    // already see the slot free.
                    work.release();
                    for receipt in receipts {
                        (work.sink)(TaskEvent::Finished {
                            task_id: receipt.task_id.clone(),
                            receipt,
                        });
                    }
                }
                Err(error) => work.finish(&spec_id, Err(error)),
            }
        });
        Ok(())
    }

    /// Continue an interrupted run on its thread, in the background.
    pub fn resume(&self, run_id: &str) -> Result<(), String> {
        let store = self.store()?;
        let run = store
            .get_run(run_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("la ejecución {run_id} no existe"))?;
        let task_id = run.turn_id;
        let work = self.work(&task_id)?;
        let run_id = run_id.to_owned();
        std::thread::spawn(move || {
            let result = work.store().and_then(|store| {
                let task = store
                    .get_task(&task_id)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| format!("la tarea {task_id} no existe"))?
                    .definition;
                let mut runtime = work.runtime(&task);
                work.coordinator(&store)
                    .resume_run(&run_id, &mut runtime)
                    .map_err(|error| error.to_string())
            });
            work.finish(&task_id, result);
        });
        Ok(())
    }

    fn work(&self, label: &str) -> Result<Work, String> {
        let mut active = self.active.lock().expect("active run");
        if let Some(running) = active.as_ref() {
            return Err(format!("ya hay una ejecución en curso: {running}"));
        }
        *active = Some(label.to_owned());
        Ok(Work {
            store_path: self.store_path.clone(),
            config_for: self.config_for.clone(),
            router: self.router.clone(),
            runs: self.runs.clone(),
            sink: self.sink.clone(),
            active: self.active.clone(),
        })
    }

    fn store(&self) -> Result<RouterStore, String> {
        RouterStore::open(&self.store_path).map_err(|error| error.to_string())
    }

    fn router_config(&self) -> Result<RouterConfig, String> {
        match &self.router {
            Some(config) => Ok(config.clone()),
            None => RouterConfig::load_or_default(&RouterConfig::default_path())
                .map_err(|error| error.to_string()),
        }
    }

    fn spec(&self, store: &RouterStore, spec_id: &str) -> Result<StoredSpec, String> {
        store
            .specs()
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|spec| spec.spec_id == spec_id)
            .ok_or_else(|| format!("la spec {spec_id} no está cargada"))
    }
}

/// Everything a background run needs, detached from the service.
struct Work {
    store_path: PathBuf,
    config_for: TaskConfigFactory,
    router: Option<RouterConfig>,
    runs: Arc<TaskRuns>,
    sink: TaskSink,
    active: Arc<Mutex<Option<String>>>,
}

impl Work {
    fn store(&self) -> Result<RouterStore, String> {
        RouterStore::open(&self.store_path).map_err(|error| error.to_string())
    }

    fn coordinator<'a>(&self, store: &'a RouterStore) -> TaskCoordinator<'a> {
        match &self.router {
            Some(config) => TaskCoordinator::new(store).with_router(config.clone()),
            None => TaskCoordinator::new(store),
        }
    }

    fn runtime(&self, task: &TaskDefinition) -> LocalTaskRuntime {
        let config = (self.config_for)(Path::new(&task.cwd)).with_active_task(&task.id);
        let sink = self.sink.clone();
        let task_id = task.id.clone();
        let ui: EventSink = Arc::new(move |event| {
            sink(TaskEvent::Session {
                task_id: task_id.clone(),
                event,
            })
        });
        LocalTaskRuntime::interactive(config, ui, self.runs.clone())
    }

    fn finish(&self, task_id: &str, result: Result<TaskExecutionReceipt, String>) {
        let event = match result {
            Ok(receipt) => TaskEvent::Finished {
                task_id: task_id.to_owned(),
                receipt,
            },
            Err(error) => TaskEvent::Failed {
                task_id: task_id.to_owned(),
                error,
            },
        };
        // Free the slot first so a listener can start the next run at once.
        self.release();
        (self.sink)(event);
    }

    fn release(&self) {
        *self.active.lock().expect("active run") = None;
    }
}
