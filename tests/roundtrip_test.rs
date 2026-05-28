//! Roundtrip integration test: S.DEF JSON → import → export → verify.

use cleanroom_db::Database;
use cleanroom_db::export_import::{SdefExporter, SdefImporter};
use sdef_core::SoftwareDefinition;

const EXPECTED_SDEF: &str = "tests/fixtures/todo-app/expected.sdef.json";

fn fixture_path(relative: &str) -> String {
    if std::path::Path::new(relative).exists() {
        return relative.to_string();
    }
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let candidate = dir.join(relative);
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
        if !dir.pop() {
            panic!("Cannot find fixture: {}", relative);
        }
    }
}

/// Create a temporary file-based database and return (db, path).
fn create_temp_db() -> (Database, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("cleanroom_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("Create temp dir");
    let db_path = dir.join("state.db");
    let db = Database::open(&db_path).expect("Create temp DB");
    (db, db_path)
}

#[test]
fn test_sdef_roundtrip() {
    let path = fixture_path(EXPECTED_SDEF);
    let content = std::fs::read_to_string(path).expect("Read fixture");
    let original: SoftwareDefinition = serde_json::from_str(&content).expect("Parse S.DEF");

    assert!(!original.name.is_empty());

    // File-based DB so multiple connections can coexist (WAL mode)
    let (_db, db_path) = create_temp_db();

    // Import via SdefImporter (opens its own connection)
    SdefImporter::new(rusqlite::Connection::open(&db_path).expect("Conn 1"))
        .import(&original)
        .expect("Import");

    // Export via SdefExporter (another connection)
    let exported = SdefExporter::new(rusqlite::Connection::open(&db_path).expect("Conn 2"))
        .export(&original.name)
        .expect("Export");

    assert_eq!(exported.name, original.name, "Name mismatch");
    assert_eq!(exported.description, original.description, "Description mismatch");

    let orig_models = original.data_models.unwrap_or_default();
    let exp_models = exported.data_models.unwrap_or_default();
    assert_eq!(orig_models.len(), exp_models.len(), "Model count mismatch");

    for (orig, exp) in orig_models.iter().zip(exp_models.iter()) {
        assert_eq!(orig.entity, exp.entity, "Entity '{}' mismatch", orig.entity);
        let orig_attrs = orig.attributes.as_ref().map(|v| v.len()).unwrap_or(0);
        let exp_attrs = exp.attributes.as_ref().map(|v| v.len()).unwrap_or(0);
        assert_eq!(orig_attrs, exp_attrs, "Attr count for '{}'", orig.entity);
    }
}

#[test]
fn test_import_idempotent() {
    let path = fixture_path(EXPECTED_SDEF);
    let content = std::fs::read_to_string(path).expect("Read fixture");
    let sdef: SoftwareDefinition = serde_json::from_str(&content).expect("Parse");

    let (_db, db_path) = create_temp_db();

    // Import twice
    SdefImporter::new(rusqlite::Connection::open(&db_path).expect("Conn 1"))
        .import(&sdef).expect("Import 1");
    SdefImporter::new(rusqlite::Connection::open(&db_path).expect("Conn 2"))
        .import(&sdef).expect("Import 2");

    // Verify: count documents
    let verify_conn = rusqlite::Connection::open(&db_path).expect("Conn 3");
    let count: i64 = verify_conn
        .query_row("SELECT COUNT(*) FROM sdef_documents", [], |r| r.get(0))
        .expect("Count");
    assert_eq!(count, 1, "Should have exactly 1 document after idempotent import");
}

#[test]
fn test_json_serialization_roundtrip() {
    let path = fixture_path(EXPECTED_SDEF);
    let content = std::fs::read_to_string(path).expect("Read fixture");
    let original: SoftwareDefinition = serde_json::from_str(&content).expect("Parse");

    let json = serde_json::to_string_pretty(&original).expect("Serialize");
    let restored: SoftwareDefinition = serde_json::from_str(&json).expect("Deserialize");

    assert_eq!(original.name, restored.name);
    assert_eq!(original.sdef_version, restored.sdef_version, "Version mismatch");
    assert_eq!(original.description, restored.description, "Description mismatch");
}
