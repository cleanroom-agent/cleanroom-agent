//! Task management MCP tools.
//!
//! These will be moved onto the `CleanroomMcpServer` `#[tool_handler]` impl
//! once `cleanroom-db` is implemented.

use rmcp::schemars;
use serde::Deserialize;

/// Task creation parameters.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTaskParams {
    /// Task type
    pub task_type: String,
    /// Task input parameters
    pub input: serde_json::Value,
    /// Priority (1-10)
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// Dependent task IDs
    #[serde(default)]
    pub dependencies: Vec<String>,
}

fn default_priority() -> i32 { 5 }

/// Task claim parameters.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClaimTaskParams {
    pub agent_id: String,
}

/// Task progress update parameters.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateProgressParams {
    pub task_id: String,
    pub progress: f64,
    pub message: Option<String>,
}

/// Task completion parameters.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CompleteTaskParams {
    pub task_id: String,
    pub output: serde_json::Value,
}

/// Task failure parameters.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FailTaskParams {
    pub task_id: String,
    pub error_message: String,
}

/// Heartbeat parameters.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HeartbeatParams {
    pub task_id: String,
    pub agent_id: String,
}

/// Task listing parameters.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListTasksParams {
    pub status: Option<String>,
    pub task_type: Option<String>,
    pub assigned_to: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize { 20 }
