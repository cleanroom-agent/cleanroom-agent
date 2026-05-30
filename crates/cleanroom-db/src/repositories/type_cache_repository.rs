//! Type cache repository — shared knowledge across agents.
//!
//! Stores resolved type information that has been computed by LSP analysis,
//! so other agents can look up types without re-running expensive analysis.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::instrument;

use crate::error::{DbError, DbResult};

/// Cached type information shared across agents (docs/13 §6.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCacheEntry {
    pub entity_uri: String,
    pub language: String,
    pub resolved_type: String,
    pub source_file: Option<String>,
    pub from_lsp: bool,
    pub cached_at: String,
}

/// Repository for type cache operations.
pub struct TypeCacheRepository {
    conn: Arc<Mutex<Connection>>,
}

impl TypeCacheRepository {
    /// Create a new repository.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Cache a resolved type entry. Upserts (INSERT OR REPLACE) to keep latest.
    #[instrument(skip_all)]
    pub fn cache(&self, entry: &TypeCacheEntry) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO type_cache (entity_uri, language, resolved_type, source_file, from_lsp, cached_at)
             VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)",
            params![
                entry.entity_uri,
                entry.language,
                entry.resolved_type,
                entry.source_file,
                entry.from_lsp,
            ],
        )
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Lookup a single cached type entry.
    #[instrument(skip_all)]
    pub fn lookup(&self, entity_uri: &str, language: &str) -> DbResult<Option<TypeCacheEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT entity_uri, language, resolved_type, source_file, from_lsp, cached_at
                 FROM type_cache WHERE entity_uri = ?1 AND language = ?2",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![entity_uri, language], |row| {
            Ok(TypeCacheEntry {
                entity_uri: row.get(0)?,
                language: row.get(1)?,
                resolved_type: row.get(2)?,
                source_file: row.get(3)?,
                from_lsp: row.get(4)?,
                cached_at: row.get(5)?,
            })
        });

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::QueryFailed(e.to_string())),
        }
    }

    /// Lookup multiple cached type entries by URI.
    #[instrument(skip_all)]
    pub fn lookup_batch(
        &self,
        uris: &[(String, String)],
    ) -> DbResult<Vec<TypeCacheEntry>> {
        if uris.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().unwrap();

        // Build (entity_uri = ? AND language = ?) OR ... clauses
        let mut conditions = Vec::new();
        let mut flat_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for (uri, lang) in uris {
            conditions.push("(entity_uri = ? AND language = ?)".to_string());
            flat_params.push(Box::new(uri.clone()));
            flat_params.push(Box::new(lang.clone()));
        }

        let query = format!(
            "SELECT entity_uri, language, resolved_type, source_file, from_lsp, cached_at
             FROM type_cache WHERE {}",
            conditions.join(" OR ")
        );

        let mut stmt = conn.prepare(&query).map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = flat_params.iter().map(|p| p.as_ref()).collect();

        let entries = stmt
            .query_map(rusqlite::params_from_iter(param_refs), |row| {
                Ok(TypeCacheEntry {
                    entity_uri: row.get(0)?,
                    language: row.get(1)?,
                    resolved_type: row.get(2)?,
                    source_file: row.get(3)?,
                    from_lsp: row.get(4)?,
                    cached_at: row.get(5)?,
                })
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Invalidate entries for a specific entity URI.
    #[instrument(skip_all)]
    pub fn invalidate(&self, entity_uri: &str) -> DbResult<usize> {
        let conn = self.conn.lock().unwrap();
        let rows = conn
            .execute(
                "DELETE FROM type_cache WHERE entity_uri = ?1",
                params![entity_uri],
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows)
    }

    /// Clear all cached entries for a given language.
    #[instrument(skip_all)]
    pub fn clear_by_language(&self, language: &str) -> DbResult<usize> {
        let conn = self.conn.lock().unwrap();
        let rows = conn
            .execute(
                "DELETE FROM type_cache WHERE language = ?1",
                params![language],
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    fn setup() -> (Database, TypeCacheRepository) {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS type_cache (
                entity_uri TEXT NOT NULL,
                language TEXT NOT NULL,
                resolved_type TEXT NOT NULL,
                source_file TEXT,
                from_lsp BOOLEAN NOT NULL DEFAULT 1,
                cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (entity_uri, language)
            );",
        )
        .unwrap();
        drop(conn);
        let repo = TypeCacheRepository::new(db.connection_arc());
        (db, repo)
    }

    fn sample_entry(uri: &str, lang: &str, resolved: &str) -> TypeCacheEntry {
        TypeCacheEntry {
            entity_uri: uri.to_string(),
            language: lang.to_string(),
            resolved_type: resolved.to_string(),
            source_file: Some("src/models.rs".to_string()),
            from_lsp: true,
            cached_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn test_cache_and_lookup() {
        let (_, repo) = setup();
        let entry = sample_entry("sdef://model/User", "rust", "struct User");

        repo.cache(&entry).unwrap();

        let found = repo.lookup("sdef://model/User", "rust").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().resolved_type, "struct User");
    }

    #[test]
    fn test_lookup_miss() {
        let (_, repo) = setup();
        let result = repo.lookup("sdef://nonexistent", "rust").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_upsert_overwrites() {
        let (_, repo) = setup();
        let entry1 = sample_entry("sdef://model/User", "rust", "struct User");
        repo.cache(&entry1).unwrap();

        let entry2 = TypeCacheEntry {
            resolved_type: "struct UserV2".to_string(),
            ..entry1
        };
        repo.cache(&entry2).unwrap();

        let found = repo.lookup("sdef://model/User", "rust").unwrap();
        assert_eq!(found.unwrap().resolved_type, "struct UserV2");
    }

    #[test]
    fn test_invalidate() {
        let (_, repo) = setup();
        let entry = sample_entry("sdef://model/User", "rust", "struct User");
        repo.cache(&entry).unwrap();

        let deleted = repo.invalidate("sdef://model/User").unwrap();
        assert_eq!(deleted, 1);

        let found = repo.lookup("sdef://model/User", "rust").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_clear_by_language() {
        let (_, repo) = setup();
        repo.cache(&sample_entry("sdef://a", "rust", "type A")).unwrap();
        repo.cache(&sample_entry("sdef://b", "rust", "type B")).unwrap();
        repo.cache(&sample_entry("sdef://c", "typescript", "type C")).unwrap();

        let cleared = repo.clear_by_language("rust").unwrap();
        assert_eq!(cleared, 2);

        let found = repo.lookup("sdef://c", "typescript").unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn test_lookup_batch() {
        let (_, repo) = setup();
        repo.cache(&sample_entry("sdef://a", "rust", "type A")).unwrap();
        repo.cache(&sample_entry("sdef://b", "rust", "type B")).unwrap();
        repo.cache(&sample_entry("sdef://c", "tp", "type C")).unwrap();

        let uris = vec![
            ("sdef://a".to_string(), "rust".to_string()),
            ("sdef://b".to_string(), "rust".to_string()),
            ("sdef://none".to_string(), "rust".to_string()),
        ];
        let results = repo.lookup_batch(&uris).unwrap();
        assert_eq!(results.len(), 2);
    }
}
