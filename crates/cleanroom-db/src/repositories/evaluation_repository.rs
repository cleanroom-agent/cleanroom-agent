//! Evaluation history repository — stores and retrieves evaluation run results.
//!
//! Provides CRUD operations on the `evaluation_history` table for tracking
//! analysis/generation quality over time and detecting regressions.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::instrument;

use crate::error::{DbError, DbResult};

/// A single evaluation run record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRecord {
    pub run_id: String,
    pub project_name: String,
    pub language: String,
    pub version: Option<String>,
    pub run_at: String,
    pub duration_ms: i64,
    pub file_coverage: f64,
    pub entity_coverage: f64,
    pub type_accuracy: Option<f64>,
    pub f1_score: f64,
    pub compile_pass_rate: f64,
    pub test_pass_rate: Option<f64>,
    pub roundtrip_fidelity: f64,
    pub files_analyzed: i64,
    pub entities_extracted: i64,
    pub tasks_completed: i64,
    pub tasks_failed: i64,
    pub tokens_consumed: i64,
    pub report_json: String,
    pub is_degraded: bool,
    pub degraded_metrics_json: Option<String>,
}

/// Summary of recent evaluation runs for a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationSummary {
    pub project_name: String,
    pub total_runs: usize,
    pub latest_fidelity: f64,
    pub trend: EvaluationTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvaluationTrend {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

/// Repository for evaluation_history operations.
pub struct EvaluationRepository {
    conn: Arc<Mutex<Connection>>,
}

impl EvaluationRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Store an evaluation run record.
    #[instrument(skip_all)]
    pub fn save(&self, record: &EvaluationRecord) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO evaluation_history (
                run_id, project_name, language, version, run_at, duration_ms,
                file_coverage, entity_coverage, type_accuracy, f1_score,
                compile_pass_rate, test_pass_rate, roundtrip_fidelity,
                files_analyzed, entities_extracted, tasks_completed, tasks_failed,
                tokens_consumed, report_json, is_degraded, degraded_metrics_json
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10,
                ?11, ?12, ?13,
                ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21
            )"#,
            params![
                record.run_id,
                record.project_name,
                record.language,
                record.version,
                record.run_at,
                record.duration_ms,
                record.file_coverage,
                record.entity_coverage,
                record.type_accuracy,
                record.f1_score,
                record.compile_pass_rate,
                record.test_pass_rate,
                record.roundtrip_fidelity,
                record.files_analyzed,
                record.entities_extracted,
                record.tasks_completed,
                record.tasks_failed,
                record.tokens_consumed,
                record.report_json,
                record.is_degraded,
                record.degraded_metrics_json,
            ],
        )
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get the most recent evaluation for a project.
    #[instrument(skip_all)]
    pub fn latest(&self, project_name: &str) -> DbResult<Option<EvaluationRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT run_id, project_name, language, version, run_at, duration_ms,
                        file_coverage, entity_coverage, type_accuracy, f1_score,
                        compile_pass_rate, test_pass_rate, roundtrip_fidelity,
                        files_analyzed, entities_extracted, tasks_completed, tasks_failed,
                        tokens_consumed, report_json, is_degraded, degraded_metrics_json
                 FROM evaluation_history
                 WHERE project_name = ?1
                 ORDER BY run_at DESC LIMIT 1",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![project_name], |row| {
            Ok(EvaluationRecord {
                run_id: row.get(0)?,
                project_name: row.get(1)?,
                language: row.get(2)?,
                version: row.get(3)?,
                run_at: row.get(4)?,
                duration_ms: row.get(5)?,
                file_coverage: row.get(6)?,
                entity_coverage: row.get(7)?,
                type_accuracy: row.get(8)?,
                f1_score: row.get(9)?,
                compile_pass_rate: row.get(10)?,
                test_pass_rate: row.get(11)?,
                roundtrip_fidelity: row.get(12)?,
                files_analyzed: row.get(13)?,
                entities_extracted: row.get(14)?,
                tasks_completed: row.get(15)?,
                tasks_failed: row.get(16)?,
                tokens_consumed: row.get(17)?,
                report_json: row.get(18)?,
                is_degraded: row.get(19)?,
                degraded_metrics_json: row.get(20)?,
            })
        });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::QueryFailed(e.to_string())),
        }
    }

    /// List evaluation runs for a project, ordered by recency.
    #[instrument(skip_all)]
    pub fn list_by_project(
        &self,
        project_name: &str,
        limit: usize,
    ) -> DbResult<Vec<EvaluationRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT run_id, project_name, language, version, run_at, duration_ms,
                        file_coverage, entity_coverage, type_accuracy, f1_score,
                        compile_pass_rate, test_pass_rate, roundtrip_fidelity,
                        files_analyzed, entities_extracted, tasks_completed, tasks_failed,
                        tokens_consumed, report_json, is_degraded, degraded_metrics_json
                 FROM evaluation_history
                 WHERE project_name = ?1
                 ORDER BY run_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let records = stmt
            .query_map(params![project_name, limit as i64], |row| {
                Ok(EvaluationRecord {
                    run_id: row.get(0)?,
                    project_name: row.get(1)?,
                    language: row.get(2)?,
                    version: row.get(3)?,
                    run_at: row.get(4)?,
                    duration_ms: row.get(5)?,
                    file_coverage: row.get(6)?,
                    entity_coverage: row.get(7)?,
                    type_accuracy: row.get(8)?,
                    f1_score: row.get(9)?,
                    compile_pass_rate: row.get(10)?,
                    test_pass_rate: row.get(11)?,
                    roundtrip_fidelity: row.get(12)?,
                    files_analyzed: row.get(13)?,
                    entities_extracted: row.get(14)?,
                    tasks_completed: row.get(15)?,
                    tasks_failed: row.get(16)?,
                    tokens_consumed: row.get(17)?,
                    report_json: row.get(18)?,
                    is_degraded: row.get(19)?,
                    degraded_metrics_json: row.get(20)?,
                })
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(records)
    }

    /// Get a summary with trend analysis for a project.
    #[instrument(skip_all)]
    pub fn get_summary(&self, project_name: &str) -> DbResult<EvaluationSummary> {
        let records = self.list_by_project(project_name, 10)?;

        let total_runs = records.len();
        if total_runs == 0 {
            return Ok(EvaluationSummary {
                project_name: project_name.to_string(),
                total_runs: 0,
                latest_fidelity: 0.0,
                trend: EvaluationTrend::Unknown,
            });
        }

        let latest_fidelity = records[0].roundtrip_fidelity;

        let trend = if total_runs < 2 {
            EvaluationTrend::Unknown
        } else {
            let oldest = &records[records.len() - 1].roundtrip_fidelity;
            let diff = latest_fidelity - oldest;
            if diff > 0.02 {
                EvaluationTrend::Improving
            } else if diff < -0.02 {
                EvaluationTrend::Degrading
            } else {
                EvaluationTrend::Stable
            }
        };

        Ok(EvaluationSummary {
            project_name: project_name.to_string(),
            total_runs,
            latest_fidelity,
            trend,
        })
    }

    /// Delete evaluation records older than N days.
    #[instrument(skip_all)]
    pub fn prune_older_than(&self, days: i64) -> DbResult<usize> {
        let conn = self.conn.lock().unwrap();
        let rows = conn
            .execute(
                "DELETE FROM evaluation_history WHERE run_at < datetime('now', ?1)",
                params![format!("-{} days", days)],
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    fn setup() -> (Database, EvaluationRepository) {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS evaluation_history (
                run_id TEXT PRIMARY KEY,
                project_name TEXT NOT NULL,
                language TEXT NOT NULL,
                version TEXT,
                run_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                duration_ms INTEGER NOT NULL DEFAULT 0,
                file_coverage REAL NOT NULL DEFAULT 0,
                entity_coverage REAL NOT NULL DEFAULT 0,
                type_accuracy REAL,
                f1_score REAL NOT NULL DEFAULT 0,
                compile_pass_rate REAL NOT NULL DEFAULT 0,
                test_pass_rate REAL,
                roundtrip_fidelity REAL NOT NULL DEFAULT 0,
                files_analyzed INTEGER NOT NULL DEFAULT 0,
                entities_extracted INTEGER NOT NULL DEFAULT 0,
                tasks_completed INTEGER NOT NULL DEFAULT 0,
                tasks_failed INTEGER NOT NULL DEFAULT 0,
                tokens_consumed INTEGER NOT NULL DEFAULT 0,
                report_json TEXT NOT NULL DEFAULT '{}',
                is_degraded BOOLEAN NOT NULL DEFAULT FALSE,
                degraded_metrics_json TEXT
            );",
        )
        .unwrap();
        drop(conn);
        let repo = EvaluationRepository::new(db.connection_arc());
        (db, repo)
    }

    fn make_record(id: &str, project: &str, fidelity: f64) -> EvaluationRecord {
        // Use staggered timestamps so ordering is deterministic in tests
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let offset = COUNTER.fetch_add(1, Ordering::SeqCst);
        let ts = chrono::Utc::now() + chrono::Duration::seconds(offset as i64);

        EvaluationRecord {
            run_id: id.to_string(),
            project_name: project.to_string(),
            language: "rust".to_string(),
            version: Some("0.1.0".to_string()),
            run_at: ts.to_rfc3339(),
            duration_ms: 5000,
            file_coverage: 0.95,
            entity_coverage: 0.88,
            type_accuracy: Some(0.92),
            f1_score: 0.90,
            compile_pass_rate: 1.0,
            test_pass_rate: Some(0.95),
            roundtrip_fidelity: fidelity,
            files_analyzed: 50,
            entities_extracted: 120,
            tasks_completed: 9,
            tasks_failed: 0,
            tokens_consumed: 45000,
            report_json: "{}".to_string(),
            is_degraded: false,
            degraded_metrics_json: None,
        }
    }

    #[test]
    fn test_save_and_latest() {
        let (_, repo) = setup();
        let record = make_record("run-1", "test-proj", 0.95);
        repo.save(&record).unwrap();

        let latest = repo.latest("test-proj").unwrap().unwrap();
        assert_eq!(latest.run_id, "run-1");
        assert_eq!(latest.roundtrip_fidelity, 0.95);
    }

    #[test]
    fn test_latest_empty() {
        let (_, repo) = setup();
        let result = repo.latest("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_by_project() {
        let (_, repo) = setup();
        repo.save(&make_record("run-1", "test-proj", 0.80)).unwrap();
        repo.save(&make_record("run-2", "test-proj", 0.85)).unwrap();
        repo.save(&make_record("run-3", "other-proj", 0.90)).unwrap();

        let records = repo.list_by_project("test-proj", 10).unwrap();
        assert_eq!(records.len(), 2);
        // Most recent first
        assert_eq!(records[0].run_id, "run-2");
    }

    #[test]
    fn test_trend_improving() {
        let (_, repo) = setup();
        repo.save(&make_record("run-1", "trend-proj", 0.70)).unwrap();
        repo.save(&make_record("run-2", "trend-proj", 0.73)).unwrap();
        repo.save(&make_record("run-3", "trend-proj", 0.76)).unwrap();

        let summary = repo.get_summary("trend-proj").unwrap();
        assert_eq!(summary.total_runs, 3);
        assert!(matches!(summary.trend, EvaluationTrend::Improving));
    }

    #[test]
    fn test_trend_degrading() {
        let (_, repo) = setup();
        repo.save(&make_record("run-1", "deg-proj", 0.90)).unwrap();
        repo.save(&make_record("run-2", "deg-proj", 0.85)).unwrap();

        let summary = repo.get_summary("deg-proj").unwrap();
        assert!(matches!(summary.trend, EvaluationTrend::Degrading));
    }

    #[test]
    fn test_prune_older_than() {
        let (_, repo) = setup();
        repo.save(&make_record("old-1", "prune-proj", 0.80)).unwrap();

        // Prune records older than 999 days (nothing — records are fresh)
        let pruned = repo.prune_older_than(999).unwrap();
        assert_eq!(pruned, 0, "Fresh records should not be pruned");
    }
}
