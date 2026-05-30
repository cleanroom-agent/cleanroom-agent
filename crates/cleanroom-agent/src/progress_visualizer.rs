//! Progress visualization for CLI workflows.
//!
//! Implements §7 of docs/15-user-interaction.md: renders ASCII progress bars
//! and task status summaries from the database.

use cleanroom_db::{Database, DbError, TaskRepository, TaskStatus};
use std::sync::Arc;

/// Renders progress information for a workflow.
pub struct ProgressVisualizer;

impl ProgressVisualizer {
    /// Render workflow progress as an ASCII progress bar with task counts.
    ///
    /// Returns a formatted string suitable for terminal display.
    pub fn render_workflow_progress(
        db: &Arc<Database>,
        _document_name: Option<&str>,
    ) -> Result<String, DbError> {
        let task_repo = TaskRepository::new(db.connection_arc());

        let tasks = task_repo.list(None, None, None)?;

        if tasks.is_empty() {
            return Ok("No tasks in queue.".to_string());
        }

        let completed = tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let in_progress = tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
        let pending = tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();
        let failed = tasks.iter()
            .filter(|t| t.status == TaskStatus::Failed || t.status == TaskStatus::FailedPermanently)
            .count();
        let total = tasks.len();
        let processed = completed + failed;

        let bar_len = 30usize;
        let filled = if total > 0 {
            (processed as f64 / total as f64 * bar_len as f64) as usize
        } else {
            0
        };
        let bar: String = "█".repeat(filled.min(bar_len))
            + &"░".repeat(bar_len.saturating_sub(filled));

        let pct = if total > 0 {
            (processed as f64 / total as f64 * 100.0)
        } else {
            0.0
        };

        Ok(format!(
            "\n═══ Workflow Progress ═══\n\
             Progress: [{bar}] {processed}/{total} ({pct:.1}%)\n\
             \n\
             ▸ Completed:  {completed}\n\
             ▸ In Progress: {in_progress}\n\
             ▸ Pending:    {pending}\n\
             ▸ Failed:     {failed}\n\
             ═══ End ═══\n",
            bar = bar,
            processed = processed,
            total = total,
            pct = pct,
            completed = completed,
            in_progress = in_progress,
            pending = pending,
            failed = failed,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleanroom_db::Database;

    #[test]
    fn test_render_empty_queue() {
        let db = Arc::new(Database::in_memory().unwrap());
        let result = ProgressVisualizer::render_workflow_progress(&db, None).unwrap();
        assert!(result.contains("No tasks"));
    }
}
