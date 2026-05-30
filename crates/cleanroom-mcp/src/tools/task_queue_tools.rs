//! Task queue management MCP tool parameters.
//!
//! These tools allow viewing and manipulating the task queue from CLI
//! via the UDS MCP transport. Only pending tasks can be modified.

use serde::Deserialize;
use rmcp::schemars;

/// Parameters for `get_task_queue` — list tasks with optional filters.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTaskQueueParams {
    #[serde(default)]
    pub document_name: Option<String>,
    /// Filter by task status (optional)
    #[serde(default)]
    pub filter_status: Option<Vec<String>>,
    /// Filter by task type (optional)
    #[serde(default)]
    pub filter_type: Option<String>,
}

/// Parameters for `insert_task` — create a new pending task.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InsertTaskParams {
    /// Task type (e.g. "EXTRACT_DATA_MODEL")
    pub task_type: String,
    /// Priority (higher = earlier execution)
    #[serde(default)]
    pub priority: Option<i32>,
    /// JSON input payload for the task
    pub input: serde_json::Value,
    /// Task ID of a predecessor (new task depends on it)
    #[serde(default)]
    pub after_task_id: Option<String>,
    /// Explicit dependency task IDs
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    /// Maximum retry count (optional)
    #[serde(default)]
    pub max_retries: Option<i32>,
}

/// Parameters for `remove_task` — delete a pending task.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoveTaskParams {
    pub task_id: String,
}

/// Parameters for `modify_task` — update a pending task's properties.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ModifyTaskParams {
    pub task_id: String,
    /// New priority (optional)
    #[serde(default)]
    pub priority: Option<i32>,
    /// New input JSON (optional)
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    /// New dependency list (optional)
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    /// Max retries (optional)
    #[serde(default)]
    pub max_retries: Option<i32>,
}
