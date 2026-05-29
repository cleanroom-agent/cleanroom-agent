//! S.DEF File Exporter — writes S.DEF content to disk as a shard file tree.
//!
//! Produces the directory structure defined in the design document §3.1.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::instrument;

use rusqlite::params;
use serde_json::json;

use crate::error::{DbError, DbResult};
use crate::Database;

/// The file exporter writes S.DEF entities from DB to the standard shard tree.
pub struct SdefFileExporter {
    db: Database,
}

impl SdefFileExporter {
    /// Create a new file exporter.
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Export a complete document to the given output directory.
    #[instrument(skip(self))]
    pub fn export_to_disk(&self, document_name: &str, output_dir: &Path) -> DbResult<PathBuf> {
        let root = output_dir.join(document_name);
        fs::create_dir_all(&root)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create output dir: {}", e)))?;

        self.write_root_index(document_name, &root)?;
        self.write_metadata(document_name, &root)?;
        self.write_system_boundary(document_name, &root)?;
        self.write_architecture(document_name, &root)?;
        self.write_data_models(document_name, &root)?;
        self.write_contracts(document_name, &root)?;
        self.write_behavior(document_name, &root)?;
        self.write_ui(document_name, &root)?;
        self.write_tests(document_name, &root)?;
        self.write_design_decisions(document_name, &root)?;
        self.write_deployment(document_name, &root)?;
        self.write_sdef_state(document_name, &root)?;

        Ok(root)
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    fn write_json(path: &Path, value: &serde_json::Value) -> DbResult<()> {
        let json_str = serde_json::to_string_pretty(value)
            .map_err(|e| DbError::QueryFailed(format!("Serialization error: {}", e)))?;
        fs::write(path, &json_str)
            .map_err(|e| DbError::QueryFailed(format!("Failed to write {}: {}", path.display(), e)))
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
        self.db.connection()
    }

    // ── Root index ──────────────────────────────────────────────────────

    fn write_root_index(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let conn = self.conn();
        let (version, description): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT version, description FROM sdef_documents WHERE name = ?1",
                params![document_name],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((None, None));
        drop(conn);

        let index = json!({
            "sdef_version": "2026-05-27",
            "name": document_name,
            "version": version,
            "description": description,
            "sections": {
                "metadata": "metadata.sdef.json",
                "system_boundary": "system-boundary.sdef.json",
                "architecture": "architecture.sdef.json",
                "data_models": "data-models/index.sdef.json",
                "contracts": "contracts/index.sdef.json",
                "behavior": "behavior/index.sdef.json",
                "ui": "ui/index.sdef.json",
                "tests": "tests/index.sdef.json",
                "design_decisions": "design-decisions/index.sdef.json",
                "deployment": "deployment.sdef.json",
            },
            "exported_at": chrono::Utc::now().to_rfc3339(),
        });
        Self::write_json(&root.join("sdef-index.sdef.json"), &index)
    }

    // ── Metadata ────────────────────────────────────────────────────────

    fn write_metadata(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let meta = json!({
            "document_name": document_name,
            "sdef_version": "2026-05-27",
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        Self::write_json(&root.join("metadata.sdef.json"), &meta)
    }

    // ── System boundary ─────────────────────────────────────────────────

    fn write_system_boundary(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let conn = self.conn();
        let boundary: Result<(String, Option<String>), _> = conn.query_row(
            "SELECT core_purpose, data_json FROM system_boundaries WHERE document_name = ?1",
            params![document_name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );
        drop(conn);

        let content = match boundary {
            Ok((purpose, data)) => {
                let parsed = data
                    .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
                    .unwrap_or(json!({}));
                json!({
                    "document_name": document_name,
                    "core_purpose": purpose,
                    "data": parsed,
                })
            }
            Err(_) => json!({"document_name": document_name, "core_purpose": ""}),
        };
        Self::write_json(&root.join("system-boundary.sdef.json"), &content)
    }

    // ── Architecture ────────────────────────────────────────────────────

    fn write_architecture(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let conn = self.conn();
        let arch_style: String = conn
            .query_row(
                "SELECT COALESCE(style, '') FROM architecture_docs WHERE document_name = ?1",
                params![document_name],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_default();

        let mut layers_st = conn
            .prepare("SELECT layer_name, components_json FROM architecture_layers WHERE document_name = ?1")
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut layers = Vec::new();
        let layer_rows = layers_st
            .query_map(params![document_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for row in layer_rows {
            if let Ok((name, comps)) = row {
                let parsed = comps
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    .unwrap_or(json!([]));
                layers.push(json!({"name": name, "components": parsed}));
            }
        }
        drop(layers_st);
        drop(conn);

        let content = json!({
            "document_name": document_name,
            "style": if arch_style.is_empty() { None } else { Some(arch_style) },
            "layers": layers,
        });
        Self::write_json(&root.join("architecture.sdef.json"), &content)
    }

    // ── Data models ─────────────────────────────────────────────────────

    fn write_data_models(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let dir = root.join("data-models");
        fs::create_dir_all(&dir)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create data-models dir: {}", e)))?;

        let models = self.fetch_data_models(document_name)?;
        let mut index_entries = Vec::new();

        for (entity, status, version, description, logical_model) in &models {
            let attrs = self.fetch_data_attributes(document_name, entity)?;
            let rels = self.fetch_data_relationships(document_name, entity)?;

            let filename = format!("{}.sdef.json", entity.to_lowercase().replace(' ', "-"));
            let entity_content = json!({
                "entity": entity,
                "status": status,
                "version": version,
                "description": description,
                "logical_model": logical_model,
                "attributes": attrs,
                "relationships": rels,
            });
            Self::write_json(&dir.join(&filename), &entity_content)?;
            index_entries.push(json!({"entity": entity, "file": &filename}));
        }

        let idx = json!({"document_name": document_name, "entities": index_entries});
        Self::write_json(&dir.join("index.sdef.json"), &idx)
    }

    fn fetch_data_models(
        &self,
        document_name: &str,
    ) -> DbResult<Vec<(String, String, Option<String>, Option<String>, Option<String>)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT entity, status, version, description, logical_model FROM data_models WHERE document_name = ?1",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    fn fetch_data_attributes(
        &self,
        document_name: &str,
        entity: &str,
    ) -> DbResult<Vec<serde_json::Value>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT name, attr_type, format, description, required, identity, generated, unique_flag, internal, deprecated, default_value, constraints_json FROM data_attributes WHERE document_name = ?1 AND entity = ?2",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name, entity], |row| {
                let constraints: Option<serde_json::Value> = row
                    .get::<_, Option<String>>(11)?
                    .and_then(|s| serde_json::from_str(&s).ok());
                Ok(json!({
                    "name": row.get::<_, String>(0)?,
                    "type": row.get::<_, String>(1)?,
                    "format": row.get::<_, Option<String>>(2)?,
                    "description": row.get::<_, Option<String>>(3)?,
                    "required": row.get::<_, bool>(4)?,
                    "identity": row.get::<_, bool>(5)?,
                    "generated": row.get::<_, bool>(6)?,
                    "unique": row.get::<_, bool>(7)?,
                    "internal": row.get::<_, bool>(8)?,
                    "deprecated": row.get::<_, bool>(9)?,
                    "default": row.get::<_, Option<String>>(10)?,
                    "constraints": constraints,
                }))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    fn fetch_data_relationships(
        &self,
        document_name: &str,
        entity: &str,
    ) -> DbResult<Vec<serde_json::Value>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT kind, target, foreign_key, join_table, on_delete FROM data_relationships WHERE document_name = ?1 AND entity = ?2",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name, entity], |row| {
                Ok(json!({
                    "kind": row.get::<_, String>(0)?,
                    "target": row.get::<_, String>(1)?,
                    "foreign_key": row.get::<_, Option<String>>(2)?,
                    "join_table": row.get::<_, Option<String>>(3)?,
                    "on_delete": row.get::<_, Option<String>>(4)?,
                }))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    // ── Contracts ───────────────────────────────────────────────────────

    fn write_contracts(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let contracts_dir = root.join("contracts");
        fs::create_dir_all(&contracts_dir)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create contracts dir: {}", e)))?;

        let contracts = self.fetch_contracts(document_name)?;
        let mut index_by_type: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

        for (name, ctype, status, description, http_method) in &contracts {
            let methods = self.fetch_contract_methods(document_name, name)?;

            let sub_dir = match ctype.as_str() {
                "interface" => "interfaces",
                "class" => "classes",
                "enum" => "enums",
                "api" => "apis",
                _ => "other",
            };
            let type_dir = contracts_dir.join(sub_dir);
            fs::create_dir_all(&type_dir)
                .map_err(|e| DbError::QueryFailed(format!("Cannot create contracts/{} dir: {}", sub_dir, e)))?;

            let filename = format!("{}.sdef.json", name.to_lowercase().replace(' ', "-"));
            let entity_content = json!({
                "name": name,
                "contract_type": ctype,
                "status": status,
                "description": description,
                "http_method": http_method,
                "methods": methods,
            });
            Self::write_json(&type_dir.join(&filename), &entity_content)?;
            index_by_type
                .entry(ctype.clone())
                .or_default()
                .push(json!({"name": name, "file": &filename}));
        }

        let idx = json!({
            "document_name": document_name,
            "interfaces": index_by_type.get("interface").cloned().unwrap_or_default(),
            "classes": index_by_type.get("class").cloned().unwrap_or_default(),
            "enums": index_by_type.get("enum").cloned().unwrap_or_default(),
            "apis": index_by_type.get("api").cloned().unwrap_or_default(),
        });
        Self::write_json(&contracts_dir.join("index.sdef.json"), &idx)
    }

    fn fetch_contracts(
        &self,
        document_name: &str,
    ) -> DbResult<Vec<(String, String, String, String, Option<String>)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT name, contract_type, COALESCE(status,'active'), COALESCE(description,''), http_method FROM contracts WHERE document_name = ?1",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    fn fetch_contract_methods(
        &self,
        document_name: &str,
        contract_name: &str,
    ) -> DbResult<Vec<serde_json::Value>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT signature, COALESCE(status,'active'), behavior, preconditions_json, postconditions_json, errors_json FROM contract_methods WHERE document_name = ?1 AND contract_name = ?2",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name, contract_name], |row| {
                let pre: Option<serde_json::Value> = row
                    .get::<_, Option<String>>(3)?
                    .and_then(|s| serde_json::from_str(&s).ok());
                let post: Option<serde_json::Value> = row
                    .get::<_, Option<String>>(4)?
                    .and_then(|s| serde_json::from_str(&s).ok());
                let err: Option<serde_json::Value> = row
                    .get::<_, Option<String>>(5)?
                    .and_then(|s| serde_json::from_str(&s).ok());
                Ok(json!({
                    "signature": row.get::<_, String>(0)?,
                    "status": row.get::<_, String>(1)?,
                    "behavior": row.get::<_, Option<String>>(2)?,
                    "preconditions": pre,
                    "postconditions": post,
                    "errors": err,
                }))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    // ── Behavior ────────────────────────────────────────────────────────

    fn write_behavior(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let behavior_dir = root.join("behavior");
        fs::create_dir_all(&behavior_dir)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create behavior dir: {}", e)))?;

        let funcs_dir = behavior_dir.join("functions");
        fs::create_dir_all(&funcs_dir)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create behavior/functions dir: {}", e)))?;

        let functions = self.fetch_functions(document_name)?;
        let mut func_index = Vec::new();
        for (name, desc, logic, complexity, pure) in &functions {
            let filename = format!("{}.sdef.json", name.to_lowercase().replace(' ', "-"));
            let content = json!({
                "name": name,
                "description": desc,
                "logic": logic,
                "complexity": complexity,
                "pure_function": pure,
            });
            Self::write_json(&funcs_dir.join(&filename), &content)?;
            func_index.push(json!({"name": name, "file": &filename}));
        }

        let flows = self.fetch_flows(document_name)?;
        let flows_dir = behavior_dir.join("flows");
        fs::create_dir_all(&flows_dir)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create behavior/flows dir: {}", e)))?;
        let mut flow_index = Vec::new();
        for (name, desc, trigger) in &flows {
            let filename = format!("{}.sdef.json", name.to_lowercase().replace(' ', "-"));
            let content = json!({"name": name, "description": desc, "trigger": trigger});
            Self::write_json(&flows_dir.join(&filename), &content)?;
            flow_index.push(json!({"name": name, "file": &filename}));
        }

        let idx = json!({
            "document_name": document_name,
            "functions": func_index,
            "flows": flow_index,
        });
        Self::write_json(&behavior_dir.join("index.sdef.json"), &idx)
    }

    fn fetch_functions(
        &self,
        document_name: &str,
    ) -> DbResult<Vec<(String, Option<String>, Option<String>, Option<String>, bool)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT name, description, logic, complexity, pure_function FROM function_specs WHERE document_name = ?1",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, bool>(4)?,
                ))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    fn fetch_flows(
        &self,
        document_name: &str,
    ) -> DbResult<Vec<(String, Option<String>, Option<String>)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT name, description, trigger FROM flow_specs WHERE document_name = ?1",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    // ── UI ──────────────────────────────────────────────────────────────

    fn write_ui(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let ui_dir = root.join("ui");
        fs::create_dir_all(&ui_dir)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create ui dir: {}", e)))?;

        let conn = self.conn();
        let ui_doc: Result<(Option<String>, String), _> = conn.query_row(
            "SELECT pen_version, raw_content_json FROM ui_documents WHERE document_name = ?1",
            params![document_name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );
        drop(conn);

        if let Ok((pen_ver, raw)) = &ui_doc {
            let doc_content: serde_json::Value =
                serde_json::from_str(raw).unwrap_or(json!({"raw": raw}));
            let doc_file = json!({
                "document_name": document_name,
                "pen_version": pen_ver,
                "content": doc_content,
            });
            Self::write_json(&ui_dir.join("document.sdef.json"), &doc_file)?;
        }

        Self::write_json(
            &ui_dir.join("design-system.sdef.json"),
            &json!({"document_name": document_name}),
        )?;

        let screens_dir = ui_dir.join("screens");
        fs::create_dir_all(&screens_dir)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create ui/screens dir: {}", e)))?;

        let screens = self.fetch_ui_screens(document_name)?;
        let mut screen_index = Vec::new();
        for (sid, name, route, purpose, layout) in &screens {
            let filename = format!("{}.sdef.json", sid);
            let content = json!({
                "id": sid,
                "name": name,
                "route": route,
                "purpose": purpose,
                "layout_description": layout,
            });
            Self::write_json(&screens_dir.join(&filename), &content)?;
            screen_index.push(json!({"id": sid, "name": name, "file": &filename}));
        }

        let idx = json!({
            "document_name": document_name,
            "design_system": "design-system.sdef.json",
            "document": ui_doc.is_ok().then(|| "document.sdef.json"),
            "screens": screen_index,
        });
        Self::write_json(&ui_dir.join("index.sdef.json"), &idx)
    }

    fn fetch_ui_screens(
        &self,
        document_name: &str,
    ) -> DbResult<Vec<(String, String, Option<String>, Option<String>, Option<String>)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, route, purpose, layout_description FROM ui_screens WHERE document_name = ?1",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    // ── Tests ───────────────────────────────────────────────────────────

    fn write_tests(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let tests_dir = root.join("tests");
        fs::create_dir_all(&tests_dir)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create tests dir: {}", e)))?;

        let groups = self.fetch_test_groups(document_name)?;
        let mut unit_index = Vec::new();
        let mut integration_index = Vec::new();

        for (gid, module_id, interface_id) in &groups {
            let cases = self.fetch_test_cases(*gid)?;
            for (tc_id, desc, tc_type) in &cases {
                let filename = format!("{}.sdef.json", tc_id);
                let content = json!({
                    "id": tc_id,
                    "description": desc,
                    "test_type": tc_type,
                    "module_id": module_id,
                    "interface_id": interface_id,
                });

                match tc_type.as_str() {
                    "unit" => {
                        let unit_dir = tests_dir.join("unit-tests");
                        fs::create_dir_all(&unit_dir).map_err(|e| {
                            DbError::QueryFailed(format!("Cannot create unit-tests dir: {}", e))
                        })?;
                        Self::write_json(&unit_dir.join(&filename), &content)?;
                        unit_index.push(json!({"id": tc_id, "description": desc, "file": &filename}));
                    }
                    "integration" => {
                        let int_dir = tests_dir.join("integration-tests");
                        fs::create_dir_all(&int_dir).map_err(|e| {
                            DbError::QueryFailed(format!("Cannot create integration-tests dir: {}", e))
                        })?;
                        Self::write_json(&int_dir.join(&filename), &content)?;
                        integration_index.push(json!({"id": tc_id, "description": desc, "file": &filename}));
                    }
                    _ => {}
                }
            }
        }

        let idx = json!({
            "document_name": document_name,
            "unit_tests": unit_index,
            "integration_tests": integration_index,
        });
        Self::write_json(&tests_dir.join("index.sdef.json"), &idx)
    }

    fn fetch_test_groups(
        &self,
        document_name: &str,
    ) -> DbResult<Vec<(i64, Option<String>, Option<String>)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id, module_id, interface_id FROM test_groups WHERE document_name = ?1")
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    fn fetch_test_cases(
        &self,
        group_id: i64,
    ) -> DbResult<Vec<(String, String, String)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id, description, test_type FROM test_cases WHERE group_id = ?1")
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![group_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    // ── Design decisions ────────────────────────────────────────────────

    fn write_design_decisions(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let dd_dir = root.join("design-decisions");
        fs::create_dir_all(&dd_dir)
            .map_err(|e| DbError::QueryFailed(format!("Cannot create design-decisions dir: {}", e)))?;

        let decisions = self.fetch_design_decisions(document_name)?;
        let mut index_entries = Vec::new();

        for (dd_id, topic, decision, rationale, context, alternatives) in &decisions {
            let filename = format!("dd-{}.sdef.json", dd_id);
            let content = json!({
                "id": dd_id,
                "topic": topic,
                "decision": decision,
                "rationale": rationale,
                "context": context,
                "alternatives": alternatives.as_ref().and_then(|s| {
                    serde_json::from_str::<serde_json::Value>(s).ok()
                }),
            });
            Self::write_json(&dd_dir.join(&filename), &content)?;
            index_entries.push(json!({"id": dd_id, "topic": topic, "file": &filename}));
        }

        let idx = json!({"document_name": document_name, "decisions": index_entries});
        Self::write_json(&dd_dir.join("index.sdef.json"), &idx)
    }

    fn fetch_design_decisions(
        &self,
        document_name: &str,
    ) -> DbResult<Vec<(String, String, String, Option<String>, Option<String>, Option<String>)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, topic, decision, rationale, context, alternatives_json FROM design_decisions WHERE document_name = ?1",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    // ── Deployment ──────────────────────────────────────────────────────

    fn write_deployment(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let conn = self.conn();
        let deployment: Option<(Option<String>, Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT deployment_type, provider, region FROM deployment_configs WHERE document_name = ?1",
                params![document_name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();
        drop(conn);

        let deps = self.fetch_dependencies(document_name)?;
        let content = json!({
            "document_name": document_name,
            "deployment": deployment,
            "dependencies": deps,
        });
        Self::write_json(&root.join("deployment.sdef.json"), &content)
    }

    fn fetch_dependencies(&self, document_name: &str) -> DbResult<Vec<serde_json::Value>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT dep_name, dep_version, dep_type, source_url FROM dependencies WHERE document_name = ?1",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![document_name], |row| {
                Ok(json!({
                    "name": row.get::<_, String>(0)?,
                    "version": row.get::<_, Option<String>>(1)?,
                    "type": row.get::<_, String>(2)?,
                    "source_url": row.get::<_, Option<String>>(3)?,
                }))
            })
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        for r in rows {
            if let Ok(v) = r {
                results.push(v);
            }
        }
        drop(stmt);
        drop(conn);
        Ok(results)
    }

    // ── .sdef-state directory ───────────────────────────────────────────

    fn write_sdef_state(&self, document_name: &str, root: &Path) -> DbResult<()> {
        let state_dir = root.join(".sdef-state");
        fs::create_dir_all(&state_dir.join("checkpoints"))
            .map_err(|e| DbError::QueryFailed(format!("Cannot create .sdef-state: {}", e)))?;

        // Copy state.db into .sdef-state/
        let candidates = [
            Path::new("state.db").to_path_buf(),
            root.parent().and_then(|p| Some(p.join("state.db"))).unwrap_or_default(),
        ];
        for candidate in &candidates {
            if candidate.exists() {
                let target = state_dir.join("state.db");
                let _ = fs::copy(candidate, &target);
                break;
            }
        }

        // Export checkpoints as JSON files
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT checkpoint_id, description, created_at FROM checkpoints WHERE document_name = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let checkpoints: Vec<(String, Option<String>, String)> = {
            let mut results = Vec::new();
            let rows = stmt
                .query_map(params![document_name], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;
            for r in rows {
                if let Ok(v) = r {
                    results.push(v);
                }
            }
            results
        };
        drop(stmt);
        drop(conn);

        for (checkpoint_id, description, created_at) in &checkpoints {
            let cp_file = state_dir
                .join("checkpoints")
                .join(format!("{}.json", checkpoint_id));
            let content = json!({
                "checkpoint_id": checkpoint_id,
                "description": description,
                "created_at": created_at,
            });
            let json_str = serde_json::to_string_pretty(&content)
                .map_err(|e| DbError::QueryFailed(format!("Serialization error: {}", e)))?;
            fs::write(&cp_file, &json_str)
                .map_err(|e| DbError::QueryFailed(format!("Failed to write checkpoint: {}", e)))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_db(tmp_dir: &Path) -> Database {
        let db_path = tmp_dir.join("test.db");
        let db = Database::open_embedded(&db_path).unwrap();
        let conn = db.connection();

        conn.execute_batch(
            "INSERT INTO sdef_documents (name, version, description, created_at, updated_at)
             VALUES ('test-proj', '1.0', 'Test', datetime(), datetime());

             INSERT INTO data_models (entity, document_name, status, description)
             VALUES ('User', 'test-proj', 'active', 'A user entity');

             INSERT INTO data_attributes (document_name, entity, name, attr_type, required)
             VALUES ('test-proj', 'User', 'id', 'UUID', 1);

             INSERT INTO data_attributes (document_name, entity, name, attr_type)
             VALUES ('test-proj', 'User', 'name', 'string');

             INSERT INTO contracts (name, document_name, contract_type, status, description)
             VALUES ('UserService', 'test-proj', 'interface', 'active', 'User service');

             INSERT INTO contract_methods (document_name, contract_name, signature, behavior)
             VALUES ('test-proj', 'UserService', 'get_user(id: UUID) -> User', 'Fetch user');

             INSERT INTO function_specs (document_name, name, description, pure_function)
             VALUES ('test-proj', 'validate_email', 'Validate email format', 1);

             INSERT INTO design_decisions (id, document_name, topic, decision, rationale)
             VALUES ('dd-001', 'test-proj', 'Use PostgreSQL', 'PostgreSQL chosen', 'ACID');",
        )
        .unwrap();

        drop(conn);
        db
    }

    #[test]
    fn test_export_full_tree() {
        let tmp = TempDir::new().unwrap();
        let db = setup_test_db(tmp.path());
        let exporter = SdefFileExporter::new(db);
        let out_dir = tmp.path().join("output");

        let root = exporter.export_to_disk("test-proj", &out_dir).unwrap();

        assert!(root.join("sdef-index.sdef.json").exists(), "Root index missing");
        assert!(root.join("metadata.sdef.json").exists(), "Metadata missing");
        assert!(root.join("data-models").join("index.sdef.json").exists(), "DM index missing");
        assert!(root.join("data-models").join("user.sdef.json").exists(), "DM entity missing");
        assert!(root.join("contracts").join("index.sdef.json").exists(), "Ct index missing");
        assert!(
            root.join("contracts").join("interfaces").join("userservice.sdef.json").exists(),
            "Interface missing"
        );
        assert!(root.join("behavior").join("index.sdef.json").exists(), "Bh index missing");
        assert!(
            root.join("behavior").join("functions").join("validate_email.sdef.json").exists(),
            "Function missing"
        );
        assert!(root.join("design-decisions").join("index.sdef.json").exists(), "DD index missing");
        assert!(
            root.join("design-decisions").join("dd-dd-001.sdef.json").exists(),
            "DD entity missing"
        );
        assert!(root.join(".sdef-state").exists(), ".sdef-state missing");
        assert!(root.join(".sdef-state").join("checkpoints").exists(), "checkpoints dir missing");
        assert!(root.join("deployment.sdef.json").exists(), "deployment missing");

        let idx_content = fs::read_to_string(root.join("sdef-index.sdef.json")).unwrap();
        let idx: serde_json::Value = serde_json::from_str(&idx_content).unwrap();
        assert_eq!(idx["name"], "test-proj");
        assert!(idx["sections"].is_object());
    }
}
