//! Domain layer types — business concepts, rules, and processes.
//!
//! The domain layer captures the business problem being solved, independent
//! of any technical implementation. It describes WHAT the software does
//! in business terms.
//!
//! # Business Concepts
//!
//! The nouns of the domain — entities that matter to the business:
//! - `id`, `name`, `description`
//! - `attributes` with abstract types
//! - `relationships` with cardinality
//! - `invariants` that must always hold
//!
//! # Business Rules
//!
//! Conditions and actions that govern business behavior:
//! - `condition` — When the rule applies
//! - `action` — What happens when condition is met
//! - `priority` — Importance level
//!
//! # Business Processes
//!
//! Multi-stage processes with exception handling:
//! - `stages` — Sequential steps with entry/exit conditions
//! - `exception_handling` — How exceptions are handled

use serde::{Deserialize, Serialize};

/// The Domain Layer describes the business problem domain.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Domain {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_concepts: Option<Vec<BusinessConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_rules: Option<Vec<BusinessRule>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_processes: Option<Vec<BusinessProcess>>,
}

/// A business concept — the nouns of the domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessConcept {
    pub id: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<TypedAttribute>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationships: Option<Vec<ConceptRelationship>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub invariants: Option<Vec<String>>,
}

/// A typed attribute of a business concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedAttribute {
    pub name: String,

    /// Abstract type (e.g. "UUID", "string", "boolean", "timestamp").
    #[serde(rename = "type")]
    pub attr_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

/// Relationship between business concepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRelationship {
    /// Role of this relationship (e.g. "created by", "contains").
    pub role: String,

    /// Target business concept ID.
    pub target: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardinality: Option<String>,
}

/// A business rule — conditions and actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessRule {
    pub id: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The condition that triggers this rule.
    pub condition: String,

    /// The action to take when the condition is met.
    pub action: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

/// A multi-stage business process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessProcess {
    pub id: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stages: Option<Vec<ProcessStage>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception_handling: Option<Vec<ExceptionHandler>>,
}

/// A stage within a business process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStage {
    pub stage: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_condition: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_condition: Option<String>,
}

/// Exception handler for a business process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionHandler {
    pub scenario: String,
    pub action: String,
}
