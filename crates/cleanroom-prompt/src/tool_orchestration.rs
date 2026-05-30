//! Tool orchestration hints — guides the LLM on which tools to use
//! for specific task types in the correct order.

use crate::system_prompt::AgentType;

/// Provides orchestration guidance for the LLM.
pub struct ToolOrchestrator;

impl ToolOrchestrator {
    /// System-level hint appended to prompts based on agent type.
    pub fn system_hint(agent_type: AgentType) -> String {
        match agent_type {
            AgentType::Producer => Self::producer_hint(),
            AgentType::Consumer => Self::consumer_hint(),
        }
    }

    /// Get a task-specific orchestration hint.
    pub fn for_task(task_type: &str, agent_type: AgentType) -> &'static str {
        match (agent_type, task_type) {
            (AgentType::Producer, "REPO_ANALYZE") => REPO_ANALYZE_FLOW,
            (AgentType::Producer, "EXTRACT_DATA_MODEL") => EXTRACT_DATA_MODEL_FLOW,
            (AgentType::Producer, "EXTRACT_MODULE") => EXTRACT_MODULE_FLOW,
            (AgentType::Producer, "EXTRACT_ARCHITECTURE") => EXTRACT_ARCHITECTURE_FLOW,
            (AgentType::Producer, "EXTRACT_UI") => EXTRACT_UI_FLOW,
            (AgentType::Producer, "EXTRACT_TESTS") => EXTRACT_TESTS_FLOW,
            (AgentType::Producer, "INFER_DESIGN_DECISIONS") => INFER_DD_FLOW,
            (AgentType::Producer, "VALIDATE_SHARD") => VALIDATE_SHARD_FLOW,
            (AgentType::Consumer, "GENERATE_CODE") => GENERATE_CODE_FLOW,
            (AgentType::Consumer, "RUN_TESTS") => RUN_TESTS_FLOW,
            (AgentType::Consumer, "MERGE_CODE") => MERGE_CODE_FLOW,
            _ => "",
        }
    }

    fn producer_hint() -> String {
        r#"PRODUCER ORCHESTRATION HINTS:

For EXTRACT_DATA_MODEL tasks:
  1. resolve_name — register the entity URI
  2. lsp_get_document_symbols — get all symbols in the file
  3. lsp_get_type_info — for each property, get precise types
  4. lsp_find_references — track cross-file references
  5. complete_task — submit S.DEF data model JSON

For EXTRACT_MODULE tasks:
  1. lsp_get_document_symbols — analyze each file in the module
  2. lsp_find_references — find all imports/exports
  3. lsp_get_diagnostics — collect warnings
  4. complete_task — submit module S.DEF fragment

For INFER_DESIGN_DECISIONS tasks:
  1. get_data_model / get_contract — load related models for context
  2. analyze patterns using LSP and AST data
  3. complete_task — submit DesignDecision records
"#.to_string()
    }

    fn consumer_hint() -> String {
        r#"CONSUMER ORCHESTRATION HINTS:

For GENERATE_CODE tasks:
  1. get_data_model — load the entity specification
  2. batch_resolve_names — pre-resolve ALL dependency symbol names
  3. get_contract — load any interface contracts
  4. Generate code (LLM internal)
  5. complete_task — submit generated files with symbols_registered list

For RUN_TESTS tasks:
  1. claim_task with task_type=RUN_TESTS
  2. Execute tests in sandbox
  3. check_consistency — verify generated code matches S.DEF
  4. complete_task with test results

For MERGE_CODE tasks:
  1. list_shards — identify all generated shards
  2. Merge files, resolve conflicts
  3. compute_fingerprints — re-fingerprint merged output
  4. complete_task with merge report
"#.to_string()
    }
}

// ── Task-specific orchestration flows ───────────────────────────────────

const REPO_ANALYZE_FLOW: &str = r#"
REPO_ANALYZE flow:
  1. Scan repository → identify all source files
  2. Detect languages → partition into modules
  3. Create sub-tasks (EXTRACT_DATA_MODEL, EXTRACT_MODULE, etc.)
  4. Build initial dependency graph
  5. complete_task with repository metadata
"#;

const EXTRACT_DATA_MODEL_FLOW: &str = r#"
EXTRACT_DATA_MODEL flow:
  1. resolve_name — register URI for this entity
  2. lsp_get_document_symbols — get symbols in source file
  3. lsp_get_type_info — for each property (repeated calls)
  4. lsp_find_references — find related entities
  5. complete_task — persist data model with attributes
"#;

const EXTRACT_MODULE_FLOW: &str = r#"
EXTRACT_MODULE flow:
  1. For each file in the module:
     a. lsp_get_document_symbols — file-level symbols
     b. lsp_find_references — cross-file refs
     c. Map symbols to S.DEF entities
  2. Aggregate results into module definition
  3. complete_task with module IR
"#;

const EXTRACT_ARCHITECTURE_FLOW: &str = r#"
EXTRACT_ARCHITECTURE flow:
  1. get_dependency_graph — load existing dependency info
  2. Identify architecture layers (presentation, business, data)
  3. Classify modules into layers
  4. Detect communication patterns
  5. complete_task with architecture description
"#;

const EXTRACT_UI_FLOW: &str = r#"
EXTRACT_UI flow:
  1. Scan UI files (.html, .tsx, .rs for Yew/Dioxus)
  2. Extract screen definitions and navigation
  3. Map to S.DEF UIScreen entities
  4. complete_task with UI specification
"#;

const EXTRACT_TESTS_FLOW: &str = r#"
EXTRACT_TESTS flow:
  1. Identify test files (spec, test, _test suffixes)
  2. Parse test structure (describe/it blocks, #[test])
  3. Extract Given/When/Then patterns
  4. Map to S.DEF test contracts
  5. complete_task with test contract JSON
"#;

const INFER_DD_FLOW: &str = r#"
INFER_DESIGN_DECISIONS flow:
  1. get_data_model — load relevant data models for context
  2. Infer decisions from:
     - Library/framework choices
     - Architectural patterns
     - Naming conventions
     - Config patterns
  3. complete_task with DesignDecision records
"#;

const VALIDATE_SHARD_FLOW: &str = r#"
VALIDATE_SHARD flow:
  1. get_data_model / get_contract — load shard data
  2. check_consistency — verify shard against DB
  3. compute_fingerprints — update fingerprints
  4. list_symbols — verify all refs resolved
  5. complete_task with validation result
"#;

const GENERATE_CODE_FLOW: &str = r#"
GENERATE_CODE flow:
  1. get_data_model — load entity spec
  2. batch_resolve_names — resolve ALL dependency symbols
  3. list_symbols — verify all symbols available
  4. Generate code matching the specification
  5. register_custom_name — register any new symbols
  6. complete_task with generated files and registered symbols
"#;

const RUN_TESTS_FLOW: &str = r#"
RUN_TESTS flow:
  1. Execute generated tests in sandbox
  2. Check test pass/fail results
  3. check_consistency — verify roundtrip
  4. complete_task with test report
"#;

const MERGE_CODE_FLOW: &str = r#"
MERGE_CODE flow:
  1. list_shards — get all generated shard statuses
  2. Merge individual files into final output
  3. Resolve cross-file conflicts
  4. compute_fingerprints — final fingerprint calculation
  5. complete_task with merge report
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_producer_hint_contains_tools() {
        let hint = ToolOrchestrator::system_hint(AgentType::Producer);
        assert!(hint.contains("lsp_get_document_symbols"));
        assert!(hint.contains("complete_task"));
    }

    #[test]
    fn test_consumer_hint_contains_tools() {
        let hint = ToolOrchestrator::system_hint(AgentType::Consumer);
        assert!(hint.contains("batch_resolve_names"));
        assert!(hint.contains("check_consistency"));
    }

    #[test]
    fn test_task_specific_flows() {
        let flow = ToolOrchestrator::for_task("EXTRACT_DATA_MODEL", AgentType::Producer);
        assert!(flow.contains("resolve_name"));
        assert!(flow.contains("lsp_get_type_info"));

        let flow = ToolOrchestrator::for_task("GENERATE_CODE", AgentType::Consumer);
        assert!(flow.contains("get_data_model"));
        assert!(flow.contains("batch_resolve_names"));
    }

    #[test]
    fn test_unknown_task_returns_empty() {
        assert_eq!(ToolOrchestrator::for_task("UNKNOWN", AgentType::Producer), "");
    }
}
