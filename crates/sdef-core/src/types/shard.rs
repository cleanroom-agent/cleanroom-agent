//! Shard types — internal runtime entities (not part of S.DEF exchange format).

use serde::{Deserialize, Serialize};

/// Metadata for a S.DEF shard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMetadata {
    pub shard_id: String,

    /// S.DEF URI (e.g. "sdef://com.example/app/contracts/interfaces#TodoService").
    pub sdef_uri: String,

    pub section_type: ShardType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_estimate: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<String>>,

    pub status: ShardStatus,
}

/// Section type of a shard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShardType {
    RootIndex,
    Metadata,
    SystemBoundary,
    Architecture,
    DataModel(String),
    Interface(String),
    Class(String),
    Enum,
    Api(String),
    Function(String),
    Flow(String),
    UiScreen(String),
    UiDesignSystem,
    UiDocument,
    UnitTest(String),
    IntegrationTest,
    DesignDecision(String),
    CompatibilityModule(String),
    Deployment,
    ReconstructionRules,
    Other(String),
}

/// Lifecycle status of a shard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ShardStatus {
    Pending,
    Generating,
    Generated,
    Validating,
    Validated,
    CodeGenerated,
    Tested,
    Failed,
}
