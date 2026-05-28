//! Versioning and compatibility types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A record of a software version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRecord {
    /// Version identifier (e.g. "1.0.0", "2.0.0").
    pub version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,

    #[serde(default)]
    pub deprecated: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub eol_date: Option<String>,

    /// Breaking changes introduced in this version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breaking_changes: Option<Vec<String>>,

    /// Notes on backward compatibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility_notes: Option<String>,
}

/// Deprecation metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeprecationInfo {
    /// Version in which this element was deprecated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since_version: Option<String>,

    /// Reference to the element that replaces this one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,

    /// Version in which this element is planned (or was) removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removal_version: Option<String>,

    /// Guidance for migrating from this element to its replacement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_guide: Option<String>,
}

/// Compatibility mapping — describes how a legacy element maps to the current version.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompatibilityMapping {
    /// The target element this one forwards to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maps_to: Option<String>,

    /// Mapping of legacy field names to current field names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_mapping: Option<HashMap<String, String>>,

    /// Pseudocode describing the transformation logic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_logic: Option<String>,

    #[serde(default)]
    pub bidirectional: bool,
}

/// Data migration specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMigration {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Source entity name (old version).
    pub from_entity: String,

    /// Target entity name (new version).
    pub to_entity: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_version: Option<String>,

    /// Pseudocode describing the migration algorithm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
}
