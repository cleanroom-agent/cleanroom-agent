//! CleanroomAgent — top-level agent entry point using adk-rust.
//!
//! Wraps an adk-rust LLM agent alongside database connectivity,
//! providing a unified entry point for produce/consume/resume modes.

use std::path::PathBuf;
use std::sync::Arc;

use cleanroom_db::Database;
use tracing::{info, instrument};

use crate::consumer::{ConsumerAgent, ConsumerConfig, CompatibilityMode, Fidelity};
use crate::orchestrator::{Orchestrator, OrchestratorConfig};
use crate::producer::{ProducerAgent, ProducerConfig};

/// Run mode for the agent.
#[derive(Debug, Clone)]
pub enum RunMode {
    /// Analyze a code repository and output S.DEF.
    Produce {
        repo_path: PathBuf,
        output_path: PathBuf,
        project_name: String,
    },
    /// Read S.DEF and generate code.
    Consume {
        sdef_path: PathBuf,
        output_path: PathBuf,
        language: String,
        framework: Option<String>,
        compat_mode: CompatibilityMode,
        fidelity: Fidelity,
    },
    /// Resume a paused workflow from checkpoint.
    Resume {
        document: String,
        retry_failed: bool,
    },
}

/// Configuration for the top-level CleanroomAgent.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Database path.
    pub db_path: PathBuf,
    /// LLM model name (e.g. "gemini-2.5-flash").
    pub model_name: Option<String>,
    /// System prompt for the LLM agent.
    pub system_prompt: Option<String>,
    /// Agent name (used for identification).
    pub agent_name: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("state.db"),
            model_name: Some("gemini-2.5-flash".to_string()),
            system_prompt: Some(
                "You are Cleanroom Agent, an intelligent system for software \
                 definition analysis and code generation using the S.DEF format. \
                 You work with S.DEF (Software Definition Exchange Format) to \
                 analyze code repositories and generate functionally equivalent software."
                    .to_string(),
            ),
            agent_name: "cleanroom-agent".to_string(),
        }
    }
}

/// The top-level Cleanroom Agent.
///
/// Wraps an adk-rust `LlmAgent` for LLM interaction capabilities,
/// alongside database connectivity and component configuration.
pub struct CleanroomAgent {
    /// Database connection.
    pub db: Arc<Database>,
    /// adk-rust LLM agent (for LLM-driven tasks).
    pub llm_agent: Option<Arc<dyn adk_rust::Agent>>,
    /// Configuration.
    config: AgentConfig,
}

impl CleanroomAgent {
    /// Create a new CleanroomAgent.
    /// Uses `provider_from_env()` to auto-detect the LLM provider
    /// (GOOGLE_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY).
    #[instrument(skip_all)]
    pub fn new(config: AgentConfig) -> Result<Self, cleanroom_db::DbError> {
        let db = Database::open(&config.db_path)?;

        // Build adk-rust LLM agent if env provider is available
        let llm_agent = match adk_rust::provider_from_env() {
            Ok(model) => {
                let agent = match adk_rust::agent::LlmAgentBuilder::new(&config.agent_name)
                    .description("Cleanroom Agent - S.DEF intelligent agent system")
                    .instruction(
                        config
                            .system_prompt
                            .as_deref()
                            .unwrap_or("You are Cleanroom Agent."),
                    )
                    .model(model)
                    .build()
                {
                    Ok(a) => Some(Arc::new(a) as Arc<dyn adk_rust::Agent>),
                    Err(e) => {
                        tracing::warn!(error = %e, "LlmAgent build failed; continuing without LLM");
                        None
                    }
                };
                agent
            }
            Err(e) => {
                tracing::info!(error = %e, "No LLM provider available; running without LLM capabilities");
                None
            }
        };

        Ok(Self {
            db: Arc::new(db),
            llm_agent,
            config,
        })
    }

    /// Run the agent in the specified mode.
    #[instrument(skip(self))]
    pub async fn run(&self, mode: RunMode) -> anyhow::Result<()> {
        match mode {
            RunMode::Produce {
                repo_path,
                output_path,
                project_name,
            } => self.run_producer(repo_path, output_path, project_name).await,
            RunMode::Consume {
                sdef_path,
                output_path,
                language,
                framework,
                compat_mode,
                fidelity,
            } => {
                self.run_consumer(sdef_path, output_path, &language, framework.as_deref(), compat_mode, fidelity)
                    .await
            }
            RunMode::Resume {
                document,
                retry_failed,
            } => self.run_resume(&document, retry_failed).await,
        }
    }

    /// Run in produce mode: analyze repo → S.DEF.
    async fn run_producer(
        &self,
        repo_path: PathBuf,
        output_path: PathBuf,
        project_name: String,
    ) -> anyhow::Result<()> {
        let config = OrchestratorConfig {
            repo_path,
            output_path,
            db_path: self.config.db_path.clone(),
            project_name,
            checkpoint_interval_secs: 600,
            agent_idle_timeout_secs: 300,
        };
        let orchestrator = Orchestrator::new(config)?;
        orchestrator.start_workflow().await?;

        let producer = ProducerAgent::new(ProducerConfig::default(), orchestrator.db().clone());
        while let Ok(Some(task)) = producer.process_next_task().await {
            info!(task_id = %task.task_id, "Processed task");
        }
        info!(project = %project_name, "Production complete");
        Ok(())
    }

    /// Run in consume mode: S.DEF → code.
    async fn run_consumer(
        &self,
        sdef_path: PathBuf,
        output_path: PathBuf,
        language: &str,
        framework: Option<&str>,
        compat_mode: CompatibilityMode,
        fidelity: Fidelity,
    ) -> anyhow::Result<()> {
        // Import S.DEF file into database
        let sdef_content = std::fs::read_to_string(&sdef_path)?;
        let sdef: sdef_core::SoftwareDefinition = serde_json::from_str(&sdef_content)?;

        let importer = cleanroom_db::export_import::SdefImporter::new(
            rusqlite::Connection::open(&self.config.db_path)?,
        );
        importer.import(&sdef)?;

        // Generate code
        let config = ConsumerConfig {
            language: language.to_string(),
            framework: framework.map(|s| s.to_string()),
            compatibility_mode: compat_mode,
            fidelity,
            output_path,
        };
        let consumer = ConsumerAgent::new(config, self.db.clone());
        consumer.generate_code().await?;

        info!("Consumption complete");
        Ok(())
    }

    /// Run in resume mode: restore workflow state.
    async fn run_resume(&self, document: &str, retry_failed: bool) -> anyhow::Result<()> {
        use crate::scheduler::Scheduler;

        let scheduler = Scheduler::new(self.db.clone());
        let repo = cleanroom_db::TaskRepository::new(self.db.connection_arc());

        let all_tasks = repo.list(None, None, None)?;
        let doc_tasks: Vec<_> = all_tasks
            .iter()
            .filter(|t| t.input_json.contains(document))
            .collect();

        if doc_tasks.is_empty() {
            info!(document = %document, "No tasks found for document");
            return Ok(());
        }

        // Reset in-progress tasks
        for task in doc_tasks.iter().filter(|t| {
            matches!(t.status, cleanroom_db::TaskStatus::InProgress | cleanroom_db::TaskStatus::Assigned)
        }) {
            repo.update_status(&task.task_id, cleanroom_db::TaskStatus::Pending)?;
        }

        // Retry failed tasks if requested
        if retry_failed {
            scheduler.retry_failed_tasks()?;
        }

        let pending_count = doc_tasks
            .iter()
            .filter(|t| t.status == cleanroom_db::TaskStatus::Pending)
            .count();
        info!(document = %document, pending = %pending_count, "Workflow resumable");

        Ok(())
    }

    /// Get a reference to the database.
    pub fn db(&self) -> &Arc<Database> {
        &self.db
    }

    /// Get the configuration.
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
}

impl std::fmt::Debug for CleanroomAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CleanroomAgent")
            .field("db_path", &self.config.db_path)
            .field("has_llm", &self.llm_agent.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.agent_name, "cleanroom-agent");
        assert!(config.model_name.is_some());
        assert!(config.system_prompt.is_some());
    }

    #[test]
    fn test_run_mode_debug() {
        let mode = RunMode::Produce {
            repo_path: PathBuf::from("/tmp/repo"),
            output_path: PathBuf::from("/tmp/output"),
            project_name: "test".to_string(),
        };
        let debug = format!("{:?}", mode);
        assert!(debug.contains("Produce"));
        assert!(debug.contains("/tmp/repo"));
    }

    #[test]
    fn test_agent_config_constructor() {
        let config = AgentConfig {
            db_path: PathBuf::from(":memory:"),
            model_name: Some("gpt-4".to_string()),
            system_prompt: Some("Test prompt".to_string()),
            agent_name: "custom-agent".to_string(),
        };
        assert_eq!(config.agent_name, "custom-agent");
        assert_eq!(config.model_name.unwrap(), "gpt-4");
        assert_eq!(config.system_prompt.unwrap(), "Test prompt");
    }
}
