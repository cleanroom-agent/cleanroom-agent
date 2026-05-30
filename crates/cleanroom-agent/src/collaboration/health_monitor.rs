//! Health Monitor — detects zombie agents and recovers their tasks.
//!
//! Part of the multi-agent collaboration system (docs/13 §5.2).
//! Runs in the background and periodically checks for agents that have
//! stopped sending heartbeats, then reassigns their tasks to healthy agents.

use std::sync::Arc;
use std::time::Duration;

use cleanroom_db::Database;
use tracing::{info, warn, instrument};

/// Background health checker for agent liveness.
///
/// Detects zombie agents (no heartbeat within timeout) and reassigns
/// their in-progress tasks back to the pending queue.
pub struct HealthMonitor {
    /// Maximum allowed time since last heartbeat before agent is considered dead.
    heartbeat_timeout: Duration,
    /// Interval between health check cycles.
    check_interval: Duration,
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self {
            heartbeat_timeout: Duration::from_secs(120), // 2 minutes
            check_interval: Duration::from_secs(30),     // 30 seconds
        }
    }
}

impl HealthMonitor {
    /// Create a new health monitor.
    pub fn new(heartbeat_timeout: Duration, check_interval: Duration) -> Self {
        Self {
            heartbeat_timeout,
            check_interval,
        }
    }

    /// Get the heartbeat timeout duration.
    pub fn heartbeat_timeout(&self) -> Duration {
        self.heartbeat_timeout
    }

    /// Get the check interval duration.
    pub fn check_interval(&self) -> Duration {
        self.check_interval
    }

    /// Run a single health check cycle.
    ///
    /// Finds tasks that have been `in_progress` without a heartbeat
    /// for longer than `heartbeat_timeout` and reassigns them to `pending`.
    /// Returns the number of recovered (reassigned) tasks.
    #[instrument(skip_all)]
    pub async fn check(&self, db: &Database) -> Result<usize, cleanroom_db::DbError> {
        let conn = db.connection();
        let timeout_secs = self.heartbeat_timeout.as_secs() as i64;

        // Find zombie tasks: in_progress with stale heartbeat
        let mut find_stmt = conn
            .prepare(
                "SELECT task_id, assigned_to
                 FROM tasks
                 WHERE status = 'in_progress'
                   AND last_heartbeat IS NOT NULL
                   AND last_heartbeat < datetime('now', ?1)",
            )
            .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

        let zombies: Vec<(String, Option<String>)> = find_stmt
            .query_map(rusqlite::params![format!("-{} seconds", timeout_secs)], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        drop(find_stmt);

        if zombies.is_empty() {
            return Ok(0);
        }

        let recovered = zombies.len();
        for (task_id, assigned_to) in &zombies {
            warn!(
                task_id = %task_id,
                agent = %assigned_to.as_deref().unwrap_or("unknown"),
                "Detected zombie task — reassigning to pending"
            );

            conn.execute(
                "UPDATE tasks SET status = 'pending', assigned_to = NULL WHERE task_id = ?1",
                rusqlite::params![task_id],
            )
            .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;
        }

        drop(conn);

        info!(
            recovered = recovered,
            timeout_secs = timeout_secs,
            "Health check complete — reassigned zombie tasks"
        );

        Ok(recovered)
    }

    /// Run health checks continuously in the background.
    ///
    /// Spawns a tokio task that runs `check()` at the configured interval.
    /// Returns a shutdown sender that can be used to stop the monitor.
    pub fn start(
        monitor: Self,
        db: Arc<Database>,
    ) -> tokio::sync::oneshot::Sender<()> {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            info!(
                interval_secs = monitor.check_interval.as_secs(),
                timeout_secs = monitor.heartbeat_timeout.as_secs(),
                "Health monitor started"
            );

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        info!("Health monitor shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(monitor.check_interval) => {
                        match monitor.check(&db).await {
                            Ok(n) if n > 0 => {
                                info!(recovered = n, "Health monitor recovered tasks");
                            }
                            Ok(_) => {
                                // No issues found — silent success
                            }
                            Err(e) => {
                                warn!(error = %e, "Health monitor check failed");
                            }
                        }
                    }
                }
            }
        });

        shutdown_tx
    }

    /// Mark agents that have been offline too long.
    ///
    /// Agents with no heartbeat for `heartbeat_timeout` are marked as 'offline'.
    /// Returns the number of agents marked offline.
    #[instrument(skip_all)]
    pub fn mark_offline_agents(
        &self,
        db: &Database,
    ) -> Result<usize, cleanroom_db::DbError> {
        let conn = db.connection();
        let timeout_secs = self.heartbeat_timeout.as_secs() as i64;

        let rows = conn
            .execute(
                "UPDATE agents SET status = 'offline', current_task_id = NULL
                 WHERE status = 'online'
                   AND last_seen < datetime('now', ?1)",
                rusqlite::params![format!("-{} seconds", timeout_secs)],
            )
            .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

        if rows > 0 {
            warn!(count = rows, "Marked offline agents");
        }

        drop(conn);
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleanroom_db::{Database, Task, TaskRepository, TaskStatus, TaskType};

    fn create_test_task(task_id: &str, agent_id: &str) -> Task {
        // Use a fixed timestamp far in the past so SQLite datetime() comparison works
        let stale_time = "2000-01-01T00:00:00Z";
        Task {
            task_id: task_id.to_string(),
            task_type: TaskType::RepoAnalyze,
            status: TaskStatus::InProgress,
            priority: 5,
            input_json: r#"{"test": true}"#.to_string(),
            output_json: None,
            error_message: None,
            assigned_to: Some(agent_id.to_string()),
            progress: 0.5,
            created_at: chrono::Utc::now().to_rfc3339(),
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            last_heartbeat: Some(stale_time.to_string()),
            dependencies_json: "[]".to_string(),
            version: 1,
        }
    }

    #[tokio::test]
    async fn test_detect_zombie_task() {
        let db = Arc::new(Database::in_memory().unwrap());
        let repo = TaskRepository::new(db.connection_arc());

        // Create a task with stale heartbeat (200 seconds ago)
        let task = create_test_task("zombie-1", "dead-agent-1");
        repo.create(&task).unwrap();

        let monitor = HealthMonitor {
            heartbeat_timeout: Duration::from_secs(100), // timeout = 100s, task is 200s old
            check_interval: Duration::from_secs(30),
        };

        let recovered = monitor.check(&db).await.unwrap();
        assert_eq!(recovered, 1, "Should detect one zombie task");

        // Verify task is now pending
        let recovered_task = repo.get("zombie-1").unwrap();
        assert_eq!(recovered_task.status, TaskStatus::Pending);
        assert!(recovered_task.assigned_to.is_none());
    }

    #[tokio::test]
    async fn test_no_zombies_with_recent_heartbeat() {
        let db = Arc::new(Database::in_memory().unwrap());
        let repo = TaskRepository::new(db.connection_arc());

        // Create a task with recent heartbeat
        let mut task = create_test_task("alive-1", "alive-agent-1");
        task.last_heartbeat = Some(chrono::Utc::now().to_rfc3339());
        repo.create(&task).unwrap();

        let monitor = HealthMonitor {
            heartbeat_timeout: Duration::from_secs(100),
            check_interval: Duration::from_secs(30),
        };

        let recovered = monitor.check(&db).await.unwrap();
        assert_eq!(recovered, 0, "Should not detect any zombies");
    }
}
