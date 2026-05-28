//! sdef-core — Rust types for S.DEF (Software Definition Exchange Format)
//!
//! Canonical source: S.DEF/schema/draft/schema.ts
//!
//! All types are serde-serializable for JSON/YAML export.

pub mod types;
pub mod version;

pub use types::*;
pub use version::*;

// Re-export all top-level types for convenience
pub use types::root::SoftwareDefinition;
