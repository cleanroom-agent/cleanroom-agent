//! Deployment types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Deployment and runtime requirements.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Deployment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RuntimeRequirement>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_output: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_steps: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<Vec<ConfigurationVar>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling: Option<ScalingStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitoring: Option<String>,
}

/// Runtime requirement.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeRequirement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_versions: Option<HashMap<String, String>>,
}

/// Configuration variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationVar {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
}

/// Scaling strategy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScalingStrategy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
