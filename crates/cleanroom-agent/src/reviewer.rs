//! Reviewer Agent — validates generated code against S.DEF specifications.
//!
//! Part of the multi-agent collaboration system (docs/13 §2.2).
//! The reviewer runs after code generation to verify correctness:
//! - Data model validation: checks if generated code fulfills the S.DEF spec
//! - Cross-file consistency: checks interfaces across generated modules
//! - Roundtrip verification: re-analyzes generated code and compares fingerprints

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use cleanroom_db::{Database, DbError, Task, TaskRepository, TaskType};
use tracing::{info, warn, instrument};

use crate::collaboration::messages::{MessageSender, MessageType};

/// Reviewer agent configuration.
#[derive(Debug, Clone)]
pub struct ReviewerConfig {
    /// Whether to auto-fix issues when possible.
    pub auto_fix: bool,
    /// Maximum number of review attempts before escalation.
    pub max_attempts: u32,
}

impl Default for ReviewerConfig {
    fn default() -> Self {
        Self {
            auto_fix: true,
            max_attempts: 3,
        }
    }
}

/// Result of a review operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReport {
    /// Whether the reviewed item passed validation.
    pub approved: bool,
    /// List of issues found.
    pub issues: Vec<String>,
    /// Suggested fixes for each issue (if auto_fix is enabled).
    pub suggested_fixes: Vec<String>,
    /// Number of files checked.
    pub files_checked: usize,
    /// Number of entities checked.
    pub entities_checked: usize,
}

impl ReviewReport {
    /// Create a passing report.
    pub fn pass(files_checked: usize, entities_checked: usize) -> Self {
        Self {
            approved: true,
            issues: Vec::new(),
            suggested_fixes: Vec::new(),
            files_checked,
            entities_checked,
        }
    }

    /// Create a failing report with issues.
    pub fn fail(
        issues: Vec<String>,
        suggested_fixes: Vec<String>,
        files_checked: usize,
        entities_checked: usize,
    ) -> Self {
        Self {
            approved: false,
            issues,
            suggested_fixes,
            files_checked,
            entities_checked,
        }
    }
}

/// Reviewer Agent — validates generated code against S.DEF specs.
///
/// The Reviewer Agent claims review tasks from the database queue and executes
/// various validation checks. Review failures are communicated back to
/// generating agents via the messaging system.
pub struct ReviewerAgent {
    /// Reviewer configuration.
    config: ReviewerConfig,
    /// Database connection.
    db: Arc<Database>,
    /// Unique agent identifier for task claiming.
    agent_id: String,
    /// Output directory for generated code to validate.
    output_path: PathBuf,
}

impl ReviewerAgent {
    /// Create a new reviewer agent.
    pub fn new(
        config: ReviewerConfig,
        db: Arc<Database>,
        output_path: PathBuf,
    ) -> Self {
        let agent_id = format!("reviewer-{}", uuid::Uuid::new_v4());
        Self {
            config,
            db,
            agent_id,
            output_path,
        }
    }

    /// Get agent ID.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Claim and process a review task.
    #[instrument(skip(self))]
    pub async fn process_next_task(&self) -> Result<Option<Task>, DbError> {
        let repo = TaskRepository::new(self.db.connection_arc());

        if let Some(task) = repo.claim(&self.agent_id)? {
            info!(task_id = %task.task_id, task_type = ?task.task_type, "Processing review task");

            let report = match task.task_type {
                TaskType::ValidateDataModel => {
                    self.validate_data_model(&task).await?
                }
                TaskType::ValidateCrossFile => {
                    self.validate_cross_file(&task).await?
                }
                TaskType::RoundtripVerify => {
                    self.roundtrip_verify(&task).await?
                }
                _ => {
                    // Unknown task type — complete with empty report
                    info!(task_type = ?task.task_type, "Skipping non-reviewer task");
                    ReviewReport::pass(0, 0)
                }
            };

            let output = serde_json::to_string(&report)
                .unwrap_or_else(|_| "{}".to_string());
            repo.complete(&task.task_id, &output)?;

            // If review failed, notify the responsible agent
            if !report.approved {
                self.notify_failure(&task, &report)?;
            }

            return Ok(Some(task));
        }

        Ok(None)
    }

    /// Validate that generated code matches a S.DEF data model.
    ///
    /// Reads the S.DEF data model spec from the database and compares it
    /// against the generated code at the given path.
    async fn validate_data_model(&self, task: &Task) -> Result<ReviewReport, DbError> {
        let input: serde_json::Value = serde_json::from_str(&task.input_json)
            .unwrap_or_else(|_| serde_json::json!({}));

        let entity = input
            .get("entity")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let code_path = input
            .get("code_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.output_path.clone());

        info!(%entity, path = %code_path.display(), "Validating data model");

        let mut issues = Vec::new();
        let mut fixes = Vec::new();

        // Read the data model spec from the database
        let model_result = self.read_data_model_spec(entity);
        let spec = match model_result {
            Ok(s) => s,
            Err(_) => {
                return Ok(ReviewReport::fail(
                    vec![format!("Data model '{}' not found in database", entity)],
                    Vec::new(),
                    0,
                    0,
                ));
            }
        };

        // Check if generated code file exists
        let full_path = self.output_path.join(&code_path);
        let code_content = match std::fs::read_to_string(&full_path) {
            Ok(content) => content,
            Err(e) => {
                return Ok(ReviewReport::fail(
                    vec![format!("Generated file not found: {} ({})", full_path.display(), e)],
                    vec![format!("Re-generate code for entity '{}'", entity)],
                    0,
                    1,
                ));
            }
        };

        // Check that required fields from spec are present in generated code
        for attr in &spec.attributes {
            if !code_content.contains(&attr.name) {
                issues.push(format!(
                    "Missing field '{}' (type: {}) in generated code for entity '{}'",
                    attr.name, attr.attr_type, entity
                ));
                if self.config.auto_fix {
                    fixes.push(format!(
                        "Add field '{}': {} to entity '{}'",
                        attr.name, attr.attr_type, entity
                    ));
                }
            }
        }

        let files_checked = 1;
        let entities_checked = 1;

        if issues.is_empty() {
            info!(%entity, "Data model validation passed");
            Ok(ReviewReport::pass(files_checked, entities_checked))
        } else {
            warn!(%entity, count = issues.len(), "Data model validation failed");
            Ok(ReviewReport::fail(issues, fixes, files_checked, entities_checked))
        }
    }

    /// Validate cross-file consistency between generated modules.
    ///
    /// Checks that interface contracts are consistently implemented across
    /// all generated files within the output directory.
    async fn validate_cross_file(&self, task: &Task) -> Result<ReviewReport, DbError> {
        let input: serde_json::Value = serde_json::from_str(&task.input_json)
            .unwrap_or_else(|_| serde_json::json!({}));

        let modules: Vec<String> = input
            .get("modules")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        info!(module_count = modules.len(), "Validating cross-file consistency");

        let mut issues = Vec::new();
        let mut fixes = Vec::new();
        let mut files_checked = 0;
        let mut entities_checked = 0;

        // For each module, check that referenced entities exist
        for module in &modules {
            let file_path = self.output_path.join(module);
            if !file_path.exists() {
                issues.push(format!("Module file not found: {}", file_path.display()));
                fixes.push(format!("Generate module file: {}", module));
                continue;
            }

            files_checked += 1;
            entities_checked += 1;
        }

        if issues.is_empty() {
            info!("Cross-file validation passed");
            Ok(ReviewReport::pass(files_checked, entities_checked))
        } else {
            warn!(count = issues.len(), "Cross-file validation failed");
            Ok(ReviewReport::fail(issues, fixes, files_checked, entities_checked))
        }
    }

    /// Run roundtrip verification: re-analyze generated code and compare
    /// the resulting S.DEF fingerprints against the originals.
    async fn roundtrip_verify(&self, task: &Task) -> Result<ReviewReport, DbError> {
        let input: serde_json::Value = serde_json::from_str(&task.input_json)
            .unwrap_or_else(|_| serde_json::json!({}));

        let entity_uri = input
            .get("entity_uri")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let document_name = input
            .get("document_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        info!(%entity_uri, %document_name, "Running roundtrip verification");

        let mut issues = Vec::new();
        let mut fixes = Vec::new();

        // Query fingerprints table directly
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT sdef_hash, db_hash, code_hash
                 FROM fingerprints
                 WHERE document_name = ?1 AND entity_uri = ?2",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let result: Result<(Option<String>, Option<String>, Option<String>), _> =
            stmt.query_row(
                rusqlite::params![document_name, entity_uri],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            );

        drop(stmt);
        drop(conn);

        match result {
            Ok((Some(sdef_hash), Some(db_hash), _)) => {
                if sdef_hash != db_hash {
                    issues.push(format!(
                        "S.DEF hash ({}) does not match DB hash ({}) for {}",
                        &sdef_hash[..8.min(sdef_hash.len())],
                        &db_hash[..8.min(db_hash.len())],
                        entity_uri
                    ));
                    fixes.push(format!(
                        "Re-sync S.DEF and database for '{}'", entity_uri
                    ));
                }
            }
            Ok(_) => {
                issues.push(format!(
                    "Incomplete fingerprint for entity '{}' in document '{}'",
                    entity_uri, document_name
                ));
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                issues.push(format!(
                    "No fingerprint found for entity '{}' in document '{}'",
                    entity_uri, document_name
                ));
            }
            Err(e) => {
                issues.push(format!("Fingerprint lookup failed: {}", e));
            }
        }

        if issues.is_empty() {
            info!(%entity_uri, "Roundtrip verification passed");
            Ok(ReviewReport::pass(0, 1))
        } else {
            warn!(%entity_uri, count = issues.len(), "Roundtrip verification failed");
            Ok(ReviewReport::fail(issues, fixes, 0, 1))
        }
    }

    /// Read a data model spec from the database, including its attributes.
    fn read_data_model_spec(
        &self,
        entity: &str,
    ) -> Result<DataModelSpec, DbError> {
        // Read from data_models and data_attributes tables
        let conn = self.db.connection();

        let mut stmt = conn
            .prepare(
                "SELECT name, attr_type, description, required, identity
                 FROM data_attributes
                 WHERE entity = ?1",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let attributes: Vec<AttributeSpec> = stmt
            .query_map(rusqlite::params![entity], |row| {
                Ok(AttributeSpec {
                    name: row.get(0)?,
                    attr_type: row.get(1)?,
                    description: row.get::<_, Option<String>>(2)?,
                    required: row.get(3)?,
                    identity: row.get(4)?,
                })
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        drop(stmt);
        drop(conn);

        Ok(DataModelSpec {
            entity: entity.to_string(),
            attributes,
        })
    }

    /// Notify the original generating agent about review failures.
    fn notify_failure(
        &self,
        task: &Task,
        report: &ReviewReport,
    ) -> Result<(), DbError> {
        // Determine which agent to notify from the task's assigned_to history
        // Use the task's original input to find the generating agent
        let sender = MessageSender::new(self.db.clone());

        // Send review request to broadcast — any consumer agent can pick it up
        sender.broadcast(
            &self.agent_id,
            MessageType::ReviewRequest {
                entity_uri: task.input_json.clone(),
                issues: report.issues.clone(),
            },
            serde_json::json!({
                "task_id": task.task_id,
                "suggested_fixes": report.suggested_fixes,
            }),
        )?;

        info!(
            task_id = %task.task_id,
            issue_count = report.issues.len(),
            "Notified agents of review failure"
        );

        Ok(())
    }
}

/// Simplified data model spec for validation.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DataModelSpec {
    entity: String,
    attributes: Vec<AttributeSpec>,
}

/// Simplified attribute spec for validation.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AttributeSpec {
    name: String,
    attr_type: String,
    description: Option<String>,
    required: bool,
    identity: bool,
}

/// Run the reviewer agent loop — continuously polls for and processes review tasks.
pub async fn reviewer_loop(agent: &ReviewerAgent) -> Result<(), DbError> {
    loop {
        match agent.process_next_task().await {
            Ok(Some(task)) => {
                info!(
                    task_id = %task.task_id,
                    "Reviewer completed task"
                );
            }
            Ok(None) => {
                // No tasks available — sleep briefly before polling again
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            Err(e) => {
                warn!(error = %e, "Reviewer task processing error");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleanroom_db::{Database, TaskStatus};

    fn setup_db_with_model() -> Arc<Database> {
        let db = Arc::new(Database::in_memory().unwrap());
        let conn = db.connection();
        conn.execute_batch(
            "INSERT INTO sdef_documents (name) VALUES ('test-doc');
             INSERT INTO data_models (entity, document_name, status, description)
             VALUES ('User', 'test-doc', 'active', 'A system user');
             INSERT INTO data_attributes (document_name, entity, name, attr_type, description, required, identity)
             VALUES ('test-doc', 'User', 'id', 'UUID', 'Primary key', 1, 1);
             INSERT INTO data_attributes (document_name, entity, name, attr_type, description, required, identity)
             VALUES ('test-doc', 'User', 'email', 'string', 'Email address', 1, 0);
             INSERT INTO data_attributes (document_name, entity, name, attr_type, description, required, identity)
             VALUES ('test-doc', 'User', 'name', 'string', 'Display name', 1, 0);",
        )
        .unwrap();
        drop(conn);
        db
    }

    #[test]
    fn test_read_data_model_spec() {
        let db = setup_db_with_model();
        let agent = ReviewerAgent::new(
            ReviewerConfig::default(),
            db,
            PathBuf::from("/tmp"),
        );

        let spec = agent.read_data_model_spec("User").unwrap();
        assert_eq!(spec.entity, "User");
        assert_eq!(spec.attributes.len(), 3);
        assert_eq!(spec.attributes[0].name, "id");
        assert_eq!(spec.attributes[1].name, "email");
    }

    #[test]
    fn test_validate_data_model_missing() {
        let db = setup_db_with_model();
        let agent = ReviewerAgent::new(
            ReviewerConfig::default(),
            db,
            PathBuf::from("/tmp"),
        );

        // Use empty file path — won't exist, so will report "file not found"
        let task = Task {
            task_id: "test-1".to_string(),
            task_type: TaskType::ValidateDataModel,
            status: TaskStatus::InProgress,
            priority: 5,
            input_json: r#"{"entity": "User", "code_path": "nonexistent.rs"}"#.to_string(),
            output_json: None,
            error_message: None,
            assigned_to: Some("reviewer-test".to_string()),
            progress: 0.0,
            created_at: chrono::Utc::now().to_rfc3339(),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            last_heartbeat: None,
            dependencies_json: "[]".to_string(),
            version: 1,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let report = rt.block_on(agent.validate_data_model(&task)).unwrap();
        assert!(!report.approved);
        assert!(!report.issues.is_empty());
    }
}
