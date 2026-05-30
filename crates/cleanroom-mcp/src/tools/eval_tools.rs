//! Evaluation tool parameters for MCP.
//!
//! These parameters are used by the MCP server to deserialize evaluation
//! tool calls from the LLM. Each struct corresponds to a specific tool and
//! is annotated with [`schemars::JsonSchema`] for JSON Schema generation.

use serde::Deserialize;
use rmcp::schemars;

/// Parameters for the `run_evaluation` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RunEvalParams {
    /// Specific benchmark project name (omit for all built-in benchmarks).
    #[serde(default)]
    pub project_name: Option<String>,
    /// Output directory for evaluation reports.
    #[serde(default)]
    pub output_dir: Option<String>,
}

/// Parameters for the `get_evaluation_report` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetEvalReportParams {
    /// Filter by project name (omit to get summaries for all projects).
    #[serde(default)]
    pub project_name: Option<String>,
    /// Maximum number of historical records to return (default: 10).
    #[serde(default)]
    pub limit: Option<usize>,
}
