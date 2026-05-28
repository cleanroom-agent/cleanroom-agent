//! Roundtrip integration test: S.DEF JSON → import into DB → verify.

use cleanroom_db::Database;
use sdef_core::SoftwareDefinition;

fn fixture_path() -> String {
    let relative = "tests/fixtures/todo-app/expected.sdef.json";
    if std::path::Path::new(relative).exists() {
        return relative.to_string();
    }
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest.join(relative);
    if candidate.exists() {
        return candidate.to_string_lossy().to_string();
    }
    panic!("Cannot find fixture");
}

fn read_fixture() -> SoftwareDefinition {
    let content = std::fs::read_to_string(fixture_path()).expect("Read fixture");
    serde_json::from_str(&content).expect("Parse S.DEF")
}

#[test]
fn test_json_serialization_roundtrip() {
    let original = read_fixture();
    let json = serde_json::to_string_pretty(&original).expect("Serialize");
    let restored: SoftwareDefinition = serde_json::from_str(&json).expect("Deserialize");
    assert_eq!(original.name, restored.name);
    assert_eq!(original.sdef_version, restored.sdef_version);
}

#[test]
fn test_import_into_db_and_query() {
    let sdef = read_fixture();
    let db = Database::in_memory().expect("Create DB");

    // Insert document and data models using the Database connection
    let conn = db.connection();
    conn.execute(
        "INSERT INTO sdef_documents (name, version, description) VALUES (?1, ?2, ?3)",
        rusqlite::params![sdef.name, sdef.version, sdef.description],
    ).expect("Insert document");

    if let Some(models) = &sdef.data_models {
        for model in models {
            conn.execute(
                "INSERT INTO data_models (entity, document_name, status) VALUES (?1, ?2, 'active')",
                rusqlite::params![model.entity, sdef.name],
            ).expect("Insert model");

            if let Some(attrs) = &model.attributes {
                for attr in attrs {
                    conn.execute(
                        "INSERT INTO data_attributes (document_name, entity, name, attr_type, required)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![sdef.name, model.entity, attr.name, attr.attr_type, attr.required],
                    ).expect("Insert attr");
                }
            }
        }
    }
    // Drop the MutexGuard before verifying
    drop(conn);

    // Verify document
    let conn = db.connection();
    let doc_name: String = conn.query_row(
        "SELECT name FROM sdef_documents LIMIT 1", [], |r| r.get(0),
    ).expect("Query document");
    assert_eq!(doc_name, sdef.name, "Document name should match");

    // Verify model count
    let model_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM data_models", [], |r| r.get(0),
    ).expect("Count models");
    let expected = sdef.data_models.as_ref().map(|v| v.len() as i64).unwrap_or(0);
    assert_eq!(model_count, expected, "Data model count mismatch");

    // Verify attribute count
    let attr_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM data_attributes", [], |r| r.get(0),
    ).expect("Count attributes");
    assert!(attr_count > 0, "Should have attributes");
}
