//! Lightweight MCP client for CLI-to-server communication over TCP.
//!
//! Connects to a running `cleanroom serve --transport tcp://127.0.0.1:<port>`
//! process and sends MCP JSON-RPC tool calls. Used by `cleanroom inspect` and
//! `cleanroom task` subcommands.
//!
//! The TCP port is discovered by reading `cleanroom.port` from the system
//! temp directory, written by the server on startup.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Default TCP host for the MCP server (loopback only for security).
const DEFAULT_HOST: &str = "127.0.0.1";

/// Send an MCP tool call and return the JSON-RPC result.
///
/// Connects to the MCP TCP server, performs initialization handshake,
/// sends the tool call, and returns the deserialized result.
///
/// # Address Discovery
///
/// The `addr` parameter can be:
/// - `"host:port"` — explicit address (e.g., `"127.0.0.1:12345"`)
/// - `None` — auto-discover from the port file in the system temp directory
pub async fn call_mcp_tool(
    tool_name: &str,
    arguments: serde_json::Value,
    addr: Option<&str>,
) -> Result<serde_json::Value, String> {
    let connect_addr = if let Some(a) = addr {
        a.to_string()
    } else {
        discover_address()?
    };

    let stream = TcpStream::connect(&connect_addr).await
        .map_err(|e| format!(
            "Cannot connect to MCP server at '{}': {}\n\
             Is `cleanroom serve --transport tcp://{}` running?",
            connect_addr, e, connect_addr
        ))?;

    let (reader, mut writer) = tokio::io::split(stream);
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    // Phase 1: Initialize
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "cleanroom-cli",
                "version": "0.1.0"
            }
        }
    });

    send_json(&mut writer, &init_request).await?;
    let _init_response = read_response(&mut buf_reader, &mut line).await?;

    // Send initialized notification
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    send_json(&mut writer, &initialized).await?;

    // Phase 2: Call tool
    let call_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    });

    send_json(&mut writer, &call_request).await?;
    let response = read_response(&mut buf_reader, &mut line).await?;

    // Extract result from JSON-RPC response
    if let Some(error) = response.get("error") {
        let msg = error.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown MCP error");
        return Err(format!("MCP error: {}", msg));
    }

    let result = response.get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    Ok(result)
}

/// Call a tool synchronously (block_on wrapper for CLI).
pub fn call_mcp_tool_sync(
    tool_name: &str,
    arguments: serde_json::Value,
    addr: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;
    rt.block_on(call_mcp_tool(tool_name, arguments, addr))
        .map_err(|e| anyhow::anyhow!("MCP call '{}' failed: {}", tool_name, e))
}

/// Discover the MCP server address from the port file.
///
/// Reads `<temp_dir>/cleanroom.port` and constructs `127.0.0.1:<port>`.
pub fn discover_address() -> Result<String, String> {
    let port_path = std::env::temp_dir().join("cleanroom.port");
    let port_str = std::fs::read_to_string(&port_path)
        .map_err(|_| format!(
            "No port file found at '{}'. Is `cleanroom serve --transport tcp://` running?",
            port_path.display()
        ))?;
    let port: u16 = port_str.trim().parse()
        .map_err(|e| format!(
            "Invalid port file '{}': {}",
            port_path.display(), e
        ))?;

    Ok(format!("{}:{}", DEFAULT_HOST, port))
}

async fn send_json<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    value: &serde_json::Value,
) -> Result<(), String> {
    let mut json = serde_json::to_string(value)
        .map_err(|e| format!("JSON serialize error: {}", e))?;
    json.push('\n');
    writer.write_all(json.as_bytes()).await
        .map_err(|e| format!("Write error: {}", e))?;
    writer.flush().await
        .map_err(|e| format!("Flush error: {}", e))?;
    Ok(())
}

async fn read_response<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
    line_buf: &mut String,
) -> Result<serde_json::Value, String> {
    line_buf.clear();
    let n = reader.read_line(line_buf).await
        .map_err(|e| format!("Read error: {}", e))?;
    if n == 0 {
        return Err("Connection closed by server".to_string());
    }
    serde_json::from_str(line_buf)
        .map_err(|e| format!("JSON parse error: {} (raw: {})", e, line_buf.trim()))
}
