//! sdef-core type modules.

pub mod root;       // SoftwareDefinition
pub mod metadata;   // SoftwareMetadata, Author
pub mod system_boundary;
pub mod design_decisions;
pub mod versioning;    // VersionRecord, DeprecationInfo, CompatibilityMapping, DataMigration
pub mod domain;
pub mod architecture;
pub mod data_model;
pub mod contracts;
pub mod behavior;
pub mod ui;
pub mod tests;
pub mod reconstruction;
pub mod deployment;
pub mod dependencies;
pub mod shard;       // ShardMetadata, ShardType, ShardStatus
