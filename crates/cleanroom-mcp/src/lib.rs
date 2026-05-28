//! cleanroom-mcp — MCP server for Cleanroom Agent.

use std::path::Path;
use std::sync::Arc;

use rmcp::{
    model::{ServerInfo, ServerCapabilities, Implementation},
    ServerHandler, serve_server, ErrorData,
};
use tracing::info;

use cleanroom_db::Database;

pub mod tools;

/// The Cleanroom MCP server.
#[derive(Debug, Clone)]
pub struct CleanroomMcpServer {
    /// Database connection.
    pub db: Arc<Database>,
}

impl CleanroomMcpServer {
    /// Create a new MCP server instance.
    pub fn new(db_path: &Path) -> Result<Self, ErrorData> {
        let db = Database::open(db_path)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Start the server over stdio transport.
    pub async fn serve(self) -> Result<(), ErrorData> {
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        let _running = serve_server(self, transport).await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(())
    }
}

impl ServerHandler for CleanroomMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("cleanroom-agent", env!("CARGO_PKG_VERSION")))
            .with_instructions("S.DEF intelligent agent system")
    }
}