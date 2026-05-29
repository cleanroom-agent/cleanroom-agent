//! LSP JSON-RPC client — communicates with LSP servers over stdio.
//!
//! Uses raw JSON messages rather than lsp_types structs to avoid API
//! compatibility issues across lsp-types versions.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde_json::Value;
use tracing::info;

use super::error::{LspError, LspResult};
use super::file_analysis::{Diagnostic, DocumentSymbol, FileAnalysis, TypeHierarchy, TypeHierarchyItem, TypeInfo};

/// A JSON-RPC message with Content-Length framing.
struct JsonRpcMessage;

impl JsonRpcMessage {
    fn encode(body: &str) -> Vec<u8> {
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut buf = Vec::with_capacity(header.len() + body.len());
        buf.extend_from_slice(header.as_bytes());
        buf.extend_from_slice(body.as_bytes());
        buf
    }

    fn decode(reader: &mut dyn BufRead) -> Result<Option<String>, String> {
        let mut header_line = String::new();
        let mut content_length: Option<usize> = None;

        loop {
            header_line.clear();
            if reader.read_line(&mut header_line).map_err(|e| e.to_string())? == 0 {
                return Ok(None);
            }
            let trimmed = header_line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
                content_length =
                    Some(len_str.trim().parse::<usize>().map_err(|e| e.to_string())?);
            }
        }

        let len = content_length.ok_or_else(|| "Missing Content-Length header".to_string())?;
        let mut body_buf = vec![0u8; len];
        reader.read_exact(&mut body_buf).map_err(|e| e.to_string())?;
        String::from_utf8(body_buf).map_err(|e| e.to_string()).map(Some)
    }
}

/// A client connected to a running LSP server.
pub struct LspClient {
    #[allow(dead_code)]
    child: Child,
    stdin: Mutex<ChildStdin>,
    stdout: Mutex<BufReader<ChildStdout>>,
    next_id: AtomicU64,
    pub language: String,
    #[allow(dead_code)]
    initialized: bool,
}

impl LspClient {
    /// Spawn an LSP server process and perform the initialize handshake.
    pub fn spawn(
        command: &str,
        args: &[String],
        language: &str,
        workspace_root: &str,
    ) -> LspResult<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                LspError::ServerStartFailed(format!("Failed to spawn '{}': {}", command, e))
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or(LspError::ServerStartFailed("No stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(LspError::ServerStartFailed("No stdout".to_string()))?;

        let mut client = Self {
            child,
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            next_id: AtomicU64::new(1),
            language: language.to_string(),
            initialized: false,
        };

        client.initialize(workspace_root)?;
        Ok(client)
    }

    fn initialize(&mut self, workspace_root: &str) -> LspResult<()> {
        let init_params = serde_json::json!({
            "processId": null,
            "rootUri": format!("file://{}", workspace_root),
            "capabilities": {
                "textDocument": {
                    "documentSymbol": { "dynamicRegistration": false },
                    "hover": { "dynamicRegistration": false },
                    "diagnostic": { "dynamicRegistration": false }
                }
            }
        });
        let response = self.send_request("initialize", init_params)?;
        if response.get("error").is_some() {
            return Err(LspError::AnalysisFailed(format!("Initialize error: {:?}", response)));
        }
        info!(language = %self.language, "LSP server initialized");
        self.send_notification("initialized", serde_json::json!({}))?;
        self.initialized = true;
        Ok(())
    }

    /// Open a document.
    pub fn did_open(&self, file_path: &str, text: &str, language_id: &str) -> LspResult<()> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": format!("file://{}", file_path),
                "languageId": language_id,
                "version": 1,
                "text": text
            }
        });
        self.send_notification("textDocument/didOpen", params)
    }

    /// Get document symbols.
    pub fn document_symbols(&self, file_path: &str) -> LspResult<Vec<DocumentSymbol>> {
        let params = serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file_path) }
        });
        let response = self.send_request("textDocument/documentSymbol", params)?;
        let raw = match (response.get("result"), response.get("error")) {
            (Some(r), _) => r.clone(),
            (None, Some(e)) => {
                return Err(LspError::AnalysisFailed(format!("LSP error: {:?}", e)));
            }
            (None, None) => return Ok(vec![]),
        };
        convert_symbols_from_json(&raw)
    }

    /// Get hover/type info at a position.
    pub fn hover(&self, file_path: &str, line: u32, character: u32) -> LspResult<Option<TypeInfo>> {
        let params = serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file_path) },
            "position": { "line": line, "character": character }
        });
        let response = self.send_request("textDocument/hover", params)?;
        if let Some(result) = response.get("result") {
            if result.is_null() {
                return Ok(None);
            }
            let type_name = result
                .get("contents")
                .and_then(|c| c.get("value").and_then(|v| v.as_str()))
                .or_else(|| result.get("contents").and_then(|c| c.as_str()))
                .unwrap_or("unknown")
                .to_string();
            Ok(Some(TypeInfo { type_name, kind: None, documentation: None, defined_in_file: None, range: None, type_parameters: vec![] }))
        } else {
            Ok(None)
        }
    }

    /// Find references at a position.
    pub fn find_references(&self, file_path: &str, line: u32, character: u32) -> LspResult<Vec<lsp_types::Location>> {
        let params = serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file_path) },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        });
        let response = self.send_request("textDocument/references", params)?;
        if let Some(result) = response.get("result") {
            Ok(serde_json::from_value(result.clone()).unwrap_or_default())
        } else {
            Ok(vec![])
        }
    }

    /// Get diagnostics.
    pub fn diagnostics(&self, file_path: &str) -> LspResult<Vec<Diagnostic>> {
        let params = serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file_path) }
        });
        let response = self.send_request("textDocument/diagnostic", params);
        if let Ok(resp) = response {
            if let Some(result) = resp.get("result") {
                if let Some(items) = result.get("items").and_then(|v| v.as_array()) {
                    return Ok(items.iter().filter_map(|d| {
                        let msg = d.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let sev = super::file_analysis::DiagnosticSeverity::Information;
                        let range = d.get("range").and_then(|r| {
                            let sl = r.get("start")?.get("line")?.as_u64()? as u32;
                            let sc = r.get("start")?.get("character")?.as_u64()? as u32;
                            let el = r.get("end")?.get("line")?.as_u64()? as u32;
                            let ec = r.get("end")?.get("character")?.as_u64()? as u32;
                            Some((sl, sc, el, ec))
                        });
                        Some(Diagnostic { severity: sev, message: msg, range, code: None })
                    }).collect());
                }
            }
        }
        Ok(vec![])
    }

    /// Get type hierarchy supertypes.
    pub fn type_hierarchy(&self, file_path: &str, line: u32, character: u32) -> LspResult<TypeHierarchy> {
        let params = serde_json::json!({
            "item": {
                "uri": format!("file://{}", file_path),
                "range": { "start": { "line": line, "character": character }, "end": { "line": line, "character": character + 1 } },
                "selectionRange": { "start": { "line": line, "character": character }, "end": { "line": line, "character": character + 1 } },
                "name": "", "kind": 5
            }
        });
        let response = self.send_request("textDocument/typeHierarchy/supertypes", params)?;
        let supertypes: Vec<TypeHierarchyItem> = response.get("result").and_then(|r| r.as_array())
            .map(|arr| arr.iter().map(|item| TypeHierarchyItem {
                name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                kind: item.get("kind").and_then(|v| v.as_u64()).map(|k| format!("{}", k)).unwrap_or_default(),
                file_path: item.get("uri").and_then(|v| v.as_str()).map(|s| s.to_string()),
                range: None,
            }).collect()).unwrap_or_default();
        Ok(TypeHierarchy { type_name: String::new(), supertypes, subtypes: vec![] })
    }

    /// Full analysis: open + get symbols.
    pub fn analyze_file(&self, file_path: &str, language_id: &str) -> LspResult<FileAnalysis> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| LspError::AnalysisFailed(format!("Cannot read {}: {}", file_path, e)))?;
        self.did_open(file_path, &content, language_id)?;
        let symbols = self.document_symbols(file_path)?;
        let dx = self.diagnostics(file_path)?;
        Ok(FileAnalysis {
            file_path: file_path.to_string(),
            language: language_id.to_string(),
            symbols,
            imports: vec![],
            exports: vec![],
            references: vec![],
            diagnostics: dx,
        })
    }

    // ============ Internal JSON-RPC ============

    fn send_notification(&self, method: &str, params: Value) -> LspResult<()> {
        let body = serde_json::json!({ "jsonrpc": "2.0", "method": method, "params": params });
        let encoded = JsonRpcMessage::encode(&body.to_string());
        self.stdin.lock().map_err(|e| LspError::CommunicationError(e.to_string()))?
            .write_all(&encoded).map_err(|e| LspError::CommunicationError(e.to_string()))?;
        Ok(())
    }

    fn send_request(&self, method: &str, params: Value) -> LspResult<Value> {
        let req_id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let body = serde_json::json!({ "jsonrpc": "2.0", "id": req_id, "method": method, "params": params });
        let encoded = JsonRpcMessage::encode(&body.to_string());
        self.stdin.lock().map_err(|e| LspError::CommunicationError(e.to_string()))?
            .write_all(&encoded).map_err(|e| LspError::CommunicationError(e.to_string()))?;

        loop {
            let raw = self.read_raw()
                .map_err(|e| LspError::CommunicationError(e.to_string()))?;
            let response: Value = serde_json::from_str(&raw)
                .map_err(|e| LspError::CommunicationError(e.to_string()))?;
            if let Some(resp_id) = response.get("id").and_then(|v| v.as_u64()) {
                if resp_id == req_id {
                    return Ok(response);
                }
            }
        }
    }

    fn read_raw(&self) -> Result<String, String> {
        let mut stdout = self.stdout.lock().map_err(|e| e.to_string())?;
        loop {
            let raw = JsonRpcMessage::decode(&mut *stdout)?;
            if let Some(msg) = raw {
                return Ok(msg);
            }
        }
    }
}

// ============ Symbol Conversion ============

fn convert_symbols_from_json(value: &Value) -> LspResult<Vec<DocumentSymbol>> {
    match value {
        Value::Array(arr) => Ok(arr.iter().filter_map(parse_symbol).collect()),
        _ => Ok(vec![]),
    }
}

fn parse_symbol(item: &Value) -> Option<DocumentSymbol> {
    let name = item.get("name")?.as_str()?.to_string();
    let range = item.get("range").and_then(|r| {
        Some((
            r.get("start")?.get("line")?.as_u64()? as u32,
            r.get("start")?.get("character")?.as_u64()? as u32,
            r.get("end")?.get("line")?.as_u64()? as u32,
            r.get("end")?.get("character")?.as_u64()? as u32,
        ))
    });
    let children = item.get("children").and_then(|c| c.as_array())
        .map(|arr| arr.iter().filter_map(parse_symbol).collect()).unwrap_or_default();
    let detail = item.get("detail").and_then(|v| v.as_str()).map(|s| s.to_string());
    Some(DocumentSymbol { name, kind: lsp_types::SymbolKind::OBJECT, range, children, detail })
}

impl std::fmt::Debug for LspClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspClient").field("language", &self.language).field("initialized", &self.initialized).finish()
    }
}
