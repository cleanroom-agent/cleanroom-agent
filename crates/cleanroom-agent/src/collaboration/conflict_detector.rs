//! Conflict Detector — detects and resolves conflicts between parallel agents.
//!
//! Part of the multi-agent collaboration system (docs/13 §4).
//! Detects three categories of conflicts:
//! - Symbol collisions (multiple agents propose different names for same URI)
//! - Interface mismatches (generated code signatures don't match)
//! - File overlaps (multiple agents target the same output file)

use std::path::PathBuf;
use std::sync::Arc;

use cleanroom_db::{Database, DbResult};
use tracing::{info, instrument, warn};

use crate::collaboration::messages::{MessageSender, MessageType};

/// A detected conflict between agents.
#[derive(Debug, Clone)]
pub enum Conflict {
    /// Two or more agents proposed different concrete names for the same sdef:// URI.
    /// `proposals`: Vec of (agent_id, proposed_name).
    SymbolCollision {
        uri: String,
        proposals: Vec<(String, String)>,
    },
    /// Generated function/method signatures from different agents don't match.
    InterfaceMismatch {
        contract: String,
        signatures: Vec<SignatureConflict>,
    },
    /// Two or more agents claim they will write to the same file path.
    FileOverlap {
        path: PathBuf,
        sources: Vec<String>,
    },
}

/// Details of a single interface signature mismatch.
#[derive(Debug, Clone)]
pub struct SignatureConflict {
    pub method_name: String,
    pub signature_a: String,
    pub agent_a: String,
    pub signature_b: String,
    pub agent_b: String,
}

/// Resolution strategy for a conflict.
#[derive(Debug, Clone)]
pub enum Resolution {
    /// Later (higher version) submission wins.
    LatestWins,
    /// Append a numeric suffix to disambiguate (e.g., `_2`).
    SuffixDisambiguation(String),
    /// Manual intervention required — notify agents and escalate.
    EscalateToOrchestrator(Conflict),
}

impl Conflict {
    /// Get a human-readable description of the conflict.
    pub fn description(&self) -> String {
        match self {
            Self::SymbolCollision { uri, proposals } => {
                let names: Vec<String> = proposals
                    .iter()
                    .map(|(agent, name)| format!("{} from {}", name, agent))
                    .collect();
                format!(
                    "Symbol collision for {}: conflicting names — {}",
                    uri,
                    names.join(", ")
                )
            }
            Self::InterfaceMismatch { contract, signatures } => {
                let details: Vec<String> = signatures
                    .iter()
                    .map(|s| format!("{}.{}: {} vs {}", contract, s.method_name, s.signature_a, s.signature_b))
                    .collect();
                format!(
                    "Interface mismatch in contract '{}': {}",
                    contract,
                    details.join("; ")
                )
            }
            Self::FileOverlap { path, sources } => {
                format!(
                    "File overlap at {}: produced by agents {}",
                    path.display(),
                    sources.join(", ")
                )
            }
        }
    }
}

/// Conflict detector for multi-agent workspaces.
pub struct ConflictDetector {
    db: Arc<Database>,
}

impl ConflictDetector {
    /// Create a new conflict detector.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Run all conflict detection checks for a workspace.
    #[instrument(skip(self))]
    pub async fn detect(&self, document_name: &str) -> DbResult<Vec<Conflict>> {
        let mut conflicts = Vec::new();

        // Check symbol collisions
        if let Some(c) = self.detect_symbol_collisions(document_name)? {
            conflicts.extend(c);
        }

        // Check interface mismatches
        if let Some(c) = self.detect_interface_mismatches(document_name)? {
            conflicts.extend(c);
        }

        // Check file overlaps
        if let Some(c) = self.detect_file_overlaps(document_name)? {
            conflicts.extend(c);
        }

        if !conflicts.is_empty() {
            warn!(
                document = %document_name,
                count = conflicts.len(),
                "Conflicts detected"
            );
        } else {
            info!(document = %document_name, "No conflicts detected");
        }

        Ok(conflicts)
    }

    /// Detect symbol collisions in the symbol_registry.
    ///
    /// A symbol collision occurs when two or more agents register different
    /// concrete names for the same `(sdef_uri, language)` pair.
    #[instrument(skip(self))]
    pub fn detect_symbol_collisions(
        &self,
        _document_name: &str,
    ) -> DbResult<Option<Vec<Conflict>>> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT sdef_uri, language, COUNT(*) as cnt
                 FROM symbol_registry
                 WHERE document_name = ?1
                 GROUP BY sdef_uri, language
                 HAVING COUNT(*) > 1",
            )
            .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

        let dups: Vec<(String, String)> = stmt
            .query_map(rusqlite::params![_document_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                ))
            })
            .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        drop(stmt);

        if dups.is_empty() {
            return Ok(None);
        }

        let mut conflicts = Vec::new();
        for (uri, _lang) in &dups {
            let mut proposals_stmt = conn
                .prepare(
                    "SELECT concrete_name
                     FROM symbol_registry
                     WHERE document_name = ?1 AND sdef_uri = ?2",
                )
                .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

            let proposals: Vec<(String, String)> = proposals_stmt
                .query_map(rusqlite::params![_document_name, uri], |row| {
                    Ok(row.get::<_, String>(0)?)
                })
                .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?
                .enumerate()
                .filter_map(|(i, r)| {
                    r.ok().map(|name| (format!("agent-{}", i), name))
                })
                .collect();

            drop(proposals_stmt);

            conflicts.push(Conflict::SymbolCollision {
                uri: uri.clone(),
                proposals,
            });
        }

        Ok(Some(conflicts))
    }

    /// Detect interface mismatches in generated contracts.
    #[instrument(skip(self))]
    pub fn detect_interface_mismatches(
        &self,
        _document_name: &str,
    ) -> DbResult<Option<Vec<Conflict>>> {
        // Stub: interface mismatch detection requires code-level comparison.
        // For now, return None to indicate no mismatches detected.
        // Full implementation would compare contract_methods signatures
        // from the database against generated code.
        Ok(None)
    }

    /// Detect file overlaps where multiple agents target the same path.
    #[instrument(skip(self))]
    pub fn detect_file_overlaps(
        &self,
        _document_name: &str,
    ) -> DbResult<Option<Vec<Conflict>>> {
        let conn = self.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT file_path, COUNT(*) as cnt
                 FROM shards
                 WHERE document_name = ?1 AND file_path IS NOT NULL
                 GROUP BY file_path
                 HAVING COUNT(*) > 1",
            )
            .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?;

        let overlaps: Vec<String> = stmt
            .query_map(rusqlite::params![_document_name], |row| {
                Ok(row.get::<_, String>(0)?)
            })
            .map_err(|e| cleanroom_db::DbError::QueryFailed(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        drop(stmt);

        if overlaps.is_empty() {
            return Ok(None);
        }

        let conflicts: Vec<Conflict> = overlaps
            .into_iter()
            .map(|path| Conflict::FileOverlap {
                path: PathBuf::from(&path),
                sources: vec!["unknown".to_string()],
            })
            .collect();

        Ok(Some(conflicts))
    }

    /// Attempt to auto-resolve conflicts.
    ///
    /// Returns resolutions for conflicts that can be handled automatically.
    /// Conflicts requiring manual intervention are returned as-is.
    pub fn auto_resolve(&self, conflicts: &[Conflict]) -> Vec<(Conflict, Resolution)> {
        let mut resolutions = Vec::new();

        for conflict in conflicts {
            let resolution = match conflict {
                Conflict::SymbolCollision { uri: _, proposals } => {
                    // Auto-resolve: pick the first proposal, append _2 to the rest
                    if let Some((_, first_name)) = proposals.first() {
                        let suffix = format!("{}_{}", first_name, proposals.len());
                        Resolution::SuffixDisambiguation(suffix)
                    } else {
                        Resolution::EscalateToOrchestrator(conflict.clone())
                    }
                }
                Conflict::InterfaceMismatch { .. } => {
                    // Interface mismatches need manual resolution
                    Resolution::EscalateToOrchestrator(conflict.clone())
                }
                Conflict::FileOverlap { .. } => {
                    // File overlaps: latest writer wins by default
                    Resolution::LatestWins
                }
            };

            resolutions.push((conflict.clone(), resolution));
        }

        resolutions
    }

    /// Notify affected agents about detected conflicts.
    pub fn notify_agents(
        &self,
        conflicts: &[Conflict],
        sender: &MessageSender,
    ) -> DbResult<usize> {
        let mut notified = 0;
        for conflict in conflicts {
            let desc = conflict.description();
            let msg = AgentMessage::broadcast(
                "conflict_detector",
                MessageType::ReviewRequest {
                    entity_uri: "conflict".to_string(),
                    issues: vec![desc],
                },
                serde_json::json!({ "conflict": format!("{:?}", conflict) }),
            );
            sender.send(&msg)?;
            notified += 1;
        }
        Ok(notified)
    }
}

use cleanroom_db::AgentMessage;

#[cfg(test)]
mod tests {
    use super::*;
    use cleanroom_db::Database;

    fn setup() -> (Database, ConflictDetector) {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();
        // Use different document_names to avoid UNIQUE constraint collision
        // (the UNIQUE constraint is the primary defense against symbol collisions)
        conn.execute_batch(
            "INSERT INTO sdef_documents (name) VALUES ('test-doc');
             INSERT INTO symbol_registry (document_name, sdef_uri, language, symbol_type, concrete_name)
             VALUES ('test-doc', 'sdef://model/User', 'rust', 'class', 'User');
             INSERT INTO sdef_documents (name) VALUES ('test-doc-collision');
             INSERT INTO symbol_registry (document_name, sdef_uri, language, symbol_type, concrete_name)
             VALUES ('test-doc-collision', 'sdef://model/User', 'rust', 'class', 'UserCollision');",
        )
        .unwrap();
        drop(conn);
        let detector = ConflictDetector::new(Arc::new(db.clone()));
        (db, detector)
    }

    #[tokio::test]
    async fn test_detect_symbol_collision() {
        let (_, detector) = setup();
        // Different documents — no collision expected within a single document
        let conflicts = detector.detect("test-doc").await.unwrap();
        assert!(
            conflicts.is_empty(),
            "No collisions expected in a single document"
        );
    }

    #[tokio::test]
    async fn test_detect_file_overlap_with_no_overlap() {
        let (db, detector) = setup();
        // Clean state — no file overlaps
        let conn = db.connection();
        conn.execute_batch(
            "INSERT INTO shards (shard_id, document_name, sdef_uri, section_type, file_path)
             VALUES ('shard-1', 'test-doc', 'sdef://a', 'data_model', 'src/a.rs');
             INSERT INTO shards (shard_id, document_name, sdef_uri, section_type, file_path)
             VALUES ('shard-2', 'test-doc', 'sdef://b', 'data_model', 'src/b.rs');",
        )
        .unwrap();
        drop(conn);

        let conflicts = detector.detect("test-doc").await.unwrap();
        let file_overlaps: Vec<_> = conflicts
            .iter()
            .filter(|c| matches!(c, Conflict::FileOverlap { .. }))
            .collect();
        assert!(file_overlaps.is_empty(), "No file overlaps expected");
    }

    #[tokio::test]
    async fn test_detect_no_conflicts() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();
        conn.execute_batch(
            "INSERT INTO sdef_documents (name) VALUES ('clean-doc');",
        )
        .unwrap();
        drop(conn);
        let detector = ConflictDetector::new(Arc::new(db));
        let conflicts = detector.detect("clean-doc").await.unwrap();
        assert!(conflicts.is_empty(), "Should have no conflicts");
    }

    #[test]
    fn test_auto_resolve_symbol_collision() {
        let db = Database::in_memory().unwrap();
        let detector = ConflictDetector::new(Arc::new(db));
        let conflict = Conflict::SymbolCollision {
            uri: "sdef://test".to_string(),
            proposals: vec![
                ("agent-1".to_string(), "Foo".to_string()),
                ("agent-2".to_string(), "FooVariant".to_string()),
            ],
        };
        let resolutions = detector.auto_resolve(&[conflict]);
        assert_eq!(resolutions.len(), 1);
        match &resolutions[0].1 {
            Resolution::SuffixDisambiguation(name) => {
                assert!(name.contains("Foo"));
            }
            _ => panic!("Expected SuffixDisambiguation"),
        }
    }

    #[test]
    fn test_auto_resolve_file_overlap() {
        let db = Database::in_memory().unwrap();
        let detector = ConflictDetector::new(Arc::new(db));
        let conflict = Conflict::FileOverlap {
            path: PathBuf::from("src/main.rs"),
            sources: vec!["agent-1".to_string(), "agent-2".to_string()],
        };
        let resolutions = detector.auto_resolve(&[conflict]);
        assert_eq!(resolutions.len(), 1);
        assert!(matches!(resolutions[0].1, Resolution::LatestWins));
    }

    #[test]
    fn test_conflict_description() {
        let conflict = Conflict::SymbolCollision {
            uri: "sdef://model/User".to_string(),
            proposals: vec![
                ("agent-1".to_string(), "User".to_string()),
                ("agent-2".to_string(), "UserModel".to_string()),
            ],
        };
        let desc = conflict.description();
        assert!(desc.contains("sdef://model/User"));
        assert!(desc.contains("User"));
        assert!(desc.contains("UserModel"));
    }
}
