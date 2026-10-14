use crate::tiers::Tier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RouteStatus {
    Accepted,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunStatus {
    Queued,
    Running,
    AwaitingApproval,
    Completed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Planned,
    Ready,
    AwaitingApproval,
    Running,
    Completed,
    Failed,
    Blocked,
    Interrupted,
}

impl TaskStatus {
    pub fn can_transition_to(&self, next: &Self) -> bool {
        use TaskStatus::*;
        matches!(
            (self, next),
            (Planned, Ready | Blocked)
                | (Ready, AwaitingApproval | Running | Blocked)
                | (AwaitingApproval, Ready | Running | Blocked)
                | (Running, Completed | Failed | Interrupted | Blocked)
                | (Interrupted, Ready | Running | Blocked)
                | (Failed, Blocked)
                | (Blocked, Ready)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskSandbox {
    #[serde(rename = "type")]
    pub kind: String,
    pub writable_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "TaskDefinitionWire")]
pub struct TaskDefinition {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub depends_on: Vec<String>,
    pub status: TaskStatus,
    /// Tier the spec asks for; `router.toml` turns it into a model.
    pub tier: Tier,
    /// Profile of a manifest written before tiers, kept to explain the tier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Model and effort resolved when the task is dispatched; empty until
    /// then. The definition recorded with a run carries them, so resuming
    /// uses the same model even if `router.toml` changed.
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub effort: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestone: Option<String>,
    pub approval: String,
    pub scope: Vec<String>,
    pub cwd: String,
    pub sandbox: TaskSandbox,
    pub acceptance: Vec<String>,
    pub verify: String,
}

/// Tier for the profiles of manifests written before tiers.
pub fn tier_for_profile(profile: &str) -> Option<Tier> {
    match profile {
        "micro" | "routine" => Some(Tier::Economico),
        "standard" => Some(Tier::Estandar),
        "deep" | "frontier" => Some(Tier::Fuerte),
        _ => None,
    }
}

/// Serialized form: `tier`, or a legacy `profile` that maps to one.
#[derive(Deserialize)]
struct TaskDefinitionWire {
    id: String,
    title: String,
    prompt: String,
    depends_on: Vec<String>,
    status: TaskStatus,
    #[serde(default)]
    tier: Option<Tier>,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    model: String,
    #[serde(default)]
    effort: String,
    #[serde(default)]
    milestone: Option<String>,
    approval: String,
    scope: Vec<String>,
    cwd: String,
    sandbox: TaskSandbox,
    acceptance: Vec<String>,
    verify: String,
}

impl TryFrom<TaskDefinitionWire> for TaskDefinition {
    type Error = String;

    fn try_from(wire: TaskDefinitionWire) -> Result<Self, String> {
        let tier = match (wire.tier, &wire.profile) {
            (Some(tier), _) => tier,
            (None, Some(profile)) => tier_for_profile(profile)
                .ok_or_else(|| format!("task {} uses unknown profile {profile}", wire.id))?,
            (None, None) => return Err(format!("task {} declares no tier", wire.id)),
        };
        Ok(Self {
            id: wire.id,
            title: wire.title,
            prompt: wire.prompt,
            depends_on: wire.depends_on,
            status: wire.status,
            tier,
            profile: wire.profile,
            model: wire.model,
            effort: wire.effort,
            milestone: wire.milestone,
            approval: wire.approval,
            scope: wire.scope,
            cwd: wire.cwd,
            sandbox: wire.sandbox,
            acceptance: wire.acceptance,
            verify: wire.verify,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredTask {
    pub spec_id: String,
    pub definition: TaskDefinition,
    /// Tier the user chose by hand, which wins over the spec and risk rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier_override: Option<Tier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskApproval {
    pub task_id: String,
    pub decision: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutePromptInput {
    pub prompt: String,
    pub session_id: String,
    pub turn_id: String,
    pub cwd: String,
    pub permission_mode: Option<String>,
}

pub struct RouteReceipt {
    pub route_id: String,
    pub run_id: String,
    pub profile: String,
    pub model: String,
    pub effort: String,
    pub status: RouteStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredRun {
    pub run_id: String,
    pub route_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub cwd: String,
    pub prompt: String,
    pub profile: String,
    pub model: String,
    pub effort: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventRecord {
    pub run_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}
