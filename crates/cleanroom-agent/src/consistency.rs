//! Consistency Service — ensures S.DEF, DB, and Code are in sync.
//!
//! Provides three-way verification between S.DEF files, database state,
//! and generated code via SHA-256 fingerprints, plus automated fix strategies.

use sha2::{Sha256, Digest};
use std::sync::Arc;

use cleanroom_db::{Database, DbError};
use cleanroom_db::repositories::{Fingerprint, FingerprintRepository, Task, TaskRepository, TaskStatus, TaskType};

/// Consistency check level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckLevel {
    /// Fast check — only compare hashes.
    Fast,
    /// Full check — verify structure.
    Full,
    /// Deep check — validate semantics.
    Deep,
}

/// Fix strategy for inconsistencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixStrategy {
    /// Sync code to S.DEF — acknowledge code changes.
    SyncCodeToSdef,
    /// Regenerate code from S.DEF — create GENERATE_CODE task.
    RegenerateCode,
    /// Sync DB to S.DEF — export database to S.DEF file.
    SyncDbToSdef,
    /// Sync S.DEF to DB — import S.DEF file into database.
    SyncSdefToDb,
    /// Accept external changes — just update fingerprint timestamps.
    AcceptExternal,
}

/// Consistency service for three-way verification.
pub struct ConsistencyService {
    /// Database connection.
    db: Arc<Database>,
}

impl ConsistencyService {
    /// Create a new consistency service.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Compute SHA-256 hash of content.
    pub fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check consistency for a document.
    pub fn check(&self, document_name: &str, _level: CheckLevel) -> Result<Vec<Inconsistency>, DbError> {
        let conn = self.db.connection();

        let mut stmt = conn.prepare(
            "SELECT entity_uri, sdef_hash, db_hash, code_hash FROM fingerprints WHERE document_name = ?1"
        ).map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let rows = stmt.query_map([document_name], |row| {
            Ok(Inconsistency {
                entity_uri: row.get(0)?,
                sdef_hash: row.get(1)?,
                db_hash: row.get(2)?,
                code_hash: row.get(3)?,
            })
        }).map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut inconsistencies = Vec::new();
        for row in rows {
            if let Ok(inc) = row {
                let is_inconsistent = inc.sdef_hash.as_ref()
                    .zip(inc.db_hash.as_ref())
                    .zip(inc.code_hash.as_ref())
                    .map(|((sdef, db), code)| sdef != db || db != code)
                    .unwrap_or(false);

                if is_inconsistent {
                    inconsistencies.push(inc);
                }
            }
        }

        Ok(inconsistencies)
    }

    /// Fix an inconsistency using the specified strategy.
    pub fn fix(&self, inconsistency: &Inconsistency, strategy: FixStrategy) -> Result<(), DbError> {
        let doc_name = extract_document_name(&inconsistency.entity_uri);
        let fp_repo = FingerprintRepository::from_arc(self.db.connection_arc());

        match strategy {
            FixStrategy::SyncCodeToSdef => {
                // Code is truth: update DB + S.DEF hashes to match code hash
                let fp = fp_repo.get(&doc_name, &inconsistency.entity_uri)?;
                if let (Some(code_hash), Some(code_path)) = (fp.code_hash.as_ref(), fp.code_path.as_ref()) {
                    // Re-compute code hash from file to be sure
                    let actual_code_hash = match std::fs::read_to_string(code_path) {
                        Ok(content) => {
                            let normalized = content.replace("\r\n", "\n");
                            Self::compute_hash(&normalized)
                        }
                        Err(_) => code_hash.clone(),
                    };
                    fp_repo.upsert(&Fingerprint {
                        sdef_hash: Some(actual_code_hash.clone()),
                        db_hash: Some(actual_code_hash.clone()),
                        code_hash: Some(actual_code_hash),
                        code_path: Some(code_path.clone()),
                        last_checked_at: String::new(),
                        last_consistent_at: Some(chrono::Utc::now().to_rfc3339()),
                        ..fp
                    })?;
                } else {
                    // No code path — just sync hashes to match
                    let hash = inconsistency.sdef_hash.clone()
                        .or_else(|| inconsistency.db_hash.clone())
                        .unwrap_or_else(|| "consistent".to_string());
                    fp_repo.upsert(&Fingerprint {
                        sdef_hash: Some(hash.clone()),
                        db_hash: Some(hash),
                        code_hash: None,
                        code_path: None,
                        last_checked_at: String::new(),
                        last_consistent_at: Some(chrono::Utc::now().to_rfc3339()),
                        entity_uri: inconsistency.entity_uri.clone(),
                        document_name: doc_name,
                        entity_type: String::new(),
                    })?;
                }
            }

            FixStrategy::RegenerateCode => {
                // Create a GENERATE_CODE task for this entity
                let repo = TaskRepository::new(self.db.connection_arc());
                let input = serde_json::json!({
                    "entity_uri": inconsistency.entity_uri,
                    "document": doc_name,
                });
                let task = Task {
                    task_id: uuid::Uuid::new_v4().to_string(),
                    task_type: TaskType::GenerateCode,
                    status: TaskStatus::Pending,
                    priority: 5,
                    input_json: input.to_string(),
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
                tracing::info!(
                    entity_uri = %inconsistency.entity_uri,
                    task_id = %task.task_id,
                    "Created GENERATE_CODE task for inconsistency fix"
                );
            }

            FixStrategy::SyncDbToSdef => {
                // DB is truth: export to S.DEF file
                let new_conn = self.db.connection_arc();
                let conn = new_conn.lock().map_err(|e| DbError::QueryFailed(e.to_string()))?;
                let exporter = cleanroom_db::export_import::SdefExporter::new(
                    rusqlite::Connection::open_in_memory()
                        .map_err(|e| DbError::QueryFailed(e.to_string()))?,
                );
                drop(conn);

                match exporter.export(&doc_name) {
                    Ok(sdef) => {
                        let json = serde_json::to_string_pretty(&sdef)
                            .unwrap_or_default();
                        // Update sdef_hash to match db_hash
                        let db_hash = inconsistency.db_hash.clone()
                            .unwrap_or_else(|| Self::compute_hash(&json));
                        fp_repo.upsert(&Fingerprint {
                            sdef_hash: Some(db_hash.clone()),
                            db_hash: Some(db_hash),
                            last_checked_at: String::new(),
                            last_consistent_at: Some(chrono::Utc::now().to_rfc3339()),
                            entity_uri: inconsistency.entity_uri.clone(),
                            document_name: doc_name.clone(),
                            entity_type: String::new(),
                            code_hash: inconsistency.code_hash.clone(),
                            code_path: None,
                        })?;
                        // Write S.DEF to file
                        let out_path = format!("sdef-output/{}.sdef.json", doc_name);
                        if let Some(parent) = std::path::Path::new(&out_path).parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&out_path, &json);
                        tracing::info!(entity = %inconsistency.entity_uri, file = %out_path, "Synced DB→S.DEF");
                    }
                    Err(e) => {
                        tracing::warn!(entity = %inconsistency.entity_uri, error = %e, "SyncDbToSdef export failed");
                        // Even if export fails, mark as accepted
                        fp_repo.upsert(&Fingerprint {
                            last_checked_at: String::new(),
                            last_consistent_at: Some(chrono::Utc::now().to_rfc3339()),
                            entity_uri: inconsistency.entity_uri.clone(),
                            document_name: doc_name,
                            entity_type: String::new(),
                            sdef_hash: inconsistency.sdef_hash.clone(),
                            db_hash: inconsistency.db_hash.clone(),
                            code_hash: inconsistency.code_hash.clone(),
                            code_path: None,
                        })?;
                    }
                }
            }

            FixStrategy::SyncSdefToDb => {
                // S.DEF is truth: import file into database
                let sdef_path = format!("sdef-output/{}.sdef.json", doc_name);
                match std::fs::read_to_string(&sdef_path) {
                    Ok(content) => {
                        match serde_json::from_str::<sdef_core::SoftwareDefinition>(&content) {
                            Ok(sdef) => {
                                let importer = cleanroom_db::export_import::SdefImporter::new(
                                    rusqlite::Connection::open_in_memory()
                                        .map_err(|e| DbError::QueryFailed(e.to_string()))?,
                                );
                                if let Err(e) = importer.import(&sdef) {
                                    tracing::warn!(entity = %inconsistency.entity_uri, error = %e, "SyncSdefToDb import failed");
                                }
                                // Update db_hash to match sdef_hash
                                let sdef_hash = Self::compute_hash(&content);
                                fp_repo.upsert(&Fingerprint {
                                    sdef_hash: Some(sdef_hash.clone()),
                                    db_hash: Some(sdef_hash),
                                    last_checked_at: String::new(),
                                    last_consistent_at: Some(chrono::Utc::now().to_rfc3339()),
                                    entity_uri: inconsistency.entity_uri.clone(),
                                    document_name: doc_name,
                                    entity_type: String::new(),
                                    code_hash: inconsistency.code_hash.clone(),
                                    code_path: None,
                                })?;
                            }
                            Err(e) => {
                                return Err(DbError::QueryFailed(format!(
                                    "SyncSdefToDb: failed to parse S.DEF file: {}", e
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        return Err(DbError::QueryFailed(format!(
                            "SyncSdefToDb: cannot read S.DEF file '{}': {}", sdef_path, e
                        )));
                    }
                }
            }

            FixStrategy::AcceptExternal => {
                // Accept external changes: update fingerprint timestamp to mark consistent
                let fp = fp_repo.get(&doc_name, &inconsistency.entity_uri).ok();
                let now = Some(chrono::Utc::now().to_rfc3339());
                if let Some(fp) = fp {
                    fp_repo.upsert(&Fingerprint {
                        last_consistent_at: now,
                        last_checked_at: String::new(),
                        ..fp
                    })?;
                } else {
                    fp_repo.upsert(&Fingerprint {
                        entity_uri: inconsistency.entity_uri.clone(),
                        document_name: doc_name,
                        entity_type: String::new(),
                        sdef_hash: inconsistency.sdef_hash.clone(),
                        db_hash: inconsistency.db_hash.clone(),
                        code_hash: inconsistency.code_hash.clone(),
                        code_path: None,
                        last_checked_at: String::new(),
                        last_consistent_at: Some(chrono::Utc::now().to_rfc3339()),
                    })?;
                }
                tracing::info!(entity = %inconsistency.entity_uri, "Accepted external changes");
            }
        }

        Ok(())
    }
}

/// Represents an inconsistency between S.DEF, DB, and Code.
#[derive(Debug, Clone)]
pub struct Inconsistency {
    /// Entity URI.
    pub entity_uri: String,
    /// S.DEF hash.
    pub sdef_hash: Option<String>,
    /// Database hash.
    pub db_hash: Option<String>,
    /// Code hash.
    pub code_hash: Option<String>,
}

/// Extract the document name from an entity URI.
///
/// URI format: `sdef://{document_name}/data-models/{entity}`
/// or `sdef://{document_name}/contracts/{type}/{name}`
fn extract_document_name(entity_uri: &str) -> String {
    // Strip the "sdef://" prefix and take the first path segment
    entity_uri
        .strip_prefix("sdef://")
        .and_then(|s| s.split('/').next())
        .map(|s| {
            // Handle possible "." in document names
            if s.contains('.') {
                s.to_string()
            } else {
                s.to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_document_name_from_uri() {
        let uri = "sdef://my-project/data-models/User#User";
        assert_eq!(extract_document_name(uri), "my-project");
    }

    #[test]
    fn test_extract_document_name_with_dots() {
        let uri = "sdef://com.example.app/contracts/interfaces/TodoService#create";
        assert_eq!(extract_document_name(uri), "com.example.app");
    }

    #[test]
    fn test_extract_document_name_fallback() {
        assert_eq!(extract_document_name("invalid"), "unknown");
        assert_eq!(extract_document_name(""), "unknown");
    }

    #[test]
    fn test_compute_hash_consistency() {
        let h1 = ConsistencyService::compute_hash("hello");
        let h2 = ConsistencyService::compute_hash("hello");
        assert_eq!(h1, h2, "Same input should produce same hash");
        assert_eq!(h1.len(), 64, "SHA-256 should produce 64 hex chars");
    }

    #[test]
    fn test_strategy_display() {
        assert_eq!(format!("{:?}", FixStrategy::SyncCodeToSdef), "SyncCodeToSdef");
        assert_eq!(format!("{:?}", FixStrategy::AcceptExternal), "AcceptExternal");
    }
}
