//! Design decision types.

use serde::{Deserialize, Serialize};

/// A record of a design decision, its context, alternatives considered,
/// rationale, and consequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignDecision {
    /// Unique identifier.
    pub id: String,

    /// The topic / area this decision applies to.
    pub topic: String,

    /// The chosen approach.
    pub decision: String,

    /// Why this approach was chosen.
    pub rationale: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    /// Alternatives that were considered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<String>>,

    /// Consequences (both positive and negative).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consequences: Option<Vec<String>>,

    /// Additional constraints imposed by this decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Vec<String>>,
}
