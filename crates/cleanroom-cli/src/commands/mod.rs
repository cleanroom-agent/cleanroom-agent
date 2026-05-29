//! CLI commands — all user-facing messages use i18n via `tr_global!()`.
//!
//! Uses `CleanroomAgent` (with adk-rust integration) as the top-level entry point.

use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, Context};
use clap::Subcommand;
use cleanroom_agent::{
    AgentConfig, CleanroomAgent, RunMode,
    CompatibilityMode, Fidelity, CompatibilityResolver, CompletenessValidator, format_report,
};
use cleanroom_db::Database;
use cleanroom_i18n::tr_global;

#[derive(Subcommand)]
pub enum Commands {
    /// Production mode: analyze code repository → output S.DEF
    Produce {
        #[arg(long)]
        repo: String,
        #[arg(long, default_value = "./sdef-output")]
        output: String,
        #[arg(long)]
        exclude: Option<String>,
        #[arg(long)]
        name: Option<String>,
        /// LLM model (e.g. gemini-2.5-flash)
        #[arg(long)]
        model: Option<String>,
        /// API key for LLM provider
        #[arg(long)]
        api_key: Option<String>,
    },

    /// Consumption mode: read S.DEF → generate code
    Consume {
        #[arg(long)]
        sdef: String,
        #[arg(long, default_value = "./output")]
        output: String,
        #[arg(long)]
        language: String,
        #[arg(long)]
        framework: Option<String>,
        #[arg(long, default_value = "mixed")]
        compat_mode: String,
        #[arg(long, default_value = "medium")]
        fidelity: String,
        /// LLM model (e.g. gemini-2.5-flash)
        #[arg(long)]
        model: Option<String>,
        /// API key for LLM provider
        #[arg(long)]
        api_key: Option<String>,
    },

    /// MCP server mode
    Serve {
        #[arg(long, default_value = "stdio")]
        transport: String,
    },

    /// Resume workflow from checkpoint
    Resume {
        #[arg(long)]
        document: String,
        /// Resume failed tasks too
        #[arg(long)]
        retry_failed: bool,
    },

    /// Inspect database/S.DEF state
    Inspect {
        #[arg(long, default_value = "consistency")]
        check_type: String,
    },

    /// Export document
    Export {
        #[arg(long)]
        document: String,
        #[arg(long, default_value = "./sdef-output/sdef.json")]
        output: String,
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Import document
    Import {
        #[arg(long)]
        file: String,
    },

    /// Database migration
    Migrate {
        #[arg(long, default_value = "up")]
        direction: String,
    },
}

pub fn run(command: Commands, db_path: &str) -> Result<()> {
    match command {
        Commands::Produce { repo, output, exclude: _, name, model, api_key } => {
            produce_command(&repo, &output, db_path, name, model, api_key)
        }
        Commands::Consume { sdef, output, language, framework, compat_mode, fidelity, model, api_key } => {
            consume_command(&sdef, &output, &language, framework.as_deref(), &compat_mode, &fidelity, db_path, model, api_key)
        }
        Commands::Serve { transport } => {
            serve_command(&transport, db_path)
        }
        Commands::Resume { document, retry_failed } => {
            resume_command(&document, retry_failed, db_path)
        }
        Commands::Inspect { check_type } => {
            inspect_command(&check_type, db_path)
        }
        Commands::Export { document, output, format } => {
            export_command(&document, &output, &format, db_path)
        }
        Commands::Import { file } => {
            import_command(&file, db_path)
        }
        Commands::Migrate { direction } => {
            migrate_command(&direction, db_path)
        }
    }
}

fn set_api_key(key: Option<String>) {
    if let Some(k) = key {
        if std::env::var("GOOGLE_API_KEY").is_err() {
            std::env::set_var("GOOGLE_API_KEY", k);
        }
    }
}

fn produce_command(
    repo: &str, output: &str, db_path: &str,
    name: Option<String>, model: Option<String>, api_key: Option<String>,
) -> Result<()> {
    set_api_key(api_key);
    use tokio::runtime::Runtime;
    let project_name = name.unwrap_or_else(|| {
        Path::new(repo).file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string())
    });

    let rt = Runtime::new().context(tr_global!("cli.error_runtime"))?;
    rt.block_on(async {
        let agent_config = AgentConfig {
            db_path: Path::new(db_path).to_path_buf(),
            model_name: model,
            agent_name: "cleanroom-producer".to_string(),
            ..AgentConfig::default()
        };
        let agent = CleanroomAgent::new(agent_config)
            .context(tr_global!("cli.error_orchestrator"))?;

        let pn = project_name.clone();
        agent.run(RunMode::Produce {
            repo_path: Path::new(repo).to_path_buf(),
            output_path: Path::new(output).to_path_buf(),
            project_name,
        }).await?;

        println!("{}", tr_global!("cli.produce_complete", pn));
        Ok(())
    })
}

fn consume_command(
    sdef: &str, output: &str, language: &str, framework: Option<&str>,
    compat_mode: &str, fidelity: &str, db_path: &str,
    model: Option<String>, api_key: Option<String>,
) -> Result<()> {
    set_api_key(api_key);
    println!("{}", tr_global!("cli.consume_step1", sdef));

    let cm = match compat_mode {
        "full" => CompatibilityMode::Full,
        "mixed" => CompatibilityMode::Mixed,
        "clean" => CompatibilityMode::Clean,
        _ => CompatibilityMode::Mixed,
    };
    let fid = match fidelity {
        "high" => Fidelity::High,
        "low" => Fidelity::Low,
        _ => Fidelity::Medium,
    };

    let rt = tokio::runtime::Runtime::new().context(tr_global!("cli.error_runtime"))?;
    rt.block_on(async {
        let agent_config = AgentConfig {
            db_path: Path::new(db_path).to_path_buf(),
            model_name: model,
            agent_name: "cleanroom-consumer".to_string(),
            ..AgentConfig::default()
        };
        let agent = CleanroomAgent::new(agent_config)
            .context(tr_global!("cli.error_orchestrator"))?;

        agent.run(RunMode::Consume {
            sdef_path: Path::new(sdef).to_path_buf(),
            output_path: Path::new(output).to_path_buf(),
            language: language.to_string(),
            framework: framework.map(|s| s.to_string()),
            compat_mode: cm,
            fidelity: fid,
        }).await?;

        // Run completeness validation
        let validator = CompletenessValidator::new(agent.db().clone());
        match validator.validate("") {
            Ok(report) => println!("{}", format_report(&report)),
            Err(_) => {}
        }
        Ok(())
    })
}

fn serve_command(transport: &str, db_path: &str) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context(tr_global!("cli.error_runtime"))?;
    rt.block_on(async {
        let server = cleanroom_mcp::CleanroomMcpServer::new(Path::new(db_path))
            .context(tr_global!("cli.error_mcp_server"))?;
        println!("{}", tr_global!("cli.serve_starting", transport));
        server.serve().await?;
        Ok(())
    })
}

fn resume_command(document: &str, retry_failed: bool, db_path: &str) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context(tr_global!("cli.error_runtime"))?;
    rt.block_on(async {
        let agent_config = AgentConfig {
            db_path: Path::new(db_path).to_path_buf(),
            ..AgentConfig::default()
        };
        let agent = CleanroomAgent::new(agent_config)
            .context(tr_global!("cli.error_runtime"))?;

        agent.run(RunMode::Resume {
            document: document.to_string(),
            retry_failed,
        }).await?;
        Ok(())
    })
}

fn inspect_command(check_type: &str, db_path: &str) -> Result<()> {
    let db = match Database::open(Path::new(db_path)) {
        Ok(db) => db,
        Err(_) => {
            println!("{}", tr_global!("cli.inspect_no_db", db_path));
            return Ok(());
        }
    };

    println!("{}", tr_global!("cli.inspect_header"));
    println!("{}", tr_global!("cli.inspect_db", db_path));

    match check_type {
        "consistency" => {
            let conn = db.connection();
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM fingerprints WHERE sdef_hash != db_hash OR db_hash != code_hash"
            ).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let inconsistent: i64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
            println!("{}", tr_global!("cli.inspect_inconsistent", inconsistent));

            let mut stmt = conn.prepare("SELECT COUNT(*) FROM fingerprints")
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let total: i64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
            println!("{}", tr_global!("cli.inspect_total_fp", total));
            if total > 0 {
                let pct = 100.0 * (total - inconsistent) as f64 / total as f64;
                println!("{}", tr_global!("cli.inspect_consistency", pct));
            }
        }
        "coverage" => {
            let conn = db.connection();
            let models: i64 = conn.query_row("SELECT COUNT(*) FROM data_models", [], |r| r.get(0)).unwrap_or(0);
            let attrs: i64 = conn.query_row("SELECT COUNT(*) FROM data_attributes", [], |r| r.get(0)).unwrap_or(0);
            let contracts: i64 = conn.query_row("SELECT COUNT(*) FROM contracts", [], |r| r.get(0)).unwrap_or(0);
            let functions: i64 = conn.query_row("SELECT COUNT(*) FROM function_specs", [], |r| r.get(0)).unwrap_or(0);
            let symbols: i64 = conn.query_row("SELECT COUNT(*) FROM symbol_registry", [], |r| r.get(0)).unwrap_or(0);

            println!("{}", tr_global!("cli.inspect_coverage"));
            println!("{}", tr_global!("cli.inspect_data_models", models));
            println!("{}", tr_global!("cli.inspect_attributes", attrs));
            println!("{}", tr_global!("cli.inspect_contracts", contracts));
            println!("{}", tr_global!("cli.inspect_functions", functions));
            println!("{}", tr_global!("cli.inspect_symbols", symbols));
        }
        "progress" => {
            let conn = db.connection();
            let mut stmt = conn.prepare(
                "SELECT status, COUNT(*) FROM tasks GROUP BY status ORDER BY status"
            ).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            }).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            println!("{}", tr_global!("cli.inspect_task_progress"));
            let mut total = 0i64;
            let mut results = Vec::new();
            for row in rows.flatten() {
                results.push(row);
                total += results.last().unwrap().1;
            }
            for (status, count) in &results {
                let pct = if total > 0 { 100.0 * *count as f64 / total as f64 } else { 0.0 };
                println!("{}", tr_global!("cli.inspect_task_line", status, count, pct));
            }
        }
        _ => {
            println!("{}", tr_global!("cli.inspect_unknown_check", check_type));
        }
    }
    Ok(())
}

fn export_command(document: &str, output: &str, format: &str, db_path: &str) -> Result<()> {
    use std::io::Write;

    let db = Database::open(Path::new(db_path))?;
    let conn = db.connection();

    let mut stmt = conn.prepare(
        "SELECT name, version, description FROM sdef_documents WHERE name = ?1"
    ).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let (name, version, description): (String, Option<String>, Option<String>) = stmt.query_row(
        rusqlite::params![document],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    ).map_err(|_e| anyhow::anyhow!(tr_global!("cli.error_no_doc_in_db")))?;

    drop(stmt);

    let sdef = build_export_sdef(&conn, &name, version, description)?;

    let output_dir = Path::new(output).parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(output_dir)
        .context(tr_global!("cli.error_output_dir"))?;

    match format {
        "yaml" | "yml" => {
            let yaml = serde_yaml::to_string(&sdef)
                .context(tr_global!("cli.error_serialize_yaml"))?;
            let mut file = std::fs::File::create(output)
                .context(tr_global!("cli.error_output_file"))?;
            file.write_all(yaml.as_bytes())?;
        }
        _ => {
            let json = serde_json::to_string_pretty(&sdef)
                .context(tr_global!("cli.error_serialize_json"))?;
            let mut file = std::fs::File::create(output)
                .context(tr_global!("cli.error_output_file"))?;
            file.write_all(json.as_bytes())?;
        }
    }

    println!("{}", tr_global!("cli.export_success", document, output));
    Ok(())
}

fn build_export_sdef(
    conn: &rusqlite::Connection,
    name: &str,
    version: Option<String>,
    description: Option<String>,
) -> Result<sdef_core::SoftwareDefinition> {
    let mut sdef = sdef_core::SoftwareDefinition::default();
    sdef.sdef_version = sdef_core::CURRENT_SCHEMA_VERSION.to_string();
    sdef.name = name.to_string();
    sdef.version = version;
    sdef.description = description;

    let mut stmt = conn.prepare(
        "SELECT entity, status, version, description, logical_model FROM data_models WHERE document_name = ?1"
    ).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut rows = stmt.query(rusqlite::params![name])
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut models = Vec::new();
    while let Some(row) = rows.next().map_err(|e| anyhow::anyhow!(e.to_string()))? {
        let entity: String = row.get(0).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let status: Option<String> = row.get(1).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let version: Option<String> = row.get(2).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let description: Option<String> = row.get(3).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut attr_stmt = conn.prepare(
            "SELECT name, attr_type, format, description, required, identity, generated, unique_flag, internal, deprecated
             FROM data_attributes WHERE document_name = ?1 AND entity = ?2"
        ).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut attr_rows = attr_stmt.query(rusqlite::params![name, &entity])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut attrs = Vec::new();
        while let Some(ar) = attr_rows.next().map_err(|e| anyhow::anyhow!(e.to_string()))? {
            attrs.push(sdef_core::DataAttribute {
                name: ar.get(0).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                attr_type: ar.get(1).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                format: ar.get(2).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                description: ar.get(3).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                required: ar.get(4).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                default: None,
                identity: ar.get(5).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                generated: ar.get(6).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                unique: ar.get(7).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                internal: ar.get(8).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                deprecated: ar.get(9).map_err(|e| anyhow::anyhow!(e.to_string()))?,
                compatibility: None,
                constraints: None,
            });
        }
        drop(attr_rows);
        drop(attr_stmt);

        models.push(sdef_core::DataModel {
            entity,
            status,
            version,
            deprecated: None,
            description,
            logical_model: None,
            attributes: if attrs.is_empty() { None } else { Some(attrs) },
            relationships: None,
            validation_rules: None,
            physical_design: None,
        });
    }
    drop(rows);
    drop(stmt);

    if !models.is_empty() {
        sdef.data_models = Some(models);
    }

    let count = sdef.data_models.as_ref().map(|v| v.len()).unwrap_or(0);
    println!("{}", tr_global!("cli.export_data_models", count));
    Ok(sdef)
}

fn import_sdef_file(file: &str, db_path: &str) -> Result<String> {
    let content = std::fs::read_to_string(file)
        .context(tr_global!("cli.import_fail_read"))?;

    let sdef: sdef_core::SoftwareDefinition = if file.ends_with(".yaml") || file.ends_with(".yml") {
        serde_yaml::from_str(&content)
            .context(tr_global!("cli.import_fail_parse_yaml"))?
    } else {
        serde_json::from_str(&content)
            .context(tr_global!("cli.import_fail_parse_json"))?
    };

    let db = Database::open(Path::new(db_path))?;
    let conn = db.connection();

    // Use the export_import importer for full data model + contract import
    let importer = cleanroom_db::export_import::SdefImporter::new(
        rusqlite::Connection::open(db_path)?,
    );
    importer.import(&sdef)?;

    let model_count = sdef.data_models.as_ref().map(|v| v.len()).unwrap_or(0);
    println!("{}", tr_global!("cli.import_success", sdef.name, model_count));
    Ok(sdef.name)
}

fn import_command(file: &str, db_path: &str) -> Result<()> {
    import_sdef_file(file, db_path)?;
    Ok(())
}

fn migrate_command(direction: &str, db_path: &str) -> Result<()> {
    match direction {
        "up" => {
            let _db = Database::open(Path::new(db_path))?;
            println!("{}", tr_global!("cli.migrate_success"));
        }
        "down" => {
            println!("{}", tr_global!("cli.migrate_down_unsupported"));
        }
        _ => {
            println!("{}", tr_global!("cli.migrate_unknown_direction", direction));
        }
    }
    Ok(())
}
