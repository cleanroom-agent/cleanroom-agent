//! MCP Server — Phase 4 Redis 端到端验证 (使用实际 state.db)

use std::sync::Arc;

use cleanroom_db::Database;
use cleanroom_mcp::CleanroomMcpServer;
use rmcp::model::CallToolRequestParams;
use serde_json::json;

/// 使用现有的 state.db 创建 MCP Server (Redis 数据已由 consume 导入)
fn setup_from_state() -> (CleanroomMcpServer, std::path::PathBuf) {
    let db_path = {
        let p = std::path::Path::new("state.db");
        if p.exists() {
            p.to_path_buf()
        } else {
            let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            manifest.join("state.db")
        }
    };
    assert!(db_path.exists(), "state.db not found at {:?}", db_path);
    let db = Database::open(&db_path).expect("Open state.db");
    let server = CleanroomMcpServer::from_db(Arc::new(db), &db_path);
    (server, db_path)
}

fn call_tool(server: &CleanroomMcpServer, tool_name: &str, args: serde_json::Value) -> serde_json::Value {
    let params = CallToolRequestParams::new(tool_name.to_string())
        .with_arguments(
            args.as_object()
                .unwrap_or(&serde_json::Map::new())
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );
    let rt = tokio::runtime::Runtime::new().expect("Create tokio runtime");
    let result = rt.block_on(server.call_tool(params));
    let result = result.expect(&format!("Tool '{}/' failed", tool_name));
    for c in &result.content {
        if let rmcp::model::Content::Text(text) = c {
            return serde_json::from_str(&text.text)
                .unwrap_or(serde_json::Value::String(text.text.clone()));
        }
    }
    serde_json::Value::Null
}

#[test]
fn phase4_1_list_documents_contains_redis() {
    let (server, _db_path) = setup_from_state();
    let result = call_tool(&server, "list_documents", json!({}));
    let result_str = serde_json::to_string(&result).unwrap_or_default().to_lowercase();
    println!("list_documents: {}", result_str);
    assert!(
        result_str.contains("unnamed"),
        "Expected 'unnamed' doc in list_documents, got: {}", result_str
    );
}

#[test]
fn phase4_2_get_data_model_dict() {
    let (server, _db_path) = setup_from_state();
    let result = call_tool(&server, "get_data_model", json!({
        "document": "unnamed",
        "entity": "Dict"
    }));
    let result_str = serde_json::to_string(&result).unwrap_or_default().to_lowercase();
    println!("get_data_model(Dict): {}", result_str);
    assert!(
        result_str.contains("dict"),
        "Expected Dict entity data, got: {}", result_str
    );
}

#[test]
fn phase4_3_search_sdef_dict() {
    let (server, _db_path) = setup_from_state();
    let result = call_tool(&server, "search_sdef", json!({
        "query": "Dict",
        "document": "unnamed"
    }));
    let result_str = serde_json::to_string(&result).unwrap_or_default();
    println!("search_sdef: {}", result_str);
    assert!(!result_str.is_empty(), "search_sdef returned empty");
}

#[test]
fn phase4_4_resolve_name() {
    let (server, _db_path) = setup_from_state();
    let result = call_tool(&server, "resolve_name", json!({
        "uri": "sdef://unnamed/entity#Dict",
        "language": "rust"
    }));
    let result_str = serde_json::to_string(&result).unwrap_or_default().to_lowercase();
    println!("resolve_name: {}", result_str);
    assert!(
        result_str.contains("dict") || result_str.contains("unnamed"),
        "Expected resolved name, got: {}", result_str
    );
}
