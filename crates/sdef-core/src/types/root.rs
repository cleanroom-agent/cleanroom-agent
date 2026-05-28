//! Root definition.

use serde::{Deserialize, Serialize};

use super::{metadata, system_boundary, design_decisions, versioning, domain, architecture, data_model, contracts, behavior, ui, tests, reconstruction, deployment, dependencies};

/// The root object of any S.DEF document.
/// Contains all layers of software description.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoftwareDefinition {
    /// Schema version (date-based, e.g. "2026-05-27").
    pub sdef_version: String,

    /// The software identifier (e.g. "com.example.todoapp").
    pub name: String,

    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Version of the software being described.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<metadata::SoftwareMetadata>,

    /// System boundary — what the software does and does NOT do.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_boundary: Option<system_boundary::SystemBoundary>,

    /// Design decisions with rationale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design_decisions: Option<Vec<design_decisions::DesignDecision>>,

    /// Version history.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_history: Option<Vec<versioning::VersionRecord>>,

    /// Domain model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<domain::Domain>,

    /// Architecture — structural layers, modules, and communication patterns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<architecture::Architecture>,

    /// Data model — entities, attributes, relationships, and validation rules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_models: Option<Vec<data_model::DataModel>>,

    /// Contracts — interfaces, classes, enums, and API endpoints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contracts: Option<contracts::Contracts>,

    /// Behavior — functions, flows, and state machines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<behavior::Behavior>,

    /// User interface — screens, components, interactions, and navigation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<ui::UserInterface>,

    /// Test contracts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<tests::TestContract>,

    /// Reconstruction rules — fidelity target, technology constraints, and directives.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconstruction_rules: Option<reconstruction::ReconstructionRules>,

    /// External software dependencies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<dependencies::Dependency>>,

    /// Deployment and runtime requirements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment: Option<deployment::Deployment>,

    /// Resources the software provides or consumes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<dependencies::Resource>>,
}
