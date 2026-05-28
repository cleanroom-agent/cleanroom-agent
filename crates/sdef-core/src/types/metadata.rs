//! Metadata types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Software metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoftwareMetadata {
    /// Author / maintainer information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<Author>>,

    /// Software license identifier (SPDX).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Homepage URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    /// Source code repository URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    /// Category (e.g. "web_application", "library", "cli_tool").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Tags for categorization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Target platforms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_platforms: Option<Vec<String>>,

    /// Compatibility policy (e.g. "none", "active", "full").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility_policy: Option<String>,

    /// Arbitrary annotations for extensibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, serde_json::Value>>,
}

/// Author information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
