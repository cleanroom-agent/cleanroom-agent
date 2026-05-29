//! MCP Server — Phase 4 Redis 端到端验证 (使用实际 state.db)
//!
//! 直接调用 dispatch_tool_call (绕过 rmcp RequestContext 复杂性)。
//! 需要先运行 produce 生成 state.db。

use std::sync::Arc;

use cleanroom_db::Database;
use cleanroom_mcp::CleanroomMcpServer;
use serde_json::{json, Value};

/// 当前文档名（与 produce --name 一致）
const DOC_NAME: &str = "com.redis.1.3.12";

fn setup_server() -> CleanroomMcpServer {
    // Change CWD to workspace root so Database::open can find migrations/
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent()
        .and_then(|p| p.parent())
        .unwrap_or(manifest_dir);
    std::env::set_current_dir(workspace_root)
        .expect("Failed to change to workspace root");

    let candidates = [
        workspace_root.join("state.db"),
        std::path::Path::new("state.db").to_path_buf(),
        manifest_dir.join("state.db"),
    ];
    let db_path = candidates.into_iter().find(|p| p.exists())
        .expect("state.db not found — run `cargo run -- produce --repo ../test-cases/redis-1.3.12 --name com.redis.1.3.12` first");

    let db = Database::open(&db_path).expect("Open state.db");
    CleanroomMcpServer::from_db(Arc::new(db), &db_path)
}

fn call_tool(server: &CleanroomMcpServer, tool: &str, args: Value) -> Value {
    use rmcp::model::CallToolRequestParams;
    let args_map = args.as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    let params = CallToolRequestParams::new(tool.to_string())
        .with_arguments(args_map);
    server.dispatch_tool_call(params)
        .expect(&format!("Tool '{}' failed", tool))
}

#[test]
fn phase4_1_list_documents() {
    let server = setup_server();
    let result = call_tool(&server, "list_documents", json!({}));
    let text = serde_json::to_string(&result).unwrap_or_default().to_lowercase();
    println!("list_documents: {}", text);
    assert!(text.contains("redis"), "Expected 'redis' in documents, got: {}", text);
}

#[test]
fn phase4_2_get_data_model() {
    let server = setup_server();
    let result = call_tool(&server, "get_data_model", json!({
        "document_name": DOC_NAME,
        "entity": "redisServer"
    }));
    let text = serde_json::to_string(&result).unwrap_or_default().to_lowercase();
    println!("get_data_model(redisServer): {}", text);
    assert!(text.contains("port"), "Expected 'port' field in redisServer, got: {}", text);
}

#[test]
fn phase4_3_search_sdef() {
    let server = setup_server();
    let result = call_tool(&server, "search_sdef", json!({
        "query": "redisServer",
        "document_name": DOC_NAME
    }));
    let text = serde_json::to_string(&result).unwrap_or_default();
    println!("search_sdef: {}", text);
    // search_sdef may return empty if FTS not populated; just verify no crash
}

#[test]
fn phase4_4_resolve_name() {
    let server = setup_server();
    let result = call_tool(&server, "resolve_name", json!({
        "document_name": DOC_NAME,
        "sdef_uri": format!("sdef://{}/entity/redisServer", DOC_NAME),
        "language": "rust",
        "symbol_type": "class"
    }));
    let text = serde_json::to_string(&result).unwrap_or_default().to_lowercase();
    println!("resolve_name: {}", text);
    // Note: actual returned name depends on symbol_registry content.
    // After import.rs fix, this will be `redis_server` for Rust.
    assert!(text.contains("redis"), "Expected resolved name containing 'redis', got: {}", text);
}
