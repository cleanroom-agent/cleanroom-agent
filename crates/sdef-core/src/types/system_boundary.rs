//! System boundary types — what the software does and does NOT do.
//!
//! The system boundary clarifies scope by explicitly stating:
//! - Core purpose in one sentence
//! - Target users
//! - Features in scope
//! - Explicit non-goals (prevents over-implementation)
//! - Success criteria
//! - Constraints
//! - External dependencies
//!
//! # Non-Goals
//!
//! The `non_goals` field is particularly important — it explicitly lists
//! what the software is NOT trying to do, preventing feature creep and
//! setting realistic expectations with stakeholders.

use serde::{Deserialize, Serialize};

/// System boundary — what the software does and does NOT do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBoundary {
    /// The core purpose of the software in one sentence.
    pub core_purpose: String,

    /// Target user personas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_users: Option<Vec<String>>,

    /// Features that are in scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_scope: Option<Vec<String>>,

    /// Features explicitly out of scope (prevents over-implementation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_goals: Option<Vec<String>>,

    /// Measurable success criteria.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_criteria: Option<Vec<String>>,

    /// Constraints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Vec<Constraint>>,

    /// External dependencies at the system level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_dependencies: Option<Vec<String>>,
}

/// A constraint on the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Constraint category (e.g. "performance", "compatibility", "security").
    pub category: String,

    /// Constraint description.
    pub description: String,
}
