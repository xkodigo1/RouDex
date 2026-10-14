pub mod app_server;
pub mod chat;
pub mod classifier;
pub mod coordinator;
pub mod dashboard;
pub mod desktop_hook;
pub mod domain;
pub mod persistence;
pub mod risk;
pub mod session;
pub mod task_manifest;
pub mod task_route;
pub mod tiers;

pub use app_server::{
    AppServerClient, AppServerConfig, AppServerError, AppServerTurn, TurnSettings,
};
pub use chat::{
    project_dir, ChatRecorder, ChatService, ChatThread, ChatTurn, ConfigFactory, ModelUsage,
    SentTurn, TokenCounts,
};
pub use classifier::{
    escalate_local, route_local, AppServerClassifier, Classification, DecisionSource,
    LocalClassifier, ModelClassifier, RouteDecision, Router,
};
pub use coordinator::{
    CoordinatorError, LocalTaskRuntime, RuntimeTurn, TaskCoordinator, TaskExecutionReceipt,
    TaskRuntime,
};
pub use dashboard::{Dashboard, DayUsage, ThreadSummary, TierUsage};
pub use desktop_hook::{DesktopSummary, HookAction, HookChange, HookOutcome, PromptSubmit};
pub use domain::{
    EventRecord, RoutePromptInput, RouteReceipt, RouteStatus, RunStatus, StoredRun, StoredTask,
    TaskApproval, TaskDefinition, TaskSandbox, TaskStatus,
};
pub use persistence::{RouterStore, StoreError};
pub use risk::{RiskMatch, RiskRules, RiskRulesError};
pub use session::{
    AccessMode, ApprovalDecision, ApprovalKind, EventSink, Session, SessionError, SessionEvent,
};
pub use task_manifest::{ManifestError, Milestone, TaskManifest};
pub use task_route::resolve_task_route;
pub use tiers::{
    CatalogModel, DesktopHookConfig, ModelTarget, RouterConfig, RouterConfigError, Tier,
    TierTargets, ROUTER_CONFIG_VERSION,
};
