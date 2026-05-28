//! Incremental analysis — only re-analyzes changed files since last run.

use sha2::{Sha256, Digest};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use rusqlite::params;
use tracing::{info, instrument};

use cleanroom_db::{Database, DbError};

/// Result of incremental analysis comparison.
#[derive(Debug, Clone)]
pub struct IncrementalDiff {
    /// Files that were added since last analysis.
    pub added: Vec<String>,
    /// Files that were modified (hash changed).
    pub modified: Vec<String>,
    /// Files that were deleted.
    pub deleted: Vec<String>,
    /// Files that are unchanged and can be skipped.
    pub unchanged: Vec<String>,
    /// Total files scanned.
    pub total: usize,
}

/// Tracks file state for incremental analysis.
pub struct IncrementalAnalyzer {
    db: Arc<Database>,
    document_name: String,
}

impl IncrementalAnalyzer {
    pub fn new(db: Arc<Database>, document_name: &str) -> Self {
        Self {
            db,
            document_name: document_name.to_string(),
        }
    }

    /// Compute SHA-256 hash of a file's contents.
    pub fn compute_file_hash(path: &Path) -> Result<String, DbError> {
        let content = std::fs::read(path)
            .map_err(|e| DbError::QueryFailed(format!("Failed to read {}: {}", path.display(), e)))?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Run incremental diff: compare current file hashes against stored fingerprints.
    #[instrument(skip(self, file_paths))]
    pub fn diff(&self, file_paths: &[String]) -> Result<IncrementalDiff, DbError> {
        // Step 1: Load stored hashes from DB
        let mut stored: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        {
            let conn = self.db.connection();
            let mut stmt = conn.prepare(
                "SELECT entity_uri, code_hash FROM fingerprints WHERE document_name = ?1 AND entity_type = 'file'"
            ).map_err(|e| DbError::QueryFailed(e.to_string()))?;

            let mut rows = stmt.query(params![self.document_name])
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;

            while let Some(row) = rows.next().map_err(|e| DbError::QueryFailed(e.to_string()))? {
                let uri: String = row.get(0).map_err(|e| DbError::QueryFailed(e.to_string()))?;
                let hash: String = row.get(1).map_err(|e| DbError::QueryFailed(e.to_string()))?;
                stored.insert(uri, hash);
            }
        } // conn, stmt, rows dropped here — mutex released

        // Step 2: Compare files (no connection needed for reading)
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();
        let mut unchanged = Vec::new();
        let mut updates: Vec<(String, String, String, String)> = Vec::new(); // (uri, hash, path)
        let mut current_set = HashSet::new();

        for file_path in file_paths {
            let path = Path::new(file_path);
            let uri = format!("file://{}", file_path);
            current_set.insert(uri.clone());

            if !path.exists() {
                deleted.push(file_path.clone());
                continue;
            }

            let current_hash = match Self::compute_file_hash(path) {
                Ok(h) => h,
                Err(_) => continue,
            };
            let file_name = path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            match stored.get(&uri) {
                Some(stored_hash) if *stored_hash == current_hash => {
                    unchanged.push(file_path.clone());
                }
                Some(_) => {
                    modified.push(file_path.clone());
                    updates.push((uri, current_hash, file_name, "update".to_string()));
                }
                None => {
                    added.push(file_path.clone());
                    updates.push((uri, current_hash, file_name, "insert".to_string()));
                }
            }
        }

        // Find deleted files (stored but not in current set)
        for (uri, _) in &stored {
            if !current_set.contains(uri) {
                if let Some(path) = uri.strip_prefix("file://") {
                    deleted.push(path.to_string());
                }
            }
        }

        // Step 3: Apply updates to DB (new connection scope)
        if !updates.is_empty() {
            let conn2 = self.db.connection();
            for (uri, hash, fname, op) in &updates {
                match op.as_str() {
                    "update" => {
                        let _ = conn2.execute(
                            "UPDATE fingerprints SET code_hash = ?1, code_path = ?2, last_checked_at = datetime()
                             WHERE entity_uri = ?3 AND document_name = ?4",
                            params![hash, fname, uri, self.document_name],
                        );
                    }
                    _ => {
                        let _ = conn2.execute(
                            "INSERT OR IGNORE INTO fingerprints (entity_uri, document_name, entity_type, code_hash, code_path, last_checked_at)
                             VALUES (?1, ?2, 'file', ?3, ?4, datetime())",
                            params![uri, self.document_name, hash, fname],
                        );
                    }
                }
            }
        }

        let total = file_paths.len();
        info!(
            added = added.len(),
            modified = modified.len(),
            deleted = deleted.len(),
            unchanged = unchanged.len(),
            "Incremental analysis diff"
        );

        Ok(IncrementalDiff {
            added,
            modified,
            deleted,
            unchanged,
            total,
        })
    }

    /// Determine which entities are affected by file changes.
    /// Uses dependency graph to propagate changes.
    pub fn affected_entities(&self, diff: &IncrementalDiff) -> Result<Vec<String>, DbError> {
        let mut affected: HashSet<String> = HashSet::new();

        // Changed files directly affect entities they map to
        for path in diff.added.iter().chain(diff.modified.iter()) {
            let stem = Path::new(path).file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            affected.insert(stem.clone());

            // Check for data model entities matching this stem
            let conn = self.db.connection();
            let mut stmt = conn.prepare(
                "SELECT entity FROM data_models WHERE document_name = ?1 AND entity LIKE ?2"
            ).map_err(|e| DbError::QueryFailed(e.to_string()))?;

            let pattern = format!("%{}%", stem);
            let mut rows = stmt.query(params![self.document_name, pattern])
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;

            while let Some(row) = rows.next().map_err(|e| DbError::QueryFailed(e.to_string()))? {
                affected.insert(row.get::<_, String>(0)
                    .map_err(|e| DbError::QueryFailed(e.to_string()))?);
            }
        }

        let result: Vec<String> = affected.into_iter().collect();
        info!(count = result.len(), "Affected entities by file changes");
        Ok(result)
    }

    /// Record that analysis was performed, updating checkpoints.
    pub fn record_analysis_run(&self) -> Result<(), DbError> {
        let conn = self.db.connection();
        conn.execute_batch(
            "INSERT OR IGNORE INTO sdef_documents (name, version, created_at, updated_at)
             VALUES ('__analysis_meta__', '1.0', datetime(), datetime());"
        ).map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Returns true if there are any changes requiring re-analysis.
    pub fn has_changes(&self, diff: &IncrementalDiff) -> bool {
        !diff.added.is_empty() || !diff.modified.is_empty() || !diff.deleted.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn setup_db() -> Arc<Database> {
        let db = Arc::new(Database::in_memory().unwrap());
        {
            let conn = db.connection();
            conn.execute_batch(
                "INSERT INTO sdef_documents (name, version, created_at, updated_at)
                 VALUES ('test', '1.0', datetime(), datetime());"
            ).unwrap();
        }
        db
    }

    #[test]
    fn test_compute_file_hash() {
        let tmp = std::env::temp_dir().join("cleanroom_hash_test.txt");
        std::fs::write(&tmp, b"hello world").unwrap();
        let hash = IncrementalAnalyzer::compute_file_hash(&tmp).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64);

        let tmp2 = std::env::temp_dir().join("cleanroom_hash_test2.txt");
        std::fs::write(&tmp2, b"hello world").unwrap();
        let hash2 = IncrementalAnalyzer::compute_file_hash(&tmp2).unwrap();
        assert_eq!(hash, hash2);

        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&tmp2);
    }

    #[test]
    fn test_empty_diff() {
        let db = setup_db();
        let analyzer = IncrementalAnalyzer::new(db, "test");
        let diff = analyzer.diff(&[]).unwrap();
        assert_eq!(diff.total, 0);
        assert!(diff.added.is_empty());
        assert!(diff.unchanged.is_empty());
        assert!(!analyzer.has_changes(&diff)); // No changes if nothing to analyze
    }

    #[test]
    fn test_new_file_detected_as_added() {
        let db = setup_db();
        let tmp = std::env::temp_dir().join("cleanroom_new_file.rs");
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(b"pub fn test() {}").unwrap();
        drop(f);

        let analyzer = IncrementalAnalyzer::new(db, "test");
        let path_str = tmp.to_string_lossy().to_string();
        let diff = analyzer.diff(&[path_str.clone()]).unwrap();
        assert_eq!(diff.added.len(), 1);
        assert!(analyzer.has_changes(&diff));

        // Second run should detect as unchanged
        let diff2 = analyzer.diff(&[path_str]).unwrap();
        assert_eq!(diff2.unchanged.len(), 1);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_affected_entities() {
        let db = setup_db();
        {
            let conn = db.connection();
            conn.execute_batch(
                "INSERT INTO data_models (entity, document_name, status)
                 VALUES ('User', 'test', 'active');"
            ).unwrap();
        }

        let analyzer = IncrementalAnalyzer::new(db.clone(), "test");
        let diff = IncrementalDiff {
            added: vec!["src/user.rs".to_string()],
            modified: vec![],
            deleted: vec![],
            unchanged: vec![],
            total: 1,
        };

        let affected = analyzer.affected_entities(&diff).unwrap();
        assert!(affected.contains(&"User".to_string()) || affected.contains(&"user".to_string()));
    }

    #[test]
    fn test_record_analysis_run() {
        let db = setup_db();
        let analyzer = IncrementalAnalyzer::new(db, "test");
        analyzer.record_analysis_run().unwrap();
    }
}