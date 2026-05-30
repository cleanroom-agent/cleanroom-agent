//! Graceful degradation — when a component is unavailable, degrade functionality.
//!
//! Implements docs/16-resilience.md §9. When LSP, LLM, or database components
//! encounter persistent failures, the system degrades to a mode that preserves
//! partial functionality rather than failing entirely.

use std::collections::HashSet;

/// Current operating mode for graceful degradation.
///
/// Higher modes restrict more operations. The system should always
/// attempt to operate at `Normal` and degrade only when necessary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DegradationMode {
    /// All systems available — full functionality.
    Normal,

    /// LSP server unavailable for specific languages.
    /// Tree-sitter analysis only for those languages.
    NoLsp(LanguageOption),

    /// LLM provider unavailable — queue generation tasks for later.
    /// Read operations (query, import/export) still work.
    NoLLm,

    /// Database is read-only (e.g., disk full, backup in progress).
    /// Only queries and consistency checks are permitted.
    ReadOnly,

    /// Critical failure — safe shutdown ASAP.
    /// No operations are permitted.
    Emergency,
}

/// Which languages have no LSP server available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageOption {
    pub languages: HashSet<String>,
    /// Whether to allow tree-sitter fallback for these languages.
    pub fallback_to_tree_sitter: bool,
}

impl Default for DegradationMode {
    fn default() -> Self {
        DegradationMode::Normal
    }
}

impl DegradationMode {
    /// Returns true if LSP operations are usable at all.
    pub fn lsp_available(&self) -> bool {
        matches!(self, DegradationMode::Normal)
    }

    /// Returns true if LSP is unavailable for a specific language.
    /// In `Normal` mode, all languages are available.
    /// In `NoLsp` mode, only the specified languages are affected.
    pub fn lsp_available_for(&self, language: &str) -> bool {
        match self {
            DegradationMode::Normal => true,
            DegradationMode::NoLsp(ref langs) => {
                !langs.languages.contains(language) || langs.fallback_to_tree_sitter
            }
            _ => false,
        }
    }

    /// Returns true if LLM generation is available.
    pub fn llm_available(&self) -> bool {
        matches!(self, DegradationMode::Normal | DegradationMode::NoLsp(_))
    }

    /// Returns true if database writes are permitted.
    pub fn writes_permitted(&self) -> bool {
        matches!(
            self,
            DegradationMode::Normal
                | DegradationMode::NoLsp(_)
                | DegradationMode::NoLLm
        )
    }

    /// Returns true if ANY useful operation is possible.
    pub fn is_operational(&self) -> bool {
        !matches!(self, DegradationMode::Emergency)
    }

    /// Human-readable description of the current mode.
    pub fn description(&self) -> &'static str {
        match self {
            DegradationMode::Normal => "All systems operational",
            DegradationMode::NoLsp(_) => "LSP server degraded — using tree-sitter fallback",
            DegradationMode::NoLLm => "LLM unavailable — read-only mode",
            DegradationMode::ReadOnly => "Database is read-only — check disk space",
            DegradationMode::Emergency => "Critical failure — safe shutdown required",
        }
    }

    /// Create a NoLsp mode that allows tree-sitter fallback.
    pub fn no_lsp_with_fallback(languages: &[&str]) -> Self {
        DegradationMode::NoLsp(LanguageOption {
            languages: languages.iter().map(|s| s.to_string()).collect(),
            fallback_to_tree_sitter: true,
        })
    }

    /// Create a NoLsp mode that does NOT allow fallback (strict).
    pub fn no_lsp_strict(languages: &[&str]) -> Self {
        DegradationMode::NoLsp(LanguageOption {
            languages: languages.iter().map(|s| s.to_string()).collect(),
            fallback_to_tree_sitter: false,
        })
    }
}

/// Set of operations allowed in the current degradation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Operation {
    /// Query S.DEF entities
    Query,
    /// Import/export S.DEF documents
    ImportExport,
    /// Check database consistency
    ConsistencyCheck,
    /// Run LSP analysis
    LspAnalysis,
    /// Generate code via LLM
    LlmGeneration,
    /// Write to database
    Write,
}

impl DegradationMode {
    /// Check if an operation is permitted in the current mode.
    pub fn allows(&self, op: Operation) -> bool {
        match op {
            Operation::Query => self.is_operational(),
            Operation::ImportExport => self.is_operational(),
            Operation::ConsistencyCheck => self.is_operational(),
            Operation::LspAnalysis => self.lsp_available(),
            Operation::LlmGeneration => self.llm_available(),
            Operation::Write => self.writes_permitted(),
        }
    }

    /// List all operations permitted in the current mode.
    pub fn permitted_operations(&self) -> Vec<Operation> {
        let all = [
            Operation::Query,
            Operation::ImportExport,
            Operation::ConsistencyCheck,
            Operation::LspAnalysis,
            Operation::LlmGeneration,
            Operation::Write,
        ];
        all.iter().copied().filter(|op| self.allows(*op)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_allows_everything() {
        let mode = DegradationMode::Normal;
        assert!(mode.lsp_available());
        assert!(mode.llm_available());
        assert!(mode.writes_permitted());
        assert!(mode.is_operational());
    }

    #[test]
    fn test_no_lsp_degradation() {
        let mode = DegradationMode::no_lsp_strict(&["rust"]);
        assert!(!mode.lsp_available());
        assert!(!mode.lsp_available_for("rust"));
        assert!(mode.llm_available());
        assert!(mode.writes_permitted());
    }

    #[test]
    fn test_no_lsp_with_fallback() {
        let mode = DegradationMode::no_lsp_with_fallback(&["python"]);
        // Rust LSP is still available
        assert!(mode.lsp_available_for("rust"));
        // Python LSP is down but fallback is allowed
        assert!(mode.lsp_available_for("python"));
    }

    #[test]
    fn test_no_lsp_strict() {
        let mode = DegradationMode::no_lsp_strict(&["python"]);
        assert!(!mode.lsp_available_for("python"));
    }

    #[test]
    fn test_no_llm_read_only() {
        let mode = DegradationMode::NoLLm;
        assert!(!mode.llm_available());
        assert!(mode.writes_permitted());
        assert!(mode.is_operational());
    }

    #[test]
    fn test_read_only_denies_writes() {
        let mode = DegradationMode::ReadOnly;
        assert!(!mode.writes_permitted());
        assert!(mode.is_operational());
    }

    #[test]
    fn test_emergency_denies_everything() {
        let mode = DegradationMode::Emergency;
        assert!(!mode.is_operational());
        assert!(!mode.writes_permitted());
        assert!(!mode.llm_available());
        assert!(!mode.lsp_available());
    }
}
