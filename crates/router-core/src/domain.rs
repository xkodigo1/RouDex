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
pub struct TaskDefinition {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub depends_on: Vec<String>,
    pub status: TaskStatus,
    pub profile: String,
    pub model: String,
    pub effort: String,
    pub approval: String,
    pub scope: Vec<String>,
    pub cwd: String,
    pub sandbox: TaskSandbox,
    pub acceptance: Vec<String>,
    pub verify: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredTask {
    pub spec_id: String,
    pub definition: TaskDefinition,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
