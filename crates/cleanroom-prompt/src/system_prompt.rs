//! System prompt generation for Producer and Consumer agents.
//!
//! Produces the static Layer 1 of the three-layer prompt architecture:
//! role definition → behavioral rules → output format → MCP tool list.

use std::collections::HashMap;

use crate::context_budget::ContextBudget;
use crate::tool_orchestration::ToolOrchestrator;

/// Agent type determines the role prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentType {
    Producer,
    Consumer,
}

/// Fidelity level for Consumer agents.
#[derive(Debug, Clone)]
pub enum FidelityLevel {
    Prototype,
    ProductionEquivalent,
    BitIdentical,
}

impl std::fmt::Display for FidelityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prototype => write!(f, "prototype"),
            Self::ProductionEquivalent => write!(f, "production_equivalent"),
            Self::BitIdentical => write!(f, "bit_identical"),
        }
    }
}

/// Configuration for system prompt generation.
#[derive(Debug, Clone)]
pub struct SystemPromptConfig {
    pub agent_type: AgentType,
    pub target_language: Option<String>,
    pub fidelity: FidelityLevel,
    pub compatibility_mode: Option<String>,
    /// Whether to include a MCP tool list in the prompt.
    pub include_tools: bool,
    /// MCP tool descriptions (name → description pairs).
    pub tool_descriptions: HashMap<String, String>,
}

impl Default for SystemPromptConfig {
    fn default() -> Self {
        Self {
            agent_type: AgentType::Producer,
            target_language: None,
            fidelity: FidelityLevel::ProductionEquivalent,
            compatibility_mode: Some("full".to_string()),
            include_tools: true,
            tool_descriptions: HashMap::new(),
        }
    }
}

/// Full system prompt with estimated token count.
#[derive(Debug, Clone)]
pub struct GeneratedPrompt {
    pub text: String,
    pub estimated_tokens: usize,
}

/// Build a complete system prompt from configuration.
pub fn build_system_prompt(config: &SystemPromptConfig) -> GeneratedPrompt {
    let mut parts: Vec<String> = Vec::new();

    // 1. Role definition
    parts.push(build_role_prompt(config));

    // 2. Behavioral rules (shared across agent types)
    parts.push(BEHAVIORAL_RULES.to_string());

    // 3. Output format instructions
    parts.push(build_output_format(config));

    // 4. MCP tool list (if requested)
    if config.include_tools && !config.tool_descriptions.is_empty() {
        parts.push(build_tool_list(config));
    }

    // 5. Specific task orchestration hints
    parts.push(ToolOrchestrator::system_hint(config.agent_type));

    let full = parts.join("\n\n");
    let estimated = estimate_tokens(&full);

    GeneratedPrompt { text: full, estimated_tokens: estimated }
}

// ── Role Prompts ────────────────────────────────────────────────────────

fn build_role_prompt(config: &SystemPromptConfig) -> String {
    let common_rules = r#"Rules:
1. Tool-First: Think → Plan tools → Execute → Verify result.
   Never output code or S.DEF JSON directly — always use MCP tools.
2. Atomic: Each claim_task → ... → complete_task must be self-contained.
3. Verify: Before completing, check consistency using available tools.
4. Boundaries: Only use shards loaded in context. Request more via tools."#;

    match config.agent_type {
        AgentType::Producer => format!(
            "You are a software ANALYSIS agent. Your job is to deeply understand \
source code repositories and produce S.DEF (Software Definition Exchange Format) \
specifications.\n\n\
You analyze code by:\n\
1. Using LSP tools to get precise type information for every symbol\n\
2. Tracking cross-file references to build complete dependency graphs\n\
3. Extracting both explicit and implicit design decisions\n\
4. Marking compatibility layers (deprecated APIs, version bridges) with metadata\n\
5. Assigning every entity a unique sdef:// URI\n\
6. Outputting valid S.DEF JSON conforming to the schema\n\n\
{common_rules}"
        ),
        AgentType::Consumer => {
            let lang = config.target_language.as_deref().unwrap_or("rust");
            let compat = config.compatibility_mode.as_deref().unwrap_or("full");
            let fidelity = &config.fidelity;
            format!(
                "You are a software RECONSTRUCTION agent. Your job is to read \
S.DEF specifications and generate functionally equivalent code.\n\n\
Configuration:\n\
- Target language: {lang}\n\
- Compatibility mode: {compat}\n\
- Fidelity level: {fidelity}\n\n\
You generate code by:\n\
1. ALWAYS resolving symbol names through the naming service — never invent names\n\
2. Generating compilable code that exactly matches the specification\n\
3. Respecting compatibility mode when handling deprecated/legacy interfaces\n\
4. Outputting code as structured files (not raw text blocks)\n\
5. Registering all generated symbols before completing a task\n\n\
{common_rules}"
            )
        }
    }
}

// ── Behavioral Rules ────────────────────────────────────────────────────

const BEHAVIORAL_RULES: &str = r#"BEHAVIORAL RULES:

1. Tool-First: Think → Plan tools → Execute → Verify result
   Never output code or S.DEF JSON directly — ALWAYS use MCP tools.

2. Atomicity: Each claim_task → ... → complete_task must be self-contained.
   The next agent must be able to resume from where you left off.

3. Consistency: Before completing a task, verify:
   - All referenced entities exist in symbol_registry (use list_symbols)
   - Fingerprints match across sdef/db/code layers (use check_consistency)
   - No orphaned references

4. Boundaries: Your context window contains only the shards needed for
   the current task. Use get_data_model / get_contract / get_function_spec
   to fetch additional information as needed.

5. Progress: Update progress frequently with update_task_progress.
   Send heartbeats for long-running operations."#;

// ── Output Format ───────────────────────────────────────────────────────

fn build_output_format(config: &SystemPromptConfig) -> String {
    let base = r#"OUTPUT FORMAT:

All task completions must use the following pattern:
- Use complete_task with a structured output JSON object.
- Every output MUST contain an "entities" array listing changed/created entities.
- Use fail_task for error states with a detailed error_message.
- Use claim_task to acquire new work.
- Use check_consistency before completing any generation task."#;

    match config.agent_type {
        AgentType::Producer => format!(
            "{}\n\nFor analysis tasks, the output format is:\n\
{{\n  \"entities\": [\n    {{\n      \"type\": \"data_model|interface|function|design_decision\",\n      \"uri\": \"sdef://...\",\n      \"file\": \"path/to/file\",\n      \"result\": {{ /* S.DEF entity JSON */ }}\n    }}\n  ],\n  \"summary\": \"Brief summary of what was analyzed\"\n}}",
            base
        ),
        AgentType::Consumer => format!(
            "{}\n\nFor generation tasks, the output format is:\n\
{{\n  \"entities\": [\n    {{\n      \"type\": \"generated_file\",\n      \"path\": \"relative/output/path\",\n      \"name\": \"entity_name\",\n      \"language\": \"{}\"\n    }}\n  ],\n  \"files\": [\n    {{\n      \"path\": \"src/models/user.rs\",\n      \"content\": \"// generated code\",\n      \"symbols_registered\": [\"sdef://...\"]\n    }}\n  ],\n  \"verification\": {{\n    \"compiled\": true,\n    \"lints\": 0\n  }}\n}}",
            base,
            config.target_language.as_deref().unwrap_or("rust")
        ),
    }
}

// ── Tool List ───────────────────────────────────────────────────────────

fn build_tool_list(config: &SystemPromptConfig) -> String {
    let mut tools_by_category: HashMap<&str, Vec<&str>> = HashMap::new();

    for name in config.tool_descriptions.keys() {
        let category = categorize_tool(name);
        tools_by_category.entry(category).or_default().push(name);
    }

    let mut lines = vec!["AVAILABLE MCP TOOLS:".to_string()];

    let order = ["Task Management", "S.DEF Query", "Naming", "Import/Export",
                 "Consistency", "LSP", "Checkpoint", "Transaction", "Compatibility"];
    for cat in &order {
        if let Some(tools) = tools_by_category.get(cat) {
            lines.push(format!("\n  {cat}:"));
            for name in tools {
                let desc = config.tool_descriptions.get(*name)
                    .map(|d| d.as_str())
                    .unwrap_or("");
                lines.push(format!("    - {name} — {desc}"));
            }
        }
    }

    lines.join("\n")
}

fn categorize_tool(name: &str) -> &'static str {
    match name {
        "create_task" | "claim_task" | "update_task_progress" | "complete_task"
        | "fail_task" | "send_heartbeat" | "get_task" | "list_tasks" => "Task Management",
        "get_data_model" | "get_contract" | "get_function_spec" | "get_ui_screen"
        | "list_documents" | "search_sdef" | "get_dependency_graph" | "list_shards" => "S.DEF Query",
        "resolve_name" | "batch_resolve_names" | "list_symbols" | "register_custom_name" => "Naming",
        "export_sdef" | "export_sdef_to_disk" | "import_sdef" | "export_shard" | "import_shard" => "Import/Export",
        "check_consistency" | "compute_fingerprints" | "resolve_inconsistency" | "get_inconsistency_report" => "Consistency",
        "lsp_initialize" | "lsp_get_document_symbols" | "lsp_get_type_info"
        | "lsp_find_references" | "lsp_get_diagnostics" | "lsp_get_hierarchy" => "LSP",
        "create_checkpoint" | "list_checkpoints" | "restore_checkpoint" => "Checkpoint",
        "begin_transaction" | "commit_transaction" | "rollback_transaction" => "Transaction",
        "set_compatibility_mode" | "list_compat_layers" | "get_compat_layer_detail" | "ignore_compat_layer" => "Compatibility",
        _ => "Other",
    }
}

// ── Token Estimation ────────────────────────────────────────────────────

/// Simple token estimator: ~4 chars per token for English text,
/// ~1.3 tokens per char for code blocks.
pub fn estimate_tokens(text: &str) -> usize {
    let mut tokens = 0;
    let mut in_code_block = false;

    for line in text.lines() {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            tokens += 1;
            continue;
        }
        if in_code_block {
            tokens += (line.len() as f64 * 1.3) as usize;
        } else {
            tokens += line.len() / 4 + 1;
        }
    }

    tokens.max(1)
}

/// Build a context-aware task prompt that fits within the budget.
pub fn build_task_prompt(
    system_prompt: &GeneratedPrompt,
    task_instruction: &str,
    dependency_context: &str,
    working_set: &str,
    budget: &ContextBudget,
) -> GeneratedPrompt {
    let static_tokens = system_prompt.estimated_tokens;
    let task_tokens = estimate_tokens(task_instruction);
    let remaining = budget.remaining_budget(static_tokens, task_tokens);

    // Fit dependency context + working set within remaining budget
    let dep_tokens = estimate_tokens(dependency_context);
    let ws_tokens = estimate_tokens(working_set);

    let (dep_text, ws_text) = if dep_tokens + ws_tokens <= remaining {
        (dependency_context.to_string(), working_set.to_string())
    } else {
        // Trim low-priority context
        let trimmed = trim_context(dependency_context, working_set, remaining);
        trimmed
    };

    let full = format!(
        "{}\n\n---\n\nDEPENDENCY CONTEXT:\n{}\n\n---\n\nCURRENT TASK:\n{}\n\n---\n\nWORKING SET:\n{}",
        system_prompt.text, dep_text, task_instruction, ws_text,
    );

    GeneratedPrompt {
        estimated_tokens: estimate_tokens(&full),
        text: full,
    }
}

/// Trim context to fit within budget, preserving MUST/HIGH priority.
fn trim_context(dep_context: &str, working_set: &str, budget: usize) -> (String, String) {
    let dep_tokens = estimate_tokens(dep_context);
    let ws_tokens = estimate_tokens(working_set);

    if dep_tokens + ws_tokens <= budget {
        return (dep_context.to_string(), working_set.to_string());
    }

    // If working set alone exceeds budget, summarize it
    if ws_tokens > budget {
        let summary = summarize(working_set, budget);
        return (
            "(Dependency context omitted — use get_data_model / get_contract to retrieve)"
                .to_string(),
            summary,
        );
    }

    // If budget fits working set + some deps
    let dep_budget = budget.saturating_sub(ws_tokens);
    let dep_summary = if dep_budget > estimate_tokens(dep_context) / 2 {
        dep_context.to_string()
    } else {
        format!("⟪ Truncated dependency context ({dep_tokens} tokens). Use MCP tools to retrieve. ⟫\n\n{}", 
            summarize(dep_context, dep_budget))
    };

    (dep_summary, working_set.to_string())
}

/// Naive summarization: take first N chars proportional to budget.
fn summarize(text: &str, budget_tokens: usize) -> String {
    let char_limit = budget_tokens * 4;
    if text.len() <= char_limit {
        return text.to_string();
    }

    // Take first 60% of budget for beginning, 40% for end
    let head_chars = (char_limit as f64 * 0.6) as usize;
    let tail_chars = (char_limit as f64 * 0.4) as usize;

    if let Some(head_end) = text.char_indices().nth(head_chars) {
        let head = &text[..head_end.0];
        let tail_start = text.len().saturating_sub(tail_chars);
        let tail = if let Some(ts) = text.char_indices().nth(tail_start) {
            &text[ts.0..]
        } else {
            ""
        };
        format!("{head}\n\n⟪ ... content trimmed for context budget ... ⟫\n\n{tail}")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_producer_prompt() {
        let config = SystemPromptConfig {
            agent_type: AgentType::Producer,
            include_tools: false,
            ..Default::default()
        };
        let prompt = build_system_prompt(&config);
        assert!(prompt.text.contains("ANALYSIS agent"));
        assert!(prompt.text.contains("Tool-First"));
        assert!(prompt.estimated_tokens > 0);
    }

    #[test]
    fn test_build_consumer_prompt() {
        let config = SystemPromptConfig {
            agent_type: AgentType::Consumer,
            target_language: Some("rust".to_string()),
            compatibility_mode: Some("mixed".to_string()),
            include_tools: false,
            ..Default::default()
        };
        let prompt = build_system_prompt(&config);
        assert!(prompt.text.contains("RECONSTRUCTION agent"));
        assert!(prompt.text.contains("rust"));
        assert!(prompt.text.contains("mixed"));
    }

    #[test]
    fn test_token_estimation() {
        let short = "hello world";
        assert!(estimate_tokens(short) >= 2);

        let code = "fn main() { println!(\"Hello\"); }\n";
        let in_block = format!("```rust\n{}\n```", code);
        let tokens = estimate_tokens(&in_block);
        assert!(tokens > 5, "Code blocks should consume tokens");
    }

    #[test]
    fn test_context_trimming() {
        let dep = "A".repeat(5000);
        let ws = "B".repeat(200);
        let budget = 50; // very small — forces trimming

        let (dep_out, ws_out) = trim_context(&dep, &ws, budget);
        assert!(ws_out.len() <= budget * 4);
        // When ws fits but dep doesn't, dep is omitted entirely
        assert!(
            dep_out.contains("Dependency context omitted"),
            "Expected dep to be omitted: got '{}'",
            &dep_out[..dep_out.len().min(100)]
        );
    }

    #[test]
    fn test_summarize_long_text() {
        let text = "X".repeat(1000);
        let result = summarize(&text, 10);
        assert!(result.len() < 1000);
        assert!(result.contains("trimmed"));
    }
}
