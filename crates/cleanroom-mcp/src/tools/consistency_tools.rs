//! Consistency check MCP tool parameters.

use rmcp::schemars;
use serde::{Deserialize, Serialize};

/// Run a consistency check.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConsistencyCheckParams {
    /// Document name to check.
    pub document_name: String,
    /// Check type: "fast" (default), "full", or "deep".
    #[serde(default = "default_check_type")]
    pub check_type: String,
}

fn default_check_type() -> String { "fast".to_string() }

/// Compute fingerprints parameters.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FingerprintParams {
    /// Document name.
    pub document_name: String,
}

/// Resolve an inconsistency.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResolveInconsistencyParams {
    /// Document name.
    pub document_name: String,
    /// Entity URI to fix.
    pub entity_uri: String,
    /// Fix strategy: "sync_code_to_sdef", "regenerate_code", "sync_db_to_sdef",
    /// "sync_sdef_to_db", or "accept_external".
    pub strategy: String,
}

/// Get inconsistency report.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InconsistencyReportParams {
    /// Document name.
    pub document_name: String,
    /// Optional filter by entity type.
    pub entity_type: Option<String>,
}

/// Inconsistency item for report output.
#[derive(Debug, Clone, Serialize)]
pub struct InconsistencyItem {
    pub entity_uri: String,
    pub entity_type: String,
    pub code_path: Option<String>,
    pub sdef_hash: Option<String>,
    pub db_hash: Option<String>,
    pub code_hash: Option<String>,
    pub last_consistent_at: Option<String>,
    pub suggested_strategies: Vec<String>,
}

/// Inconsistency report output.
#[derive(Debug, Clone, Serialize)]
pub struct InconsistencyReport {
    pub document_name: String,
    pub total_inconsistencies: usize,
    pub items: Vec<InconsistencyItem>,
}
