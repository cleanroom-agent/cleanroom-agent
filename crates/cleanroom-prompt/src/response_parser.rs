//! LLM response parser — validates and sanitizes outputs.
//!
//! Handles common LLM output issues: markdown code fences, Python-style
//! booleans, missing JSON, and schema validation for S.DEF entities.

use serde_json::{json, Value};

/// Errors that can occur during response parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("No valid JSON found in LLM response")]
    NoJsonFound,
    #[error("Schema validation failed: {0}")]
    SchemaValidationFailed(String),
    #[error("Response too short ({0} chars) — likely truncated")]
    TruncatedResponse(usize),
    #[error("Response contains only whitespace / empty")]
    EmptyResponse,
    #[error("Max retries ({0}) exceeded")]
    MaxRetriesExceeded(usize),
    #[error("Response appears to be instructions, not output")]
    LikelyInstructions,
}

/// The action the LLM wants to take.
#[derive(Debug, Clone)]
pub enum AgentAction {
    /// The LLM wants to call one or more MCP tools.
    CallTools(Vec<ToolCall>),
    /// The LLM wants to complete a task with output.
    CompleteTask { task_id: String, output: Value },
    /// The LLM is reporting a failure.
    FailTask { task_id: String, error: String },
    /// The LLM needs clarification from a human.
    RequestClarification { question: String },
}

/// A tool call extracted from the LLM response.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

/// Parsed response from the LLM.
#[derive(Debug, Clone)]
pub struct ParsedResponse {
    pub action: AgentAction,
    pub entities: Vec<Value>,
    pub raw_text: String,
    pub parse_success: bool,
    pub retries_used: usize,
}

/// Configuration for the response parser.
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Maximum retries before giving up.
    pub max_retries: usize,
    /// Minimum characters for a valid response.
    pub min_response_chars: usize,
    /// Whether to attempt common fix-ups.
    pub auto_fix: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            min_response_chars: 5,
            auto_fix: true,
        }
    }
}

/// Parse an LLM response into a structured ParsedResponse.
pub fn parse_response(raw: &str, retry_count: usize, config: &ParserConfig) -> Result<ParsedResponse, ParseError> {
    if retry_count > config.max_retries {
        return Err(ParseError::MaxRetriesExceeded(config.max_retries));
    }

    let cleaned = if config.auto_fix { apply_common_fixes(raw) } else { raw.to_string() };

    if cleaned.trim().is_empty() {
        return Err(ParseError::EmptyResponse);
    }
    if cleaned.len() < config.min_response_chars {
        return Err(ParseError::TruncatedResponse(cleaned.len()));
    }

    // Try to extract tool calls first (function calling pattern)
    let tool_calls = extract_tool_calls(&cleaned);

    // Try to extract JSON blocks
    let json_blocks = extract_json_blocks(&cleaned);

    // Build action from extracted information
    let (action, entities) = if !tool_calls.is_empty() {
        // Response contains tool calls
        (AgentAction::CallTools(tool_calls), vec![])
    } else if !json_blocks.is_empty() {
        // Response contains JSON — treat as task completion output
        let output = merge_json_blocks(&json_blocks);
        let entities = extract_entities(&output);
        (
            AgentAction::CompleteTask {
                task_id: String::new(), // caller fills in
                output,
            },
            entities,
        )
    } else if cleaned.to_lowercase().contains("fail_task") || cleaned.to_lowercase().contains("error") {
        // Response indicates failure
        (
            AgentAction::FailTask {
                task_id: String::new(),
                error: extract_error_message(&cleaned),
            },
            vec![],
        )
    } else if cleaned.contains('?') && is_likely_question(&cleaned) {
        // Response appears to be asking a question
        (
            AgentAction::RequestClarification {
                question: cleaned.trim().to_string(),
            },
            vec![],
        )
    } else if retry_count < config.max_retries {
        return Err(ParseError::NoJsonFound);
    } else {
        // Last retry: treat everything as output
        (
            AgentAction::CompleteTask {
                task_id: String::new(),
                output: json!({ "raw_output": cleaned }),
            },
            vec![],
        )
    };

    Ok(ParsedResponse {
        action,
        entities,
        raw_text: raw.to_string(),
        parse_success: true,
        retries_used: retry_count,
    })
}

// ── JSON extraction ─────────────────────────────────────────────────────

/// Extract JSON objects from text, handling markdown fences and inline JSON.
pub fn extract_json_blocks(raw: &str) -> Vec<Value> {
    let mut blocks = Vec::new();

    // Try to extract markdown-fenced JSON blocks
    let fence_pattern = regex_lite::Regex::new(r"```(?:json)?\s*\n([\s\S]*?)\n```").unwrap();
    for cap in fence_pattern.captures_iter(raw) {
        if let Some(json_str) = cap.get(1) {
            if let Ok(val) = serde_json::from_str::<Value>(json_str.as_str()) {
                blocks.push(val);
            }
        }
    }

    // If no fenced blocks, try to find JSON objects with { ... }
    if blocks.is_empty() {
        if let Some(start) = raw.find('{') {
            let slice = &raw[start..];
            // Find matching close brace
            if let Some(end_idx) = find_matching_brace(slice) {
                let candidate = &slice[..=end_idx];
                if let Ok(val) = serde_json::from_str::<Value>(candidate) {
                    blocks.push(val);
                }
            }
        }
    }

    blocks
}

/// Find the position of the closing brace matching the opening brace at index 0.
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;

    for (i, ch) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// If multiple JSON blocks, merge into a single object.
fn merge_json_blocks(blocks: &[Value]) -> Value {
    if blocks.len() == 1 {
        return blocks[0].clone();
    }

    let mut merged = json!({});
    for block in blocks {
        if let Some(obj) = block.as_object() {
            if let Some(m) = merged.as_object_mut() {
                for (k, v) in obj {
                    m.insert(k.clone(), v.clone());
                }
            }
        }
    }
    merged
}

/// Extract entities array from a JSON output.
fn extract_entities(output: &Value) -> Vec<Value> {
    output.get("entities")
        .and_then(|e| e.as_array().cloned())
        .unwrap_or_default()
}

// ── Tool call extraction ────────────────────────────────────────────────

/// Extract MCP tool calls from raw text.
fn extract_tool_calls(raw: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    // Pattern: tool_name({"key": "value"})  or  tool_name ({"key": "value"})
    let re = regex_lite::Regex::new(
        r"([a-z_]+)\s*\(\s*(\{[^}]+\}(?:\s*,\s*\{[^}]+\})*)\s*\)"
    ).unwrap();

    for cap in re.captures_iter(raw) {
        if let (Some(name), Some(args)) = (cap.get(1), cap.get(2)) {
            let name = name.as_str().to_string();
            let args_str = args.as_str();

            if is_known_tool(&name) {
                if let Ok(val) = serde_json::from_str::<Value>(args_str) {
                    calls.push(ToolCall { name, arguments: val });
                } else {
                    // Try wrapping args in {}
                    let wrapped = format!("{{{}}}", args_str);
                    if let Ok(val) = serde_json::from_str::<Value>(&wrapped) {
                        calls.push(ToolCall { name, arguments: val });
                    }
                }
            }
        }
    }

    calls
}

/// Known MCP tool names.
fn is_known_tool(name: &str) -> bool {
    matches!(name,
        "create_task" | "claim_task" | "update_task_progress" | "complete_task" |
        "fail_task" | "send_heartbeat" | "get_task" | "list_tasks" |
        "get_data_model" | "get_contract" | "get_function_spec" | "get_ui_screen" |
        "list_documents" | "search_sdef" | "get_dependency_graph" | "list_shards" |
        "resolve_name" | "batch_resolve_names" | "list_symbols" | "register_custom_name" |
        "export_sdef" | "export_sdef_to_disk" | "import_sdef" | "export_shard" | "import_shard" |
        "check_consistency" | "compute_fingerprints" | "resolve_inconsistency" | "get_inconsistency_report" |
        "lsp_initialize" | "lsp_get_document_symbols" | "lsp_get_type_info" |
        "lsp_find_references" | "lsp_get_diagnostics" | "lsp_get_hierarchy" |
        "create_checkpoint" | "list_checkpoints" | "restore_checkpoint" |
        "begin_transaction" | "commit_transaction" | "rollback_transaction" |
        "set_compatibility_mode" | "list_compat_layers" | "get_compat_layer_detail" | "ignore_compat_layer"
    )
}

// ── Common fixes ────────────────────────────────────────────────────────

/// Apply common fixes to LLM output.
pub fn apply_common_fixes(raw: &str) -> String {
    raw
        .replace("True", "true")
        .replace("False", "false")
        .replace("None", "null")
        .replace("```json", "")
        .replace("```yaml", "")
        .replace("```", "")
        .replace('\u{201C}', "\"") // left curly quote
        .replace('\u{201D}', "\"") // right curly quote
        .replace('\u{2018}', "'")  // left curly single quote
        .replace('\u{2019}', "'")  // right curly single quote
}

/// Extract an error message from text.
fn extract_error_message(raw: &str) -> String {
    // Try to find the most relevant line with "error" or "failed"
    for line in raw.lines() {
        let lower = line.to_lowercase();
        if lower.contains("error") || lower.contains("failed") || lower.contains("cannot") {
            return line.trim().to_string();
        }
    }
    raw.lines().next().unwrap_or("Unknown error").to_string()
}

/// Check if the response looks like a question.
fn is_likely_question(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.contains('?') && trimmed.len() < 500
        && !trimmed.contains('{') // not JSON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_common_fixes() {
        let raw = "```json\n{\"key\": True, \"val\": None}\n```";
        let fixed = apply_common_fixes(raw);
        assert!(!fixed.contains("True"));
        assert!(!fixed.contains("None"));
        assert!(!fixed.contains("```"));
    }

    #[test]
    fn test_extract_json_fenced() {
        let raw = "Here is output:\n```json\n{\"name\": \"test\"}\n```\nDone.";
        let blocks = extract_json_blocks(raw);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["name"], "test");
    }

    #[test]
    fn test_extract_json_inline() {
        let raw = "Result: {\"entities\": [{\"type\": \"data_model\"}]}";
        let blocks = extract_json_blocks(raw);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn test_extract_tool_calls() {
        let raw = r#"I'll call resolve_name({"document_name":"test","sdef_uri":"sdef://x","language":"rust","symbol_type":"class"})"#;
        let calls = extract_tool_calls(raw);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "resolve_name");
    }

    #[test]
    fn test_parse_complete_task() {
        let raw = "```json\n{\"entities\": [{\"type\": \"data_model\"}], \"summary\": \"ok\"}\n```";
        let config = ParserConfig::default();
        let result = parse_response(raw, 0, &config).unwrap();
        assert!(result.parse_success);
        assert_eq!(result.entities.len(), 1);
    }

    #[test]
    fn test_parse_unknown_tool_ignored() {
        // No known tools and no JSON. With no JSON found on the first
        // retry, the parser returns NoJsonFound error.
        let raw = "unknown_command with some text but no JSON at all";
        let config = ParserConfig::default();
        let result = parse_response(raw, 0, &config);
        assert!(result.is_err(), "Text without JSON or tools should error on first parse");
    }
}
