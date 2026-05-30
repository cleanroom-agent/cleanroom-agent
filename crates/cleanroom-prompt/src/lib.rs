//! cleanroom-prompt — Prompt engineering and LLM interaction layer.
//!
//! Implements the three-layer prompt architecture from docs/11-prompt-engineering.md:
//!
//! - Layer 1: System Prompt (role, rules, output format, tools)
//! - Layer 2: Context (S.DEF shards, dependency ctx, symbols)
//! - Layer 3: Task Prompt (specific instruction, tool orchestration)
//!
//! Also provides response parsing, few-shot management, and context budgeting.

pub mod context_budget;
pub mod few_shot;
pub mod response_parser;
pub mod system_prompt;
pub mod tool_orchestration;

use std::collections::HashMap;

pub use context_budget::{ContextBudget, ContextItem, ContextFitResult, Priority, TrimmedItem, build_trimmed_placeholders};
pub use few_shot::{FewShotExample, FewShotManager, load_from_database};
pub use response_parser::{parse_response, apply_common_fixes, ParseError, ParsedResponse, AgentAction, ToolCall, ParserConfig};
pub use system_prompt::{build_system_prompt, build_task_prompt, estimate_tokens, GeneratedPrompt, SystemPromptConfig, AgentType, FidelityLevel};
pub use tool_orchestration::ToolOrchestrator;

/// Top-level prompt builder that assembles the full three-layer prompt.
pub struct PromptBuilder {
    config: SystemPromptConfig,
    context_budget: ContextBudget,
    few_shot_manager: FewShotManager,
}

impl PromptBuilder {
    /// Create a new prompt builder.
    pub fn new(config: SystemPromptConfig) -> Self {
        Self {
            config,
            context_budget: ContextBudget::default(),
            few_shot_manager: FewShotManager::new(10, 100),
        }
    }

    /// Set a custom context budget.
    pub fn with_budget(mut self, budget: ContextBudget) -> Self {
        self.context_budget = budget;
        self
    }

    /// Register MCP tool descriptions for inclusion in the prompt.
    pub fn with_tools(mut self, tools: HashMap<String, String>) -> Self {
        self.config.tool_descriptions = tools;
        self.config.include_tools = true;
        self
    }

    /// Set the few-shot manager.
    pub fn with_few_shot(mut self, manager: FewShotManager) -> Self {
        self.few_shot_manager = manager;
        self
    }

    /// Build the complete prompt for a task.
    pub fn build(
        &self,
        task_instruction: &str,
        task_type: Option<&cleanroom_db::repositories::TaskType>,
        dependency_context: &[ContextItem],
        working_set: &[ContextItem],
    ) -> GeneratedPrompt {
        // Layer 1: System prompt
        let system = build_system_prompt(&self.config);

        // Fit context items within budget
        let static_tokens = system.estimated_tokens;
        let task_tokens = estimate_tokens(task_instruction);

        let dep_str: String = dependency_context.iter()
            .map(|i| format!("[{}]: {}", i.priority_label(), i.content))
            .collect::<Vec<_>>()
            .join("\n");

        let ws_str: String = working_set.iter()
            .map(|i| format!("[{}]: {}", i.priority_label(), i.content))
            .collect::<Vec<_>>()
            .join("\n");

        let dep_tokens = estimate_tokens(&dep_str);
        let ws_tokens = estimate_tokens(&ws_str);

        let (dep_fit, ws_fit) = if dep_tokens + ws_tokens <= self.context_budget.remaining_budget(static_tokens, task_tokens) {
            (dep_str, ws_str)
        } else {
            // Trim low-priority items
            let dep_items: Vec<_> = dependency_context.iter()
                .map(|i| context_budget::ContextItem::new(&i.uri, &i.content, i.priority))
                .collect();
            let ws_items: Vec<_> = working_set.iter()
                .map(|i| context_budget::ContextItem::new(&i.uri, &i.content, i.priority))
                .collect();

            let dep_result = self.context_budget.fit_items(static_tokens, task_tokens, dep_items);
            let ws_result = self.context_budget.fit_items(static_tokens + task_tokens + dep_result.total_tokens, 0, ws_items);

            let dep_out: String = dep_result.included.iter()
                .map(|i| format!("[{}] {}", i.uri, i.content))
                .collect::<Vec<_>>()
                .join("\n");
            let trimmed_note = build_trimmed_placeholders(&dep_result.trimmed);

            let ws_out: String = ws_result.included.iter()
                .map(|i| format!("[{}] {}", i.uri, i.content))
                .collect::<Vec<_>>()
                .join("\n");

            (format!("{dep_out}\n{trimmed_note}"), ws_out)
        };

        // Layer 2+3: Context + Task prompt
        let mut prompt = build_task_prompt(
            &system,
            task_instruction,
            &dep_fit,
            &ws_fit,
            &self.context_budget,
        );

        // Inject few-shot examples if available
        if let Some(tt) = task_type {
            let examples = self.few_shot_manager.select(tt, None, 3, 1500);
            if !examples.is_empty() {
                let ex_text = FewShotManager::format_examples(&examples);
                prompt.text.push_str(&ex_text);
                prompt.estimated_tokens += crate::system_prompt::estimate_tokens(&ex_text);
            }
        }

        // Inject task-specific orchestration hints
        if let Some(tt) = task_type {
            let hint = ToolOrchestrator::for_task(tt.as_str(), self.config.agent_type);
            if !hint.is_empty() {
                prompt.text.push_str("\n\n─── ORCHESTRATION HINT ───\n");
                prompt.text.push_str(hint);
                prompt.estimated_tokens += crate::system_prompt::estimate_tokens(hint);
            }
        }

        prompt
    }
}

/// Default tool descriptions for the MCP tools.
pub fn default_tool_descriptions() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let tools: &[(&str, &str)] = &[
        ("create_task", "Create a new analysis or generation task"),
        ("claim_task", "Atomically claim the next pending task"),
        ("update_task_progress", "Update task progress (0.0~1.0)"),
        ("complete_task", "Mark task as complete and submit output"),
        ("fail_task", "Mark task as failed and record error"),
        ("send_heartbeat", "Send heartbeat for a running task"),
        ("get_task", "Get task details by ID"),
        ("list_tasks", "List tasks (filter by status/type/agent)"),
        ("get_data_model", "Get data model with attributes"),
        ("get_contract", "Get contract (interface/class/API)"),
        ("get_function_spec", "Get function specification"),
        ("get_ui_screen", "Get UI screen definition"),
        ("list_documents", "List all S.DEF documents"),
        ("search_sdef", "Full-text search S.DEF content"),
        ("get_dependency_graph", "Get module dependency graph"),
        ("list_shards", "List shards (filter by type/status)"),
        ("resolve_name", "Resolve S.DEF URI to code name"),
        ("batch_resolve_names", "Batch-resolve URIs to names"),
        ("list_symbols", "List registered symbols"),
        ("register_custom_name", "Register a custom symbol name"),
        ("export_sdef", "Export complete S.DEF (JSON/YAML)"),
        ("export_sdef_to_disk", "Export S.DEF to disk as shard tree"),
        ("import_sdef", "Import S.DEF from JSON"),
        ("export_shard", "Export a single shard"),
        ("import_shard", "Import a single shard"),
        ("check_consistency", "Run consistency check"),
        ("compute_fingerprints", "Recompute consistency fingerprints"),
        ("resolve_inconsistency", "Resolve an inconsistency"),
        ("get_inconsistency_report", "Get inconsistency report"),
        ("lsp_initialize", "Initialize LSP server"),
        ("lsp_get_document_symbols", "Get file symbols via LSP"),
        ("lsp_get_type_info", "Get type info at position"),
        ("lsp_find_references", "Find symbol references"),
        ("lsp_get_diagnostics", "Get file diagnostics"),
        ("lsp_get_hierarchy", "Get type hierarchy"),
        ("create_checkpoint", "Create workflow checkpoint"),
        ("list_checkpoints", "List checkpoints"),
        ("restore_checkpoint", "Restore from checkpoint"),
        ("begin_transaction", "Begin DB transaction"),
        ("commit_transaction", "Commit transaction"),
        ("rollback_transaction", "Rollback transaction"),
        ("set_compatibility_mode", "Set compatibility mode"),
        ("list_compat_layers", "List compatibility layers"),
        ("get_compat_layer_detail", "Get compat layer details"),
        ("ignore_compat_layer", "Ignore a compat layer"),
    ];
    for (name, desc) in tools {
        map.insert(name.to_string(), desc.to_string());
    }
    map
}

// ── Priority label helper ───────────────────────────────────────────────

impl ContextItem {
    fn priority_label(&self) -> &'static str {
        match self.priority {
            Priority::Must => "MUST",
            Priority::High => "HIGH",
            Priority::Medium => "MEDIUM",
            Priority::Low => "LOW",
            Priority::Optional => "OPT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_producer_prompt_with_tools() {
        let tools = default_tool_descriptions();
        let config = SystemPromptConfig {
            agent_type: AgentType::Producer,
            ..Default::default()
        };
        let builder = PromptBuilder::new(config).with_tools(tools);
        let prompt = builder.build(
            "Analyze the User entity",
            None,
            &[],
            &[ContextItem::new("sdef://core/User", "struct User { id: u64, name: String }", Priority::Must)],
        );
        assert!(prompt.text.contains("AVAILABLE MCP TOOLS"));
        assert!(prompt.text.contains("lsp_get_type_info"));
        assert!(prompt.text.contains("struct User"));
    }

    #[test]
    fn test_build_with_task_orchestration() {
        let config = SystemPromptConfig {
            agent_type: AgentType::Producer,
            ..Default::default()
        };
        let builder = PromptBuilder::new(config);
        let task_type = cleanroom_db::repositories::TaskType::from_str("EXTRACT_DATA_MODEL").unwrap();
        let prompt = builder.build(
            "Extract data model for User",
            Some(&task_type),
            &[],
            &[],
        );
        assert!(prompt.text.contains("resolve_name"));
        assert!(prompt.text.contains("lsp_get_type_info"));
    }

    #[test]
    fn test_default_tool_descriptions_count() {
        let tools = default_tool_descriptions();
        assert_eq!(tools.len(), 45, "Should have all 45 MCP tool descriptions (some tools have aliased names)");
    }
}
