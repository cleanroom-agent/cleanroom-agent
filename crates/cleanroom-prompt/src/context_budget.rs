//! Context window budget management.
//!
//! Controls how much context is loaded per task by estimating token
//! counts and applying priority-based trimming when the budget overflows.

use serde::{Deserialize, Serialize};

/// Priority level for context items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Critical — the core shard being worked on. Never trimmed.
    Must = 0,
    /// Direct dependencies (data models, interfaces).
    High = 1,
    /// Indirect dependencies (symbol tables for upstream modules).
    Medium = 2,
    /// Related but non-essential info (design decisions, metadata).
    Low = 3,
    /// Optional history (previous task summaries).
    Optional = 4,
}

/// A single context item with its estimated token count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub uri: String,
    pub content: String,
    pub priority: Priority,
    pub estimated_tokens: usize,
}

impl ContextItem {
    pub fn new(uri: impl Into<String>, content: impl Into<String>, priority: Priority) -> Self {
        let content = content.into();
        let estimated_tokens = crate::system_prompt::estimate_tokens(&content);
        Self { uri: uri.into(), content, priority, estimated_tokens }
    }
}

/// The context budget: how many tokens can be loaded per task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    /// Maximum tokens in a single task context.
    pub max_tokens: usize,
    /// Reserved for tool responses (so the LLM can see the output).
    pub tool_response_reserve: usize,
    /// Safety margin: effective budget = max_tokens * safety_margin.
    pub safety_margin: f64,
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            max_tokens: 22000,
            tool_response_reserve: 2000,
            safety_margin: 0.85,
        }
    }
}

impl ContextBudget {
    /// The effective maximum, accounting for safety margin.
    pub fn effective_max(&self) -> usize {
        (self.max_tokens as f64 * self.safety_margin) as usize
    }

    /// Check whether adding a shard would exceed budget.
    pub fn can_fit(&self, current_tokens: usize, shard_tokens: usize) -> bool {
        current_tokens + shard_tokens + self.tool_response_reserve <= self.effective_max()
    }

    /// How many tokens are still available.
    pub fn remaining_budget(&self, static_tokens: usize, task_tokens: usize) -> usize {
        self.effective_max()
            .saturating_sub(static_tokens + task_tokens + self.tool_response_reserve)
    }

    /// Fit a list of context items within budget, trimming lowest-priority items first.
    pub fn fit_items(
        &self,
        static_tokens: usize,
        task_tokens: usize,
        items: Vec<ContextItem>,
    ) -> ContextFitResult {
        let remaining = self.remaining_budget(static_tokens, task_tokens);
        let mut included = Vec::new();
        let mut trimmed = Vec::new();
        let mut total = 0;

        // Sort by priority (Must first, Optional last)
        let mut sorted = items;
        sorted.sort_by_key(|item| item.priority);

        for item in sorted {
            if item.priority == Priority::Must || total + item.estimated_tokens <= remaining {
                total += item.estimated_tokens;
                included.push(item);
            } else {
                // Record what was trimmed so the prompt can reference it
                trimmed.push(TrimmedItem {
                    uri: item.uri.clone(),
                    priority: item.priority,
                    count_hint: None,
                });
            }
        }

        // Collapse consecutive trimmed items of same priority
        let collapsed = collapse_trimmed(trimmed);

        ContextFitResult {
            included,
            trimmed: collapsed,
            total_tokens: total,
            budget_used_pct: if self.effective_max() > 0 {
                (total + static_tokens + task_tokens) as f64 / self.effective_max() as f64
            } else {
                1.0
            },
        }
    }
}

/// The result of fitting items into a context budget.
#[derive(Debug, Clone)]
pub struct ContextFitResult {
    pub included: Vec<ContextItem>,
    pub trimmed: Vec<TrimmedItem>,
    pub total_tokens: usize,
    pub budget_used_pct: f64,
}

/// A trimmed item: was excluded due to budget constraints.
#[derive(Debug, Clone)]
pub struct TrimmedItem {
    pub uri: String,
    pub priority: Priority,
    /// How many items of the same type were trimmed (set after collapsing).
    pub count_hint: Option<usize>,
}

impl TrimmedItem {
    /// Generate a human-readable placeholder for the trimmed context.
    pub fn to_placeholder(&self) -> String {
        match self.count_hint {
            Some(n) if n > 1 => format!(
                "⟪ {} items of priority {:?} omitted (budget). Use MCP tools to retrieve. ⟫",
                n, self.priority,
            ),
            _ => format!(
                "⟪ Item '{}' omitted (budget). Use MCP tools to retrieve. ⟫",
                self.uri,
            ),
        }
    }
}

/// Collapse consecutive trimmed items of same priority into a count hint.
fn collapse_trimmed(items: Vec<TrimmedItem>) -> Vec<TrimmedItem> {
    if items.is_empty() {
        return items;
    }

    let mut result = Vec::new();
    let mut current_priority = items[0].priority;
    let mut current_count = 0;

    for item in items {
        if item.priority == current_priority {
            current_count += 1;
        } else {
            if current_count > 0 {
                result.push(TrimmedItem {
                    uri: format!("collapsed_{:?}", current_priority),
                    priority: current_priority,
                    count_hint: Some(current_count),
                });
            }
            current_priority = item.priority;
            current_count = 1;
        }
    }

    if current_count > 0 {
        result.push(TrimmedItem {
            uri: format!("collapsed_{:?}", current_priority),
            priority: current_priority,
            count_hint: Some(current_count),
        });
    }

    result
}

/// Build placeholders for trimmed items.
pub fn build_trimmed_placeholders(trimmed: &[TrimmedItem]) -> String {
    if trimmed.is_empty() {
        return String::new();
    }

    let mut lines = vec!["\n--- Context Budget Exceeded ---".to_string()];
    for item in trimmed {
        lines.push(item.to_placeholder());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(uri: &str, content: &str, priority: Priority) -> ContextItem {
        ContextItem::new(uri, content, priority)
    }

    #[test]
    fn test_budget_defaults() {
        let budget = ContextBudget::default();
        assert_eq!(budget.effective_max(), 18700); // 22000 * 0.85 = 18700
    }

    #[test]
    fn test_can_fit() {
        let budget = ContextBudget::default();
        assert!(budget.can_fit(0, 15000));
        assert!(!budget.can_fit(0, 20000));
    }

    #[test]
    fn test_fit_items_keeps_must() {
        let budget = ContextBudget::default();
        // Must item is always included regardless of size.
        // Create an Optional item that's large enough to overflow.
        let items = vec![
            make_item("sdef://core", &"X".repeat(2000), Priority::Must),
            make_item("sdef://extra", &"X".repeat(100000), Priority::Optional),
        ];

        let result = budget.fit_items(5000, 1000, items);
        // Must item is always included (even over budget)
        assert_eq!(result.included.len(), 1, "Must item should be included");
        // Optional item is trimmed because it exceeds budget
        assert_eq!(result.trimmed.len(), 1, "Optional item should be trimmed");
    }

    #[test]
    fn test_trimmed_placeholders() {
        let trimmed = vec![
            TrimmedItem { uri: "sdef://a".into(), priority: Priority::Low, count_hint: Some(3) },
        ];
        let text = build_trimmed_placeholders(&trimmed);
        assert!(text.contains("3 items"));
        assert!(text.contains("Low"));
        assert!(text.contains("budget"));
    }
}
