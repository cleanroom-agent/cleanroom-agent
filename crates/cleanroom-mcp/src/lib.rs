//! cleanroom-mcp — MCP server for Cleanroom Agent.
//!
//! Uses the official MCP Rust SDK (`rmcp`):
//! <https://github.com/modelcontextprotocol/rust-sdk>

use rmcp::{
    ServerHandler, serve_server, ErrorData,
    model::{ServerInfo, Implementation, ServerCapabilities},
    tool, tool_handler, tool_router,
    handler::server::router::tool::ToolRouter,
};

pub mod tools;

/// The Cleanroom MCP server.
#[derive(Debug, Clone)]
pub struct CleanroomMcpServer {
    /// Tool router for dispatching MCP tool calls.
    tool_router: ToolRouter<Self>,
    /// Path to the SQLite database.
    pub db_path: String,
}

impl CleanroomMcpServer {
    /// Create a new MCP server instance.
    pub fn new(db_path: String) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db_path,
        }
    }

    /// Start the server over stdio transport.
    pub async fn serve(self) -> Result<(), ErrorData> {
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        let _running = serve_server(self, transport).await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(())
    }

    fn server_info_impl() -> Implementation {
        let mut info = Implementation::new("cleanroom-agent", env!("CARGO_PKG_VERSION"));
        info.title = Some("Cleanroom Agent MCP Server".into());
        info.description = Some("S.DEF intelligent agent system".into());
        info
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CleanroomMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Self::server_info_impl())
            .with_instructions("S.DEF intelligent agent system for software definition exchange")
    }
}

// ============================================================
// Tool definitions
// ============================================================

#[tool_router(router = tool_router)]
impl CleanroomMcpServer {
    /// Ping the server.
    #[tool(description = "Ping the server")]
    async fn ping(&self) -> String {
        "ok".to_string()
    }
}
