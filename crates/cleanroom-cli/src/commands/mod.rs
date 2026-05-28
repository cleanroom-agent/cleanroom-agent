//! CLI commands.

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Production mode: analyze code repository → output S.DEF
    Produce {
        /// Repository path
        #[arg(long)]
        repo: String,

        /// Output directory
        #[arg(long, default_value = "./sdef-output")]
        output: String,

        /// Exclude patterns (gitignore-like)
        #[arg(long)]
        exclude: Option<String>,

        /// Project name
        #[arg(long)]
        name: Option<String>,
    },

    /// Consumption mode: read S.DEF → generate code
    Consume {
        /// S.DEF file or directory
        #[arg(long)]
        sdef: String,

        /// Output directory
        #[arg(long, default_value = "./output")]
        output: String,

        /// Target language
        #[arg(long)]
        language: String,

        /// Target framework
        #[arg(long)]
        framework: Option<String>,

        /// Compatibility mode
        #[arg(long, default_value = "full")]
        compat_mode: String,

        /// Reconstruction fidelity
        #[arg(long, default_value = "production_equivalent")]
        fidelity: String,
    },

    /// MCP server mode
    Serve {
        /// Transport (stdio/http)
        #[arg(long, default_value = "stdio")]
        transport: String,
    },

    /// Resume workflow
    Resume {
        /// Workflow ID
        #[arg(long)]
        workflow_id: String,
    },

    /// Inspect database/S.DEF state
    Inspect {
        /// Check type
        #[arg(long, default_value = "consistency")]
        check_type: String,
    },

    /// Database migration
    Migrate {
        /// Direction
        #[arg(long, default_value = "up")]
        direction: String,
    },
}

pub fn run(command: Commands, _db_path: &str) -> Result<()> {
    match command {
        Commands::Produce { repo, output, exclude, name } => {
            todo!("produce command: repo={}, output={}, exclude={:?}, name={:?}", repo, output, exclude, name)
        }
        Commands::Consume { sdef, output, language, framework, compat_mode, fidelity } => {
            todo!("consume command: sdef={}, output={}, language={}, framework={:?}, compat_mode={}, fidelity={}", sdef, output, language, framework, compat_mode, fidelity)
        }
        Commands::Serve { transport } => {
            todo!("serve command: transport={}", transport)
        }
        Commands::Resume { workflow_id } => {
            todo!("resume command: workflow_id={}", workflow_id)
        }
        Commands::Inspect { check_type } => {
            todo!("inspect command: check_type={}", check_type)
        }
        Commands::Migrate { direction } => {
            todo!("migrate command: direction={}", direction)
        }
    }
}
