//! Orchestrator — coordinates task execution for the Cleanroom agent.
//!
//! The orchestrator manages the workflow execution of repository analysis tasks.
//! It creates initial tasks, manages checkpoints, and handles agent idle timeouts.
//!
//! # Workflow
//!
//! 1. Create initial tasks via [`Orchestrator::create_initial_tasks`]
//! 2. Start workflow via [`Orchestrator::start_workflow`]
//! 3. Tasks are processed by [`ProducerAgent`]
//!
//! # Checkpointing
//!
//! The orchestrator periodically checkpoints progress, allowing workflow
//! resumption if interrupted. Checkpoint interval is configured via
//! [`OrchestratorConfig::checkpoint_interval_secs`].

use std::path::PathBuf;
use std::sync::Arc;

use cleanroom_db::{Database, TaskRepository, TaskType};
use tracing::{info, instrument};

/// Orchestrator configuration.
///
/// Contains all settings needed to configure the orchestrator's behavior,
/// including paths, checkpoint intervals, and timeout settings.
///
/// # Example
///
/// ```no_run
/// use cleanroom_agent::OrchestratorConfig;
/// use std::path::PathBuf;
///
/// let config = OrchestratorConfig {
///     repo_path: PathBuf::from("./my-repo"),
///     output_path: PathBuf::from("./output"),
///     db_path: PathBuf::from("state.db"),
///     project_name: "my-project".to_string(),
///     checkpoint_interval_secs: 600,
///     agent_idle_timeout_secs: 300,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Path to the source code repository to analyze
    pub repo_path: PathBuf,
    /// Directory for S.DEF output files
    pub output_path: PathBuf,
    /// Path to the SQLite database
    pub db_path: PathBuf,
    /// Name of the project/document being analyzed
    pub project_name: String,
    /// Interval between checkpoints in seconds (default: 600 = 10 minutes)
    pub checkpoint_interval_secs: u64,
    /// Idle timeout for agent tasks in seconds (default: 300 = 5 minutes)
    pub agent_idle_timeout_secs: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::from("."),
            output_path: PathBuf::from("./output"),
            db_path: PathBuf::from("state.db"),
            project_name: "unnamed".to_string(),
            checkpoint_interval_secs: 600, // 10 minutes
            agent_idle_timeout_secs: 300, // 5 minutes
        }
    }
}

/// Orchestrator — coordinates task execution for the Cleanroom agent.
///
/// The orchestrator manages the lifecycle of repository analysis tasks.
/// It creates initial tasks, handles checkpointing, and monitors agent activity.
///
/// # Tasks
///
/// The orchestrator creates the following initial task types:
/// - [`TaskType::RepoAnalyze`]: Full repository analysis task
///
/// # Example
///
/// ```no_run
/// use cleanroom_agent::{Orchestrator, OrchestratorConfig};
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = OrchestratorConfig {
///     repo_path: PathBuf::from("./my-repo"),
///     output_path: PathBuf::from("./output"),
///     db_path: PathBuf::from("state.db"),
///     project_name: "my-project".to_string(),
///     ..Default::default()
/// };
/// let orchestrator = Orchestrator::new(config)?;
/// orchestrator.start_workflow().await?;
/// # Ok(())
/// # }
/// ```
pub struct Orchestrator {
    /// Orchestrator configuration
    config: OrchestratorConfig,
    /// Database connection for task persistence
    db: Arc<Database>,
}

impl Orchestrator {
    /// Create a new orchestrator.
    pub fn new(config: OrchestratorConfig) -> Result<Self, cleanroom_db::DbError> {
        let db = Database::open(&config.db_path)?;
        Ok(Self {
            config,
            db: Arc::new(db),
        })
    }

    /// Get database.
    pub fn db(&self) -> &Arc<Database> {
        &self.db
    }

    /// Get configuration.
    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }

    /// Create initial tasks for repository analysis.
    #[instrument(skip(self))]
    pub async fn create_initial_tasks(&self) -> Result<(), cleanroom_db::DbError> {
        let repo = TaskRepository::new(self.db.connection_arc());

        // Create the main repo analyze task
        let task = cleanroom_db::Task {
            task_id: uuid::Uuid::new_v4().to_string(),
            task_type: TaskType::RepoAnalyze,
            status: cleanroom_db::TaskStatus::Pending,
            priority: 10,
            input_json: serde_json::json!({
                "repo_path": self.config.repo_path.to_string_lossy(),
                "project_name": self.config.project_name,
            }).to_string(),
            output_json: None,
            error_message: None,
            assigned_to: None,
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

        repo.create(&task)?;
        info!(task_id = %task.task_id, "Created initial task");
        Ok(())
    }

    /// Start the workflow.
    #[instrument(skip(self))]
    pub async fn start_workflow(&self) -> Result<(), cleanroom_db::DbError> {
        self.create_initial_tasks().await?;
        info!("Workflow started");
        Ok(())
    }
}