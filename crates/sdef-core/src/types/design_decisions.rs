//! Design decision types — architectural choices with rationale.
//!
//! Records significant design decisions made during software development,
//! capturing the context, alternatives considered, and consequences.
//!
//! # Purpose
//!
//! Design decisions serve as documentation for WHY the software is structured
//! a certain way. They help future maintainers understand constraints and
//! reasoning without needing to dig through meeting notes or archived emails.
//!
//! # Architecture Decision Record (ADR) Pattern
//!
//! Each [`DesignDecision`] is an ADR capturing:
//! - **Topic** — What area does this decision affect?
//! - **Decision** — What was chosen?
//! - **Rationale** — Why was this choice made?
//! - **Context** — What constraints or requirements existed?
//! - **Alternatives** — What other options were considered?
//! - **Consequences** — What are the outcomes (positive and negative)?
//! - **Constraints** — What constraints does this impose?

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
