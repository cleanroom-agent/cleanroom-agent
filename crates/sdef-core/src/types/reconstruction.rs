//! Reconstruction rules types — directives for code generation.
//!
//! Guides how the S.DEF document should be used to regenerate code.
//!
//! # Reconstruction Fidelity
//!
//! | Value | Meaning |
//! |-------|---------|
//! | `high` | Maximum detail, all optional fields populated |
//! | `medium` | Balanced detail (default) |
//! | `low` | Minimal representation |
//!
//! # Compatibility Modes
//!
//! | Mode | Description |
//! |------|-------------|
//! | `full` | Include all legacy elements |
//! | `mixed` | Include compat layers, mark deprecated |
//! | `clean` | Current version only |
//! | `custom` | User-defined via `target_versions` |
//!
//! # Tech Constraints
//!
//! Specifies the target technology stack:
//! - Language family (Rust, TypeScript, Python, Go, etc.)
//! - Runtime requirements
//! - Persistence model (SQL, NoSQL, etc.)
//! - Concurrency model
//! - Allowed licenses
//! - Preferred frameworks
//!
//! # Directives
//!
//! [`ReconstructionDirective`] guides code generation with priority levels:
//! - `must` — Must be followed
//! - `should` — Strong recommendation
//! - `may` — Optional guidance
//!
//! Locked directives cannot be modified by agents without explicit approval.

use serde::{Deserialize, Serialize};

/// Reconstruction rules guide how to rebuild from S.DEF.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReconstructionRules {
    /// Target reconstruction fidelity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconstruction_fidelity: Option<String>,

    /// Compatibility mode for code generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility_mode: Option<String>,

    /// When compatibility_mode is "custom", which versions to include compat for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_versions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tech_constraints: Option<TechConstraints>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub directives: Option<Vec<ReconstructionDirective>>,
}

/// Technology stack constraints.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TechConstraints {
    /// Target language family.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_family: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence_model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_licenses: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_frameworks: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_variables: Option<Vec<EnvironmentVariable>>,
}

/// A directive that guides reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructionDirective {
    /// Priority level: "must" | "should" | "may".
    pub priority: String,

    pub directive: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// When true, agents must not modify the targeted code without explicit approval.
    #[serde(default)]
    pub locked: bool,
}

/// Environment variable specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub name: String,
    pub description: String,

    #[serde(default)]
    pub secret: bool,
}
