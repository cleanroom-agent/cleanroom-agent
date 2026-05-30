//! LLM↔Human bridge MCP tool parameters.
//!
//! These tools allow LLM agents to request human input during task execution.
//! They are correctly placed in MCP because they are LLM-initiated calls.

use serde::Deserialize;
use rmcp::schemars;

/// Parameters for `request_clarification` — LLM asks human for clarification.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RequestClarificationParams {
    /// The question for the human user
    pub question: String,
    /// S.DEF URI providing context for the question
    #[serde(default)]
    pub context_uri: Option<String>,
    /// Pre-defined answer options
    #[serde(default)]
    pub options: Option<Vec<String>>,
}

/// Parameters for `propose_decision` — LLM proposes a design decision to human.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ProposeDecisionParams {
    /// Topic of the design decision
    pub topic: String,
    /// The proposed approach
    pub proposal: String,
    /// Rationale for the proposal
    pub rationale: String,
    /// Alternative approaches considered
    #[serde(default)]
    pub alternatives: Vec<String>,
    /// Components affected by this decision
    #[serde(default)]
    pub affects: Vec<String>,
}

/// Parameters for `preview_changes` — LLM shows generated code diff to human.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PreviewChangesParams {
    /// S.DEF entity URI for the entity being generated
    pub entity_uri: String,
    /// Target programming language
    pub target_language: String,
}

/// Parameters for `pause_workflow` / `resume_workflow` — cross-platform
/// workflow control (replaces OS signals like SIGUSR1/SIGUSR2).
///
/// Both tools take no parameters; the tool name determines the action.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PauseResumeParams {}

/// Response from `request_clarification` (returned by the server).
#[derive(Debug, serde::Serialize)]
pub struct ClarificationResponse {
    pub question_id: String,
    pub status: String, // "answered" | "pending"
    pub answer: Option<String>,
}

/// Response from `propose_decision` (returned by the server).
#[derive(Debug, serde::Serialize)]
pub struct DecisionResponse {
    pub decision_id: String,
    pub status: String, // "approved" | "modified" | "rejected"
    pub modified_proposal: Option<String>,
}

/// Response from `preview_changes` (returned by the server).
#[derive(Debug, serde::Serialize)]
pub struct PreviewResponse {
    pub entity_uri: String,
    pub diff_preview: Option<String>,
    pub affected_files: Vec<String>,
}
