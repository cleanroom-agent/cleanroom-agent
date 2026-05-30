//! Dependency and resource types.
//!
//! Describes external software dependencies and resources the software
//! provides or consumes.
//!
//! # Dependency Types
//!
//! | Type | Meaning |
//! |------|---------|
//! | `runtime` | Required at runtime (e.g., library) |
//! | `build` | Required for building (e.g., compiler) |
//! | `dev` | Required for development only |
//! | `optional` | Optional feature |
//!
//! # Resources
//!
//! Resources represent integrations with external systems:
//! - Databases
//! - Message queues
//! - File storage
//! - External APIs

use serde::{Deserialize, Serialize};

/// External software dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// "runtime" | "build" | "dev" | "optional".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Resource the software provides or consumes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub name: String,
    pub type_: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
