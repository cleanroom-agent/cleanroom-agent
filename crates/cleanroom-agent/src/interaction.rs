//! Interactive CLI modes — guided and interactive workflows.
//!
//! Implements §2-3 of docs/15-user-interaction.md:
//!
//! - **Guided mode** (`--guided`): Agent proposes decisions, user approves,
//!   then agent executes. Decision points include design decisions and
//!   compatibility layer handling.
//! - **Interactive mode** (`--interactive`): Agent asks clarification
//!   questions and adjusts analysis based on answers.
//!
//! # State Machine
//!
//! ```text
//! Pending → AwaitingReview → Accepted | ModificationsRequested | Rejected
//! ModificationsRequested → Revise → AwaitingReview
//! Accepted → execute
//! ```

use std::io::{self, Write};

/// Interaction mode selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionMode {
    /// Fully automatic — no user intervention.
    Automatic,
    /// Guided — user approves decisions at key points.
    Guided,
    /// Interactive — agent asks questions, user answers.
    Interactive,
}

/// A reviewable item presented to the user.
#[derive(Debug, Clone)]
pub enum ReviewItem {
    /// A design decision inferred by the agent.
    DesignDecision {
        topic: String,
        proposal: String,
        rationale: String,
        confidence: f64,
        alternatives: Vec<String>,
    },
    /// A compatibility layer that needs user direction.
    CompatibilityLayer {
        name: String,
        description: String,
    },
    /// A clarification question from the agent.
    Clarification {
        question: String,
        options: Vec<String>,
    },
}

/// User response to a review item.
#[derive(Debug, Clone)]
pub enum UserDecision {
    /// Accept the proposal as-is.
    Accept,
    /// Accept with modifications (new proposal text).
    Modify(String),
    /// Reject the proposal.
    Reject,
    /// Select one of the pre-defined options.
    Select(String),
}

/// A question-answer pair recorded during an interactive session.
#[derive(Debug, Clone)]
pub struct QAPair {
    pub question: String,
    pub answer: String,
}

/// Accumulator for answers collected during interactive mode.
#[derive(Debug, Default)]
pub struct InteractiveContext {
    pub qa_history: Vec<QAPair>,
    pub decisions_accepted: Vec<String>,
    pub decisions_rejected: Vec<String>,
}

impl InteractiveContext {
    pub fn record_answer(&mut self, question: &str, answer: &str) {
        self.qa_history.push(QAPair {
            question: question.to_string(),
            answer: answer.to_string(),
        });
    }
}

/// Prompt the user via stdin and return their choice.
///
/// The prompt is written to stderr so stdout can be used for structured output.
pub fn prompt_user(prompt: &str) -> io::Result<String> {
    eprint!("{} ", prompt);
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Present a review item to the user (guided mode) and get their decision.
///
/// Writes the item description to stderr and reads a single-character or
/// free-text response from stdin.
pub fn present_for_review(item: &ReviewItem, interactive: bool) -> io::Result<UserDecision> {
    match item {
        ReviewItem::DesignDecision { topic, proposal, rationale, confidence, .. } => {
            eprintln!();
            eprintln!("─── Design Decision Detected ───");
            eprintln!("  Topic: {}", topic);
            eprintln!("  Proposal: {}", proposal);
            eprintln!("  Rationale: {}", rationale);
            eprintln!("  Confidence: {:.0}%", confidence * 100.0);

            if interactive {
                let answer = prompt_user("Accept this decision? [Y/n/edit]:")?;
                match answer.to_lowercase().as_str() {
                    "" | "y" | "yes" => Ok(UserDecision::Accept),
                    "n" | "no" => Ok(UserDecision::Reject),
                    _ if answer.starts_with("edit") || answer.starts_with("e") => {
                        let edited = prompt_user("Enter updated proposal:")?;
                        Ok(UserDecision::Modify(edited))
                    }
                    _ => Ok(UserDecision::Reject),
                }
            } else {
                // Guided mode — simpler yes/no
                let answer = prompt_user("Accept? [Y/n]:")?;
                match answer.to_lowercase().as_str() {
                    "" | "y" | "yes" => Ok(UserDecision::Accept),
                    _ => Ok(UserDecision::Reject),
                }
            }
        }
        ReviewItem::CompatibilityLayer { name, description } => {
            eprintln!();
            eprintln!("─── Compatibility Layer Detected ───");
            eprintln!("  Name: {}", name);
            eprintln!("  Description: {}", description);
            eprintln!();
            eprintln!("  What should we do with it?");
            eprintln!("  [K] Keep (preserve in S.DEF for full fidelity)");
            eprintln!("  [M] Mark deprecated (keep but flag)");
            eprintln!("  [R] Remove (treat as dead code)");

            let answer = prompt_user("Choice:")?;
            match answer.to_lowercase().as_str() {
                "k" | "keep" => Ok(UserDecision::Accept),
                "m" | "mark" => Ok(UserDecision::Modify("deprecated".to_string())),
                "r" | "remove" => Ok(UserDecision::Reject),
                _ => Ok(UserDecision::Accept),
            }
        }
        ReviewItem::Clarification { question, options } => {
            eprintln!();
            eprintln!("─── Agent Clarification ───");
            eprintln!("  Question: {}", question);
            if !options.is_empty() {
                for (i, opt) in options.iter().enumerate() {
                    eprintln!("    [{}] {}", i + 1, opt);
                }
            }

            let answer = prompt_user("Your answer:")?;
            Ok(UserDecision::Select(answer))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_context_records_qa() {
        let mut ctx = InteractiveContext::default();
        ctx.record_answer("What is the database?", "PostgreSQL");
        assert_eq!(ctx.qa_history.len(), 1);
        assert_eq!(ctx.qa_history[0].question, "What is the database?");
        assert_eq!(ctx.qa_history[0].answer, "PostgreSQL");
    }

    #[test]
    fn test_interaction_mode_default() {
        assert_eq!(InteractionMode::Automatic, InteractionMode::Automatic);
    }
}
