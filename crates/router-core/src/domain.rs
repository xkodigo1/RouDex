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
