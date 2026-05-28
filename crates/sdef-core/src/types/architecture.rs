//! Architecture types.

use serde::{Deserialize, Serialize};

/// Architecture describes the high-level structure of the software.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Architecture {
    /// Architectural style (e.g. "layered", "microservices", "event-driven", "hexagonal").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<ArchitectureLayer>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules: Option<Vec<ArchitectureModule>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication: Option<Vec<CommunicationPattern>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_cutting_concerns: Option<Vec<CrossCuttingConcern>>,
}

/// A structural layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureLayer {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<String>>,
}

/// A module — a unit of related functionality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureModule {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsibility: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exports: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ModuleComponent>>,
}

/// A component within a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleComponent {
    pub name: String,

    /// Component type (e.g. "service", "controller", "repository", "model").
    pub type_: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Communication pattern between parts of the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPattern {
    pub type_: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A cross-cutting concern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossCuttingConcern {
    pub name: String,
    pub description: String,
}
