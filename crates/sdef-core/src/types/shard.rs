//! Shard types — internal runtime entities (NOT part of S.DEF exchange format).
//!
//! Shards are internal runtime entities used by Cleanroom Agent to track
//! the state of generated code during the reconstruction process. They are
//! NOT exchanged as part of the S.DEF format — only the database stores them.
//!
//! # Shard vs S.DEF
//!
//! | Aspect | S.DEF | Shard |
//! |--------|-------|-------|
//! | Purpose | Exchange format | Internal tracking |
//! | Serialized | JSON/YAML file | Database only |
//! | Content | Full spec | Generated code hash, status |
//!
//! # Shard Lifecycle
//!
//! ```text
//! Pending → Generating → Generated → Validating → Validated → CodeGenerated → Tested
//!                                                                ↓
//!                                                              Failed
//! ```
//!
//! # Shard Types
//!
//! Each shard has a `section_type` indicating what S.DEF section it belongs to:
//! - `DataModel("User")` — Data model entity shard
//! - `Interface("UserService")` — Interface contract shard
//! - `Function("createUser")` — Function spec shard
//! - `UiScreen("Dashboard")` — UI screen shard
//! - etc.

use serde::{Deserialize, Serialize};

/// Metadata for a S.DEF shard.
///
/// A shard represents a portion of the S.DEF document that can be
/// independently generated, validated, and tracked.
///
/// # Fields
///
/// - `shard_id` — Unique identifier for this shard
/// - `sdef_uri` — URI reference to the S.DEF entity (e.g., `sdef://com.example/app/contracts/interfaces#TodoService`)
/// - `section_type` — Category of this shard
/// - `file_path` — Where the generated code should go
/// - `content_hash` — SHA-256 of the generated content
/// - `status` — Current lifecycle status
/// - `token_estimate` — Estimated token count for LLM processing
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
