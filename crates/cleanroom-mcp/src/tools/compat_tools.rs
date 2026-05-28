//! Compatibility mode MCP tool parameters.

use rmcp::schemars;
use serde::{Deserialize, Serialize};

/// Set compatibility mode for a document.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetCompatModeParams {
    /// Document name.
    pub document_name: String,
    /// Compatibility mode: "full", "mixed", "clean", or "custom".
    pub mode: String,
}

/// List compatibility layers for a document.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListCompatLayersParams {
    /// Document name.
    pub document_name: String,
}

/// Get compatibility layer detail.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCompatLayerParams {
    /// Document name.
    pub document_name: String,
    /// Layer identifier.
    pub layer_id: String,
}

/// Ignore a compatibility layer (mark as resolved).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IgnoreCompatLayerParams {
    /// Document name.
    pub document_name: String,
    /// Layer identifier to ignore.
    pub layer_id: String,
}

/// Compatibility layer info.
#[derive(Debug, Clone, Serialize)]
pub struct CompatLayerInfo {
    pub layer_id: String,
    pub source_interface: Option<String>,
    pub target_interface: Option<String>,
    pub transform_type: String,
    pub bidirectional: bool,
    pub is_ignored: bool,
    pub priority: i32,
}

/// Compat mode info.
#[derive(Debug, Clone, Serialize)]
pub struct CompatModeInfo {
    pub document_name: String,
    pub current_mode: String,
    pub layers: Vec<CompatLayerInfo>,
    pub incompatibilities_remaining: usize,
}
