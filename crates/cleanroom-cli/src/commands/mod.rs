//! CLI commands.

use std::path::Path;
use anyhow::{Result, Context};
use clap::Subcommand;
use cleanroom_agent::{Orchestrator, OrchestratorConfig, ProducerAgent, ProducerConfig, ConsumerAgent, ConsumerConfig, CompatibilityMode, Fidelity, PipelineResult};
use cleanroom_db::{Database, TaskRepository, TaskStatus, TaskType};

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
    },

    /// MCP server mode
    Serve {
        #[arg(long, default_value = "stdio")]
        transport: String,
    },

    /// Resume workflow from checkpoint
    Resume {
        /// Document name to resume
        #[arg(long)]
        document: String,
        /// Resume failed tasks too
        #[arg(long, default_value = "false")]
        retry_failed: bool,
    },

    /// Inspect database/S.DEF state
    Inspect {
        #[arg(long, default_value = "consistency")]
        check_type: String,
    },

    /// Export S.DEF document from database to JSON file
    Export {
        /// Document name to export
        #[arg(long)]
        document: String,
        /// Output JSON file path
        #[arg(long, default_value = "./sdef-output/sdef.json")]
        output: String,
        /// Output format: json (default) or yaml
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Import S.DEF from JSON file into database
    Import {
        /// S.DEF JSON file to import
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
        Commands::Produce { repo, output, exclude: _, name } => {
            produce_command(&repo, &output, db_path, name)
        }
        Commands::Consume { sdef, output, language, framework, compat_mode, fidelity } => {
            consume_command(&sdef, &output, &language, framework.as_deref(), &compat_mode, &fidelity, db_path)
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

fn produce_command(repo: &str, output: &str, db_path: &str, name: Option<String>) -> Result<()> {
    use tokio::runtime::Runtime;
    let project_name = name.unwrap_or_else(|| {
        Path::new(repo).file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string())
    });

    let rt = Runtime::new().context("Failed to create Tokio runtime")?;
    rt.block_on(async {
        let config = OrchestratorConfig {
            repo_path: Path::new(repo).to_path_buf(),
            output_path: Path::new(output).to_path_buf(),
            db_path: Path::new(db_path).to_path_buf(),
            checkpoint_interval_secs: 600,
            agent_idle_timeout_secs: 300,
        };
        let orchestrator = Orchestrator::new(config).context("Failed to create orchestrator")?;
        orchestrator.start_workflow().await?;

        let producer = ProducerAgent::new(ProducerConfig::default(), orchestrator.db().clone());
        while let Ok(Some(task)) = producer.process_next_task().await {
            println!("Processed task: {}", task.task_id);
        }
        println!("Production completed for '{}'", project_name);
        Ok(())
    })
}

fn consume_command(sdef: &str, output: &str, language: &str, framework: Option<&str>, compat_mode: &str, fidelity: &str, db_path: &str) -> Result<()> {
    let compat = match compat_mode {
        "full" => CompatibilityMode::Full,
        "mixed" => CompatibilityMode::Mixed,
        "clean" => CompatibilityMode::Clean,
        "custom" => CompatibilityMode::Custom,
        _ => CompatibilityMode::Mixed,
    };
    let fid = match fidelity {
        "high" => Fidelity::High,
        "medium" => Fidelity::Medium,
        "low" => Fidelity::Low,
        _ => Fidelity::Medium,
    };
    let config = ConsumerConfig {
        language: language.to_string(),
        framework: framework.map(String::from),
        compatibility_mode: compat,
        fidelity: fid,
        output_path: Path::new(output).to_path_buf(),
    };
    let db = Database::open(Path::new(db_path))?;
    let consumer = ConsumerAgent::new(config, std::sync::Arc::new(db));
    println!("Consume: sdef={}, output={}, language={}", sdef, output, language);
    println!("Consumer agent created: {}", consumer.agent_id());
    Ok(())
}

fn serve_command(transport: &str, db_path: &str) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
    rt.block_on(async {
        let server = cleanroom_mcp::CleanroomMcpServer::new(Path::new(db_path))
            .context("Failed to create MCP server")?;
        println!("MCP server starting with {} transport...", transport);
        server.serve().await?;
        Ok(())
    })
}

fn resume_command(document: &str, retry_failed: bool, db_path: &str) -> Result<()> {
    let db = Database::open(Path::new(db_path))?;
    let repo = TaskRepository::new(db.connection_arc());

    // Find all tasks for this document
    let all_tasks = repo.list(None, None, None).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // Filter by document name in input_json
    let doc_tasks: Vec<_> = all_tasks.iter().filter(|t| {
        t.input_json.contains(document)
    }).collect();

    if doc_tasks.is_empty() {
        println!("No tasks found for document '{}'", document);
        println!("Try: inspect document to see available documents");
        return Ok(());
    }

    // Separate completed, pending, and failed tasks
    let pending: Vec<_> = doc_tasks.iter().filter(|t| t.status == TaskStatus::Pending).collect();
    let in_progress: Vec<_> = doc_tasks.iter().filter(|t| matches!(t.status, TaskStatus::InProgress | TaskStatus::Assigned)).collect();
    let failed: Vec<_> = doc_tasks.iter().filter(|t| t.status == TaskStatus::Failed).collect();
    let completed: Vec<_> = doc_tasks.iter().filter(|t| t.status == TaskStatus::Completed).collect();

    println!("=== Workflow Summary for '{}' ===", document);
    println!("  Total tasks:      {}", doc_tasks.len());
    println!("  Completed:        {}", completed.len());
    println!("  In progress:      {}", in_progress.len());
    println!("  Pending:          {}", pending.len());
    println!("  Failed:           {}", failed.len());

    // Reset in_progress tasks back to pending
    for task in &in_progress {
        repo.update_status(&task.task_id, TaskStatus::Pending)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        println!("  Reset '{}' to pending", task.task_id);
    }

    // Optionally reset failed tasks
    if retry_failed {
        for task in &failed {
            repo.update_status(&task.task_id, TaskStatus::Pending)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("  Retrying '{}'", task.task_id);
        }
    }

    println!("\nReady to resume. Run `cleanroom produce` to continue processing.");
    Ok(())
}

fn inspect_command(check_type: &str, db_path: &str) -> Result<()> {
    let db = Database::open(Path::new(db_path))?;
    println!("=== Cleanroom Inspector ===");
    println!("Database: {}", db_path);

    match check_type {
        "consistency" => {
            // Check for inconsistent fingerprints
            let conn = db.connection();
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM fingerprints WHERE sdef_hash != db_hash OR db_hash != code_hash"
            ).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let inconsistent: i64 = stmt.query_row([], |row| row.get(0))
                .unwrap_or(0);
            println!("Inconsistent fingerprints: {}", inconsistent);

            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM fingerprints"
            ).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let total: i64 = stmt.query_row([], |row| row.get(0))
                .unwrap_or(0);
            println!("Total fingerprints: {}", total);
            if total > 0 {
                let pct = 100.0 * (total - inconsistent) as f64 / total as f64;
                println!("Consistency: {:.1}%", pct);
            }
        }
        "coverage" => {
            // Count data models and attributes
            let conn = db.connection();
            let models: i64 = conn.query_row("SELECT COUNT(*) FROM data_models", [], |r| r.get(0)).unwrap_or(0);
            let attrs: i64 = conn.query_row("SELECT COUNT(*) FROM data_attributes", [], |r| r.get(0)).unwrap_or(0);
            let contracts: i64 = conn.query_row("SELECT COUNT(*) FROM contracts", [], |r| r.get(0)).unwrap_or(0);
            let functions: i64 = conn.query_row("SELECT COUNT(*) FROM function_specs", [], |r| r.get(0)).unwrap_or(0);
            let symbols: i64 = conn.query_row("SELECT COUNT(*) FROM symbol_registry", [], |r| r.get(0)).unwrap_or(0);

            println!("S.DEF coverage:");
            println!("  Data models:    {}", models);
            println!("  Attributes:     {}", attrs);
            println!("  Contracts:      {}", contracts);
            println!("  Functions:      {}", functions);
            println!("  Symbols:        {}", symbols);
        }
        "progress" => {
            let conn = db.connection();
            let mut stmt = conn.prepare(
                "SELECT status, COUNT(*) FROM tasks GROUP BY status ORDER BY status"
            ).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            }).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            println!("Task progress:");
            let mut total = 0i64;
            let mut results = Vec::new();
            for row in rows.flatten() {
                results.push(row);
                total += results.last().unwrap().1;
            }
            for (status, count) in &results {
                let pct = if total > 0 { 100.0 * *count as f64 / total as f64 } else { 0.0 };
                println!("  {:<20}: {:>4} ({:.1}%)", status, count, pct);
            }
        }
        _ => {
            println!("Unknown check type: {}", check_type);
        }
    }
    Ok(())
}

fn export_command(document: &str, output: &str, format: &str, db_path: &str) -> Result<()> {
    use std::io::Write;

    let db = Database::open(Path::new(db_path))?;
    let conn = db.connection();

    // Query the SoftwareDefinition from the database
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
    ).map_err(|e| anyhow::anyhow!("Document '{}' not found: {}", document, e))?;

    drop(stmt);

    // Build the SoftwareDefinition  
    let sdef = build_export_sdef(&conn, &name, version, description)?;

    // Write output
    let output_dir = Path::new(output).parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(output_dir)
        .context("Failed to create output directory")?;

    match format {
        "yaml" | "yml" => {
            let yaml = serde_yaml::to_string(&sdef)
                .context("Failed to serialize YAML")?;
            let mut file = std::fs::File::create(output)
                .context("Failed to create output file")?;
            file.write_all(yaml.as_bytes())?;
        }
        _ => {
            let json = serde_json::to_string_pretty(&sdef)
                .context("Failed to serialize JSON")?;
            let mut file = std::fs::File::create(output)
                .context("Failed to create output file")?;
            file.write_all(json.as_bytes())?;
        }
    }

    println!("Exported document '{}' to {}", document, output);
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

    // Export data models
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

        // Export attributes
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

    println!("Exported {} data models", sdef.data_models.as_ref().map(|v| v.len()).unwrap_or(0));
    Ok(sdef)
}

fn import_command(file: &str, db_path: &str) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .context(format!("Failed to read file: {}", file))?;

    // Determine format from file extension
    let sdef: sdef_core::SoftwareDefinition = if file.ends_with(".yaml") || file.ends_with(".yml") {
        serde_yaml::from_str(&content)
            .context("Failed to parse YAML S.DEF file")?
    } else {
        serde_json::from_str(&content)
            .context("Failed to parse JSON S.DEF file")?
    };

    let db = Database::open(Path::new(db_path))?;
    let conn = db.connection();

    // Insert document
    conn.execute(
        "INSERT OR IGNORE INTO sdef_documents (name, version, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, datetime(), datetime())",
        rusqlite::params![sdef.name, sdef.version, sdef.description],
    ).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // Import data models
    if let Some(models) = &sdef.data_models {
        for model in models {
            conn.execute(
                "INSERT OR IGNORE INTO data_models (entity, document_name, status, version, description)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    model.entity, sdef.name,
                    model.status.clone().unwrap_or_else(|| "active".to_string()),
                    model.version, model.description,
                ],
            ).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            if let Some(attrs) = &model.attributes {
                for attr in attrs {
                    conn.execute(
                        "INSERT INTO data_attributes (document_name, entity, name, attr_type, format, description,
                         required, identity, generated, unique_flag, internal, deprecated)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        rusqlite::params![
                            sdef.name, model.entity, attr.name, attr.attr_type, attr.format,
                            attr.description, attr.required, attr.identity, attr.generated,
                            attr.unique, attr.internal, attr.deprecated,
                        ],
                    ).map_err(|e| anyhow::anyhow!(e.to_string()))?;
                }
            }
        }
    }

    let model_count = sdef.data_models.as_ref().map(|v| v.len()).unwrap_or(0);
    println!("Imported document '{}' with {} data models", sdef.name, model_count);
    Ok(())
}

fn migrate_command(direction: &str, db_path: &str) -> Result<()> {
    match direction {
        "up" => {
            let _db = Database::open(Path::new(db_path))?;
            println!("Migrations applied successfully");
            Ok(())
        }
        "down" => {
            println!("Down migration not supported in this version");
            Ok(())
        }
        _ => {
            println!("Unknown migration direction: {}", direction);
            Ok(())
        }
    }
}