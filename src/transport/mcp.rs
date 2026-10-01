use crate::core::DecisionWorkflow;
use crate::domain::BrowseOptions;
use crate::storage::Queries;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

pub struct McpServer;

impl McpServer {
    /// Runs the stdio MCP server loop, processing JSON-RPC 2.0 messages from stdin and replying on stdout.
    pub fn run_stdio<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        profile_id: &str,
    ) -> std::io::Result<()> {
        let root = root.as_ref();
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let reader = std::io::BufReader::new(stdin.lock());

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
                if let Some(resp) = Self::handle_request(root, conn, collection_id, profile_id, req) {
                    let mut out = serde_json::to_string(&resp).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    })?;
                    out.push('\n');
                    stdout.write_all(out.as_bytes())?;
                    stdout.flush()?;
                }
            }
        }

        Ok(())
    }

    /// Handles a single JSON-RPC request and returns a response, or None if it's a notification.
    pub fn handle_request<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        profile_id: &str,
        req: JsonRpcRequest,
    ) -> Option<JsonRpcResponse> {
        let root = root.as_ref();
        // Notifications have no `id` and require no reply
        let id = match req.id {
            Some(id) => id,
            None => return None,
        };

        match req.method.as_str() {
            "initialize" => Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "hyperkb",
                        "version": "0.1.0"
                    }
                })),
                error: None,
            }),

            "ping" => Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({})),
                error: None,
            }),

            "tools/list" => Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({
                    "tools": Self::tool_definitions()
                })),
                error: None,
            }),

            "tools/call" => {
                let result = Self::handle_tool_call(root, conn, collection_id, profile_id, req.params);
                match result {
                    Ok(val) => Some(JsonRpcResponse {
                        jsonrpc: "2.0",
                        id,
                        result: Some(val),
                        error: None,
                    }),
                    Err(err_msg) => Some(JsonRpcResponse {
                        jsonrpc: "2.0",
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: err_msg,
                        }),
                    }),
                }
            }

            _ => Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method '{}' not found", req.method),
                }),
            }),
        }
    }

    fn tool_definitions() -> Vec<Value> {
        vec![
            json!({
                "name": "check_work",
                "description": "Before editing or proposing changes, check planned files against cited open risks and architectural boundaries. Incomplete coverage is never safe.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Root-relative file paths the agent plans to edit or inspect"
                        },
                        "version": {
                            "type": "string",
                            "description": "Optional exact target version label (e.g. v2.0)"
                        },
                        "environment": {
                            "type": "string",
                            "description": "Optional target deployment environment (e.g. production, staging)"
                        }
                    },
                    "required": ["files"]
                }
            }),
            json!({
                "name": "search",
                "description": "Search repository knowledge base, decisions, specs, and architectural records using BM25 full-text search. Private memory is withheld for agent safety.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query terms"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results to return (default: 10)"
                        }
                    },
                    "required": ["query"]
                }
            }),
            json!({
                "name": "browse",
                "description": "Browse repository documents by category (decisions, risks, specs, plans) or topic.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "description": "Optional category filter: all, decisions, risks, specs, plans"
                        },
                        "topic": {
                            "type": "string",
                            "description": "Optional topic filter"
                        }
                    }
                }
            }),
            json!({
                "name": "get_document",
                "description": "Fetch the full text and metadata of a document by its stable source ID or path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Stable document source ID or relative path"
                        }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "remember",
                "description": "Save a local working note into private memory. Local notes stay private to human review and are never committed as shared decisions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Title of the private note"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content of the private note"
                        }
                    },
                    "required": ["title", "content"]
                }
            }),
            json!({
                "name": "draft_decision",
                "description": "Create a proposed repo decision document for owner review. It cannot accept or supersede a decision.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Human-readable decision title"
                        },
                        "rationale": {
                            "type": "string",
                            "description": "Detailed architectural motivation, trade-offs, and context"
                        },
                        "author": {
                            "type": "string",
                            "description": "Agent name, persona, or author declaration"
                        },
                        "supersedes": {
                            "type": "string",
                            "description": "Optional stable ID or relative path of the existing accepted decision this replaces"
                        }
                    },
                    "required": ["title", "rationale", "author"]
                }
            }),
        ]
    }

    fn handle_tool_call(
        root: &Path,
        conn: &Connection,
        collection_id: &str,
        profile_id: &str,
        params: Option<Value>,
    ) -> Result<Value, String> {
        let params = params.ok_or_else(|| "Missing params for tools/call".to_string())?;
        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing tool name in tools/call".to_string())?;
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        match tool_name {
            "check_work" => {
                let files: Vec<String> = args
                    .get("files")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(|str_val| str_val.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                if files.is_empty() {
                    return Err("Missing required argument 'files' for check_work".into());
                }

                let version = args.get("version").and_then(|v| v.as_str());
                let env = args.get("environment").and_then(|v| v.as_str());

                let check = Queries::check_work(conn, collection_id, &files, version, env)
                    .map_err(|e| format!("Failed to check work: {}", e))?;

                let serialized = serde_json::to_string_pretty(&check)
                    .map_err(|e| format!("Serialization error: {}", e))?;

                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serialized
                        }
                    ],
                    "isError": false
                }))
            }

            "search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'query' for search".to_string())?;
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|u| u as usize)
                    .unwrap_or(10);

                // Note: include_private = false strictly withholds private memory from agents!
                let hits = Queries::search(
                    conn,
                    &[collection_id.to_string()],
                    profile_id,
                    query,
                    limit,
                    false,
                )
                .map_err(|e| format!("Failed to search: {}", e))?;

                let serialized = serde_json::to_string_pretty(&hits)
                    .map_err(|e| format!("Serialization error: {}", e))?;

                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serialized
                        }
                    ],
                    "isError": false
                }))
            }

            "browse" => {
                let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("all").to_string();
                let topic = args.get("topic").and_then(|v| v.as_str()).map(String::from);

                let opts = BrowseOptions {
                    category,
                    topic,
                    limit: 20,
                    ..Default::default()
                };

                let (docs, total) = Queries::browse(conn, &[collection_id.to_string()], &opts)
                    .map_err(|e| format!("Failed to browse: {}", e))?;

                let result_payload = json!({
                    "total": total,
                    "documents": docs
                });

                let serialized = serde_json::to_string_pretty(&result_payload)
                    .map_err(|e| format!("Serialization error: {}", e))?;

                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serialized
                        }
                    ],
                    "isError": false
                }))
            }

            "get_document" => {
                let doc_id = args
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'id' for get_document".to_string())?;

                let doc = Queries::get_document(conn, doc_id)
                    .map_err(|e| format!("Failed to get document: {}", e))?;

                match doc {
                    Some(d) => {
                        let serialized = serde_json::to_string_pretty(&d)
                            .map_err(|e| format!("Serialization error: {}", e))?;
                        Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serialized
                                }
                            ],
                            "isError": false
                        }))
                    }
                    None => Ok(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Document '{}' not found", doc_id)
                            }
                        ],
                        "isError": true
                    })),
                }
            }

            "remember" => {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'title' for remember".to_string())?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'content' for remember".to_string())?;

                let new_id = uuid::Uuid::now_v7().to_string();
                Queries::remember(conn, &new_id, profile_id, title, content, "note")
                    .map_err(|e| format!("Failed to save memory: {}", e))?;

                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("Saved note '{}' (ID: {}) to private memory.", title, new_id)
                        }
                    ],
                    "isError": false
                }))
            }

            "draft_decision" => {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'title' for draft_decision".to_string())?;
                let rationale = args
                    .get("rationale")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'rationale' for draft_decision".to_string())?;
                let author = args
                    .get("author")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'author' for draft_decision".to_string())?;
                let supersedes = args.get("supersedes").and_then(|v| v.as_str());

                match DecisionWorkflow::draft_replacement(
                    root,
                    conn,
                    collection_id,
                    title,
                    rationale,
                    author,
                    supersedes,
                ) {
                    Ok(draft) => {
                        let reply = json!({
                            "path": draft.path,
                            "id": draft.id,
                            "status": draft.status,
                            "message": format!("Draft decision saved to '{}'. Awaiting owner review before acceptance.", draft.path)
                        });
                        Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&reply).unwrap_or_default()
                                }
                            ],
                            "isError": false
                        }))
                    }
                    Err(err) => Ok(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Refused draft: {}", err)
                            }
                        ],
                        "isError": true
                    })),
                }
            }

            _ => Err(format!("Unknown tool '{}'", tool_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn test_mcp_initialize_and_tools_list() {
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();

        // 1. Initialize
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: Some(json!({})),
        };
        let init_resp = McpServer::handle_request(".", db.conn(), "coll_test", "prof_test", init_req).unwrap();
        assert_eq!(init_resp.id, json!(1));
        assert!(init_resp.result.unwrap()["serverInfo"]["name"] == "hyperkb");

        // 2. Tools list
        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "tools/list".into(),
            params: None,
        };
        let list_resp = McpServer::handle_request(".", db.conn(), "coll_test", "prof_test", list_req).unwrap();
        let tools = list_resp.result.unwrap()["tools"].as_array().unwrap().clone();
        let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(tool_names.contains(&"check_work"));
        assert!(tool_names.contains(&"search"));
        assert!(tool_names.contains(&"browse"));
        assert!(tool_names.contains(&"get_document"));
        assert!(tool_names.contains(&"remember"));
        assert!(tool_names.contains(&"draft_decision"));
    }

    #[test]
    fn test_mcp_check_work_tool_call() {
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();

        let call_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "check_work",
                "arguments": {
                    "files": ["src/main.rs"]
                }
            })),
        };

        let call_resp = McpServer::handle_request(".", db.conn(), "coll_test", "prof_test", call_req).unwrap();
        let content_text = call_resp.result.unwrap()["content"][0]["text"].as_str().unwrap().to_string();
        assert!(content_text.contains("checked_paths"));
        assert!(content_text.contains("src/main.rs"));
    }

    #[test]
    fn test_mcp_draft_decision_tool_call() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-mcp-draft-{}", uuid::Uuid::now_v7()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();

        let call_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "draft_decision",
                "arguments": {
                    "title": "Use Rust for All Kernels",
                    "rationale": "Rust delivers zero runtime overhead and memory safety.",
                    "author": "Antigravity Assistant"
                }
            })),
        };

        let call_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_test", "prof_test", call_req).unwrap();
        let content_text = call_resp.result.unwrap()["content"][0]["text"].as_str().unwrap().to_string();
        assert!(content_text.contains("docs/decisions/"));
        assert!(content_text.contains("proposed"));
    }
}
