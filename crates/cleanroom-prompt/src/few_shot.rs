//! Few-shot example management.
//!
//! Stores successful task outputs and injects them as examples
//! into prompts to improve LLM performance on similar tasks.

use std::collections::HashMap;

use chrono::Utc;
use cleanroom_db::repositories::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A recorded example of a successful task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewShotExample {
    /// Unique identifier.
    pub id: String,
    /// Task type this example is for.
    pub task_type: String,
    /// Optional language tag for language-specific examples.
    pub language: Option<String>,
    /// The task input (task input_json).
    pub input: Value,
    /// The successful output (task output_json).
    pub output: Value,
    /// Tool calls made during this task (in order).
    pub tool_trace: Vec<String>,
    /// When this example was recorded.
    pub recorded_at: String,
    /// Token count of the example (input + output + trace).
    pub estimated_tokens: usize,
}

/// Manager for few-shot examples.
#[derive(Debug, Default)]
pub struct FewShotManager {
    /// All stored examples, keyed by task_type → language.
    examples: HashMap<String, Vec<FewShotExample>>,
    /// Maximum examples per (task_type, language) pair.
    max_per_category: usize,
    /// Maximum total examples to store.
    max_total: usize,
}

impl FewShotManager {
    /// Create a new manager with limits.
    pub fn new(max_per_category: usize, max_total: usize) -> Self {
        Self {
            examples: HashMap::new(),
            max_per_category,
            max_total,
        }
    }

    /// Record a successful task as a new few-shot example.
    pub fn record(
        &mut self,
        task_type: &TaskType,
        language: Option<&str>,
        input: Value,
        output: Value,
        tool_trace: Vec<String>,
    ) {
        let key = make_key(task_type, language);
        let recorded_at = Utc::now().to_rfc3339();
        let estimated_tokens = estimate_example_tokens(&input, &output, &tool_trace);

        let example = FewShotExample {
            id: uuid::Uuid::new_v4().to_string(),
            task_type: task_type.as_str().to_string(),
            language: language.map(|s| s.to_string()),
            input,
            output,
            tool_trace,
            recorded_at,
            estimated_tokens,
        };

        let entries = self.examples.entry(key).or_default();

        // Prune oldest if exceeding per-category limit
        if entries.len() >= self.max_per_category {
            entries.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
            entries.truncate(self.max_per_category - 1);
        }

        entries.push(example);

        // Global prune — collect counts before mutating
        let total: usize = self.examples.values().map(|v| v.len()).sum();
        if total > self.max_total {
            self.prune_oldest();
        }
    }

    /// Select the best matching examples for a task type and language.
    pub fn select(
        &self,
        task_type: &TaskType,
        language: Option<&str>,
        max_count: usize,
        max_total_tokens: usize,
    ) -> Vec<&FewShotExample> {
        let mut candidates: Vec<&FewShotExample> = Vec::new();

        // Exact match: same task_type, same language
        let key = make_key(task_type, language);
        if let Some(entries) = self.examples.get(&key) {
            candidates.extend(entries);
        }

        // Fallback: same task_type, any language (if not enough)
        if candidates.len() < max_count {
            let fallback_key = make_key(task_type, None);
            if let Some(entries) = self.examples.get(&fallback_key) {
                for e in entries {
                    if !candidates.iter().any(|c| c.id == e.id) {
                        candidates.push(e);
                    }
                }
            }
        }

        // Sort by recency (newest first)
        candidates.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));

        // Fit within token budget
        let mut selected = Vec::new();
        let mut tokens = 0;
        for ex in candidates.iter().take(max_count) {
            if tokens + ex.estimated_tokens <= max_total_tokens {
                tokens += ex.estimated_tokens;
                selected.push(*ex);
            } else {
                break;
            }
        }

        selected
    }

    /// Format selected examples into a prompt-ready string.
    pub fn format_examples(examples: &[&FewShotExample]) -> String {
        if examples.is_empty() {
            return String::new();
        }

        let mut lines = vec!["\n─── FEW-SHOT EXAMPLES ───\n".to_string()];

        for (i, ex) in examples.iter().enumerate() {
            lines.push(format!(
                "【Example {} of successful {} task】",
                i + 1,
                ex.task_type,
            ));

            lines.push(format!(
                "  Input: {}",
                serde_json::to_string_pretty(&ex.input).unwrap_or_default()
            ));

            if !ex.tool_trace.is_empty() {
                lines.push(format!(
                    "  Tool calls made: {}",
                    ex.tool_trace.join(" → ")
                ));
            }

            lines.push(format!(
                "  Output pattern: {}",
                summarize_output(&ex.output)
            ));
        }

        lines.push(String::new());
        lines.join("\n")
    }

    /// Count total stored examples.
    pub fn total_count(&self) -> usize {
        self.examples.values().map(|v| v.len()).sum()
    }

    /// Clear all examples.
    pub fn clear(&mut self) {
        self.examples.clear();
    }

    fn prune_oldest(&mut self) {
        // Collect all examples and their IDs first (avoids borrow conflict)
        let mut all: Vec<(String, String)> = self.examples
            .values()
            .flatten()
            .map(|e| (e.id.clone(), e.recorded_at.clone()))
            .collect();
        all.sort_by(|a, b| a.1.cmp(&b.1));

        // Remove the oldest 20%
        let to_remove = (all.len() as f64 * 0.2).ceil() as usize;
        let remove_ids: Vec<String> = all.iter()
            .take(to_remove)
            .map(|(id, _)| id.clone())
            .collect();

        for entries in self.examples.values_mut() {
            entries.retain(|e| !remove_ids.contains(&e.id));
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn make_key(task_type: &TaskType, language: Option<&str>) -> String {
    format!(
        "{}_{}",
        task_type.as_str(),
        language.unwrap_or("*")
    )
}

fn estimate_example_tokens(input: &Value, output: &Value, trace: &[String]) -> usize {
    let input_str = serde_json::to_string(input).unwrap_or_default();
    let output_str = serde_json::to_string(output).unwrap_or_default();
    let trace_str = trace.join(" ");

    crate::system_prompt::estimate_tokens(&input_str)
        + crate::system_prompt::estimate_tokens(&output_str)
        + crate::system_prompt::estimate_tokens(&trace_str)
}

fn summarize_output(output: &Value) -> String {
    let entities = output
        .get("entities")
        .and_then(|e| e.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let files = output
        .get("files")
        .and_then(|f| f.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    format!("{entities} entities, {files} files")
}

/// Load few-shot examples from completed tasks in the database.
pub fn load_from_database(
    db: &cleanroom_db::Database,
    max_examples: usize,
) -> FewShotManager {
    let mut manager = FewShotManager::new(10, max_examples);
    let conn = db.connection();

    if let Ok(mut stmt) = conn.prepare(
        "SELECT task_type, input_json, output_json, completed_at
         FROM tasks
         WHERE status = 'completed'
           AND output_json IS NOT NULL
           AND output_json != ''
         ORDER BY completed_at DESC
         LIMIT ?1",
    ) {
        let _ = stmt.query_map(
            [max_examples],
            |row| {
                let task_type_str: String = row.get(0)?;
                let input_str: String = row.get(1)?;
                let output_str: String = row.get(2)?;

                if let (Some(task_type), Ok(input), Ok(output)) = (
                    TaskType::from_str(&task_type_str),
                    serde_json::from_str::<Value>(&input_str),
                    serde_json::from_str::<Value>(&output_str),
                ) {
                    manager.record(
                        &task_type,
                        None,
                        input,
                        output,
                        vec![],
                    );
                }
                Ok(())
            },
        );
    }

    manager
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_record_and_select() {
        let mut mgr = FewShotManager::new(10, 100);

        let tt = TaskType::ExtractDataModel;
        mgr.record(&tt, Some("rust"),
            json!({"entity": "User"}),
            json!({"entities": [{"type": "data_model"}], "files": [{"path": "user.rs"}]}),
            vec!["lsp_get_document_symbols".into(), "complete_task".into()],
        );

        let selected = mgr.select(&tt, Some("rust"), 3, 5000);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].task_type, "EXTRACT_DATA_MODEL");
    }

    #[test]
    fn test_format_examples() {
        let examples = vec![FewShotExample {
            id: "1".into(),
            task_type: "EXTRACT_DATA_MODEL".into(),
            language: None,
            input: json!({"entity": "User"}),
            output: json!({"entities": [{"type": "data_model"}]}),
            tool_trace: vec!["lsp_get_document_symbols".into(), "complete_task".into()],
            recorded_at: Utc::now().to_rfc3339(),
            estimated_tokens: 100,
        }];

        let ex_refs: Vec<&FewShotExample> = examples.iter().collect();
        let formatted = FewShotManager::format_examples(&ex_refs);
        assert!(formatted.contains("EXTRACT_DATA_MODEL"));
        assert!(formatted.contains("lsp_get_document_symbols"));
    }

    #[test]
    fn test_prune_oldest() {
        let mut mgr = FewShotManager::new(2, 5);
        let tt = TaskType::ExtractArchitecture;

        for i in 0..3 {
            mgr.record(&tt, None, json!({"i": i}), json!({"result": "ok"}), vec![]);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let selected = mgr.select(&tt, None, 10, 10000);
        assert!(selected.len() >= 2, "Expected at least 2 examples after pruning");
    }
}
