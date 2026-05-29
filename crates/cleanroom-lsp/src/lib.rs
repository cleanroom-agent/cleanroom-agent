//! cleanroom-lsp — LSP client for Cleanroom Agent.

#![allow(missing_docs)]
#![allow(dead_code)]

pub mod client;
pub mod error;
pub mod server_pool;
pub mod file_analysis;
pub mod language_detection;

pub use client::LspClient;
pub use error::{LspError, LspResult};
pub use server_pool::{LspConfig, LspServerPool, LspServerHandle};
pub use file_analysis::{
    FileAnalysis, DocumentSymbol, Diagnostic, DiagnosticSeverity,
    TypeInfo, TypeHierarchy, TypeHierarchyItem, TextPosition,
};
pub use language_detection::{detect_language, supported_languages, is_language_supported};
pub use lsp_types::{Location, SymbolKind};