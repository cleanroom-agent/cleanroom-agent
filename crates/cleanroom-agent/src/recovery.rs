//! Workflow recovery on startup.
//!
//! Implements partial workflow recovery from docs/16-resilience.md §8.
//! Recovers dangling transactions, stuck tasks, orphaned shards, and
//! verifies consistency after an unclean shutdown.

use cleanroom_db::Database;
use tracing::{info, warn};

/// Report of what was recovered on startup.
#[derive(Debug, Default, Clone)]
pub struct RecoveryReport {
    /// Number of dangling two-phase commit transactions cleaned up.
    pub dangling_transactions: usize,
    /// Number of stuck tasks reset to pending.
    pub stuck_tasks: usize,
    /// Number of tasks permanently failed (retries exhausted).
    pub permanently_failed: usize,
    /// Number of consistency issues detected.
    pub inconsistencies_found: usize,
}

impl RecoveryReport {
    /// Returns true if any recovery action was taken.
    pub fn had_issues(&self) -> bool {
        self.dangling_transactions > 0
            || self.stuck_tasks > 0
            || self.inconsistencies_found > 0
    }
}

/// On startup, check for orphaned work and recover.
///
/// Call this at the beginning of any workflow to ensure a clean state
/// after a previous crash or unclean shutdown.
///
/// # Steps
///
/// 1. Clean up dangling two-phase commit transactions
/// 2. Reset stuck tasks (in_progress beyond heartbeat timeout) to pending
/// 3. Mark exhausted tasks as failed_permanently
/// 4. Report consistency issues
pub fn recover_on_startup(db: &Database) -> Result<RecoveryReport, cleanroom_db::DbError> {
    let conn = db.connection();
    let mut report = RecoveryReport::default();

    info!("Running startup recovery checks...");

    // 1. Recover prepared transactions (two-phase commit)
    report.dangling_transactions = cleanup_dangling_transactions(&conn)?;

    // 2. Recover stuck tasks — tasks that were in_progress when
    //    the previous run crashed. Default heartbeat timeout = 5 minutes.
    let stuck_count = reset_stuck_tasks(&conn)?;
    report.stuck_tasks = stuck_count;

    // 3. Mark tasks with exhausted retries as permanently failed
    report.permanently_failed = finalize_exhausted_tasks(&conn)?;

    // 4. Count consistency issues (fingerprint mismatches)
    report.inconsistencies_found = count_inconsistencies(&conn)?;

    if report.had_issues() {
        warn!(
            dangling = report.dangling_transactions,
            stuck = report.stuck_tasks,
            failed = report.permanently_failed,
            inconsistencies = report.inconsistencies_found,
            "Recovery complete — issues were found and repaired"
        );
    } else {
        info!("Recovery check complete — no issues found");
    }

    Ok(report)
}

/// Clean up prepared transactions that were left in an incomplete state.
///
/// Transactions left in 'prepared' state after a crash are rolled back
/// to maintain database consistency.
fn cleanup_dangling_transactions(conn: &rusqlite::Connection) -> Result<usize, cleanroom_db::DbError> {
    // Count dangling transactions
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM prepared_transactions WHERE status = 'prepared'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

    if count > 0 {
        // Roll back any dangling prepared transactions
        conn.execute(
            "UPDATE prepared_transactions SET status = 'rolled_back', rollback_at = CURRENT_TIMESTAMP
             WHERE status = 'prepared'",
            [],
        )
        .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

        warn!(count, "Rolled back dangling prepared transactions");
    }

    Ok(count as usize)
}

/// Reset stuck in_progress tasks back to pending.
///
/// Tasks stuck in 'in_progress' or 'assigned' state with stale heartbeats
/// (older than 5 minutes) are assumed to belong to a crashed agent and
/// are reset so another agent can claim them.
fn reset_stuck_tasks(conn: &rusqlite::Connection) -> Result<usize, cleanroom_db::DbError> {
    let rows = conn.execute(
        "UPDATE tasks SET status = 'pending', assigned_to = NULL
         WHERE status IN ('assigned', 'in_progress')
           AND (last_heartbeat IS NULL OR last_heartbeat < datetime('now', '-5 minutes'))",
        [],
    )
    .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

    if rows > 0 {
        info!(count = rows, "Reset stuck tasks to pending");
    }

    Ok(rows)
}

/// Finalize tasks with exhausted retries as permanently failed.
///
/// Tasks that have been retried more times than allowed (retry_count >= max_retries)
/// and are currently in a transient-failure state should be finalized.
fn finalize_exhausted_tasks(conn: &rusqlite::Connection) -> Result<usize, cleanroom_db::DbError> {
    let rows = conn.execute(
        "UPDATE tasks SET status = 'failed_permanently'
         WHERE status IN ('failed', 'retrying')
           AND retry_count >= max_retries
           AND max_retries > 0",
        [],
    )
    .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

    if rows > 0 {
        info!(count = rows, "Finalized exhausted tasks as permanently failed");
    }

    Ok(rows)
}

/// Count fingerprint inconsistencies for reporting.
fn count_inconsistencies(conn: &rusqlite::Connection) -> Result<usize, cleanroom_db::DbError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fingerprints WHERE sdef_hash != db_hash OR db_hash != code_hash",
            [],
            |row| row.get(0),
        )
        .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

    Ok(count as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    /// Create an in-memory database with the minimal schema for recovery tests.
    /// Uses rusqlite directly to avoid migration errors with :memory: databases.
    fn setup_test_db() -> Arc<Mutex<rusqlite::Connection>> {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS prepared_transactions (
                transaction_id TEXT PRIMARY KEY,
                phase TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                prepared_at TIMESTAMP,
                committed_at TIMESTAMP,
                rollback_at TIMESTAMP,
                changes_json TEXT NOT NULL DEFAULT '{}',
                status TEXT NOT NULL DEFAULT 'pending'
            );
            CREATE TABLE IF NOT EXISTS tasks (
                task_id TEXT PRIMARY KEY,
                task_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                priority INTEGER DEFAULT 5,
                assigned_to TEXT,
                progress REAL DEFAULT 0,
                error_message TEXT,
                output_json TEXT,
                input_json TEXT DEFAULT '{}',
                dependencies_json TEXT DEFAULT '[]',
                retry_count INTEGER DEFAULT 0,
                max_retries INTEGER DEFAULT 3,
                last_heartbeat TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                started_at TEXT,
                completed_at TEXT,
                version INTEGER DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS fingerprints (
                fingerprint_id TEXT PRIMARY KEY,
                entity_uri TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                sdef_hash TEXT NOT NULL,
                db_hash TEXT NOT NULL,
                code_hash TEXT NOT NULL
            );"
        ).unwrap();

        Arc::new(Mutex::new(conn))
    }

    fn conn(db: &Arc<Mutex<rusqlite::Connection>>) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
        db.lock().unwrap()
    }

    #[test]
    fn test_cleanup_dangling_transactions() {
        let db = setup_test_db();
        let c = conn(&db);

        c.execute(
            "INSERT INTO prepared_transactions (transaction_id, phase, status)
             VALUES ('tx1', 'commit', 'prepared')",
            [],
        ).unwrap();

        let count = cleanup_dangling_transactions(&c).unwrap();
        assert_eq!(count, 1);

        let status: String = c.query_row(
            "SELECT status FROM prepared_transactions WHERE transaction_id = 'tx1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(status, "rolled_back");
    }

    #[test]
    fn test_reset_stuck_tasks() {
        let db = setup_test_db();
        let c = conn(&db);

        c.execute(
            "INSERT INTO tasks (task_id, task_type, status, assigned_to, last_heartbeat)
             VALUES ('t1', 'ANALYZE', 'in_progress', 'agent-1', datetime('now', '-10 minutes'))",
            [],
        ).unwrap();
        c.execute(
            "INSERT INTO tasks (task_id, task_type, status, assigned_to, last_heartbeat)
             VALUES ('t2', 'ANALYZE', 'in_progress', 'agent-2', datetime('now', '-1 minute'))",
            [],
        ).unwrap();

        let count = reset_stuck_tasks(&c).unwrap();
        // Only t1 should be reset (stale heartbeat > 5 min)
        assert_eq!(count, 1);

        let status: String = c.query_row(
            "SELECT status FROM tasks WHERE task_id = 't1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(status, "pending");
    }

    #[test]
    fn test_finalize_exhausted_tasks() {
        let db = setup_test_db();
        let c = conn(&db);

        c.execute(
            "INSERT INTO tasks (task_id, task_type, status, retry_count, max_retries)
             VALUES ('t1', 'ANALYZE', 'failed', 5, 3)",
            [],
        ).unwrap();
        c.execute(
            "INSERT INTO tasks (task_id, task_type, status, retry_count, max_retries)
             VALUES ('t2', 'ANALYZE', 'failed', 1, 3)",
            [],
        ).unwrap();

        let count = finalize_exhausted_tasks(&c).unwrap();
        // Only t1 should be finalized (retry_count >= max_retries)
        assert_eq!(count, 1);
    }

    #[test]
    fn test_recovery_report_had_issues() {
        let mut report = RecoveryReport::default();
        assert!(!report.had_issues());

        report.stuck_tasks = 5;
        assert!(report.had_issues());
    }
}
