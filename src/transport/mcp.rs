use crate::core::{DecisionWorkflow, DirectiveWorkflow, Git, GrantStore, RiskWorkflow, SessionManager};
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

        // Initialize active session for this stdio server connection
        let active_session = SessionManager::start_session(
            conn,
            collection_id,
            profile_id,
            "mcp_agent",
            None,
        ).ok();
        let active_session_id = active_session.as_ref().map(|s| s.id.as_str());

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
                if let Some(resp) = Self::handle_request_with_session(root, conn, collection_id, profile_id, active_session_id, req) {
                    let mut out = serde_json::to_string(&resp).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    })?;
                    out.push('\n');
                    stdout.write_all(out.as_bytes())?;
                    stdout.flush()?;
                }
            }
        }

        if let Some(ref sess) = active_session {
            let _ = SessionManager::end_session(conn, &sess.id, "completed");
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
        Self::handle_request_with_session(root, conn, collection_id, profile_id, None, req)
    }

    pub fn handle_request_with_session<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        profile_id: &str,
        session_id: Option<&str>,
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
                let result = Self::handle_tool_call(root, conn, collection_id, profile_id, session_id, req.params);
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
                "description": "Browse repository documents by category (tasks, decisions, risks, specs, plans), project, kind, status, or topic.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "description": "Optional category filter: all, tasks, decisions, risks, specs, plans"
                        },
                        "project": {
                            "type": "string",
                            "description": "Optional project filter (e.g. 'core-engine' or 'auth-service')"
                        },
                        "kind": {
                            "type": "string",
                            "description": "Optional kind filter: task, risk, decision, spec, plan, document"
                        },
                        "status": {
                            "type": "string",
                            "description": "Optional status filter: pending, in_progress, completed, blocked, open, accepted, superseded"
                        },
                        "topic": {
                            "type": "string",
                            "description": "Optional topic filter"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results to return (default: 20)"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Offset for pagination (default: 0)"
                        }
                    }
                }
            }),
            json!({
                "name": "list_projects",
                "description": "List all segregated projects in the repository with health, task counts, open risks, and status documentation coverage.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
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
                "description": "Create a proposed repo decision document for owner review. It cannot accept or supersede a decision without an authority grant.",
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
                        },
                        "grant_id": {
                            "type": "string",
                            "description": "Optional UUID of an active AuthorityGrant"
                        }
                    },
                    "required": ["title", "rationale", "author"]
                }
            }),
            json!({
                "name": "accept_decision",
                "description": "Ratify/accept a proposed decision document on behalf of the human authorizer using a valid AuthorityGrant. Human remains the responsible owner; delegation is recorded.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the proposed decision markdown file"
                        },
                        "grant_id": {
                            "type": "string",
                            "description": "UUID of the active AuthorityGrant"
                        },
                        "agent_id": {
                            "type": "string",
                            "description": "Agent name or identifier"
                        },
                        "supersedes": {
                            "type": "string",
                            "description": "Optional target decision ID to supersede"
                        }
                    },
                    "required": ["path", "grant_id"]
                }
            }),
            json!({
                "name": "draft_risk",
                "description": "Propose an architectural risk record citing affected file paths. Incomplete coverage or unverified risks alert developers before edits.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Human-readable risk title"
                        },
                        "rationale": {
                            "type": "string",
                            "description": "Detailed explanation of failure mode, historical regressions, and mitigation"
                        },
                        "author": {
                            "type": "string",
                            "description": "Agent name, persona, or author declaration"
                        },
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "File path glob patterns affected by this risk (e.g. ['src/storage/**', 'auth/tokens.go'])"
                        },
                        "versions": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional version labels this risk applies to"
                        },
                        "environments": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional deployment environments this risk applies to (e.g. ['production'])"
                        },
                        "grant_id": {
                            "type": "string",
                            "description": "Optional UUID of an active AuthorityGrant"
                        }
                    },
                    "required": ["title", "rationale", "author", "paths"]
                }
            }),
            json!({
                "name": "acknowledge_risk",
                "description": "Acknowledge an open risk citing a mitigating rationale under an active AuthorityGrant. Acknowledged risks no longer block commits.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "risk_id": {
                            "type": "string",
                            "description": "Stable ID or relative path of the risk"
                        },
                        "rationale": {
                            "type": "string",
                            "description": "Justification for why the risk is safely waived or mitigated"
                        },
                        "grant_id": {
                            "type": "string",
                            "description": "UUID of the active AuthorityGrant"
                        },
                        "agent_id": {
                            "type": "string",
                            "description": "Agent name or identifier"
                        }
                    },
                    "required": ["risk_id", "rationale", "grant_id"]
                }
            }),
            json!({
                "name": "get_session_briefing",
                "description": "Zero-ceremony warm-start context briefing. Returns active invariants (ADRs), high-priority risks, recent churn hotspots, friction warnings, and knowledge debt in 1 single shot.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "grant_scope": {
                            "type": "string",
                            "description": "Optional active authority grant scope pattern (e.g. 'src/**')"
                        }
                    }
                }
            }),
            json!({
                "name": "get_session_scorecard",
                "description": "Returns session efficiency and effectiveness metrics, including tool-to-edit ratio, review loop oscillations, and knowledge gaps.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID. If omitted, returns metrics for the current active session."
                        }
                    }
                }
            }),
            json!({
                "name": "record_session_metric",
                "description": "Record a file edit event or review oscillation in the session ledger to monitor coding velocity and detect review loops.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Root-relative path to the edited file"
                        },
                        "diff_lines": {
                            "type": "integer",
                            "description": "Number of lines changed in this edit (default: 0)"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "list_directives",
                "description": "List policy directives and standing invariants, optionally filtered by category, status, or target file paths via the Rule of 5.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "category": { "type": "string", "description": "Optional category (architecture, behavior, deployment, security)" },
                        "status": { "type": "string", "description": "Optional status filter (active, dormant, retired)" },
                        "paths": { "type": "array", "items": { "type": "string" }, "description": "Optional file paths to filter directives by relevance" }
                    }
                }
            }),
            json!({
                "name": "draft_directive",
                "description": "Propose or author a new standing policy directive adhering to the repo taxonomy.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Title of the directive" },
                        "category": { "type": "string", "description": "Valid repository taxonomy category (e.g. behavior, architecture)" },
                        "author": { "type": "string", "description": "Author or agent identifier" },
                        "scope": { "type": "array", "items": { "type": "string" }, "description": "File path patterns or ['*'] for global" },
                        "enforcement": { "type": "string", "description": "Enforcement mode: check_work, briefing, or manual" },
                        "supersedes": { "type": "string", "description": "Optional ID of a directive this replaces" },
                        "content": { "type": "string", "description": "Markdown rule definition" }
                    },
                    "required": ["title", "category"]
                }
            }),
            json!({
                "name": "retire_directive",
                "description": "Retire an existing directive so it is no longer enforced in pre-commit checks or briefings.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Directive ID (DIR-...)" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "audit_directives",
                "description": "Audit active repository directives against the Rule of 5 (bloat check) and stale file path patterns.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }),
            json!({
                "name": "audit_kb",
                "description": "Audit the repository knowledge base and documentation against anti-bloat ceilings (<250 lines, <2000 words), folder nesting limits (max 3 levels), frontmatter schema validity, and stale proposals (>90 days).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Optional subdirectory to audit (defaults to 'docs')" }
                    }
                }
            }),
        ]
    }

    fn ensure_session_id(
        conn: &Connection,
        collection_id: &str,
        profile_id: &str,
        session_id: Option<&str>,
    ) -> Option<String> {
        if let Some(id) = session_id {
            return Some(id.to_string());
        }
        if let Ok(sessions) = Queries::list_sessions(conn, collection_id, 5) {
            if let Some(active) = sessions.into_iter().find(|s| s.status == "active") {
                return Some(active.id);
            }
        }
        SessionManager::start_session(conn, collection_id, profile_id, "agent_mcp", None)
            .map(|s| s.id)
            .ok()
    }

    fn handle_tool_call(
        root: &Path,
        conn: &Connection,
        collection_id: &str,
        profile_id: &str,
        session_id: Option<&str>,
        params: Option<Value>,
    ) -> Result<Value, String> {
        let params = params.ok_or_else(|| "Missing params for tools/call".to_string())?;
        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing tool name in tools/call".to_string())?;
        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let current_sess_id = Self::ensure_session_id(conn, collection_id, profile_id, session_id);

        match tool_name {
            "check_work" => {
                let files: Vec<String> = args
                    .get("files")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(|str_val| {
                                crate::core::RiskEngine::normalize_path(str_val, Some(root))
                            }))
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();

                if files.is_empty() {
                    return Err("Missing required argument 'files' for check_work".into());
                }

                let version = args.get("version").and_then(|v| v.as_str());
                let env = args.get("environment").and_then(|v| v.as_str());

                let mut check = Queries::check_work(conn, collection_id, &files, version, env)
                    .map_err(|e| format!("Failed to check work: {}", e))?;

                let manifest = crate::domain::RepoManifest::load_or_default(root);
                let docs_dir = root.join(manifest.docs_root());
                for file in &files {
                    if file.ends_with(".md") || file.ends_with(".markdown") {
                        let full_p = root.join(file);
                        if full_p.exists() {
                            if let Ok(issues) = crate::core::KbLinter::lint_single_file(&full_p, &docs_dir) {
                                for issue in issues {
                                    check.hygiene_warnings.push(format!("KB Linter [{}]: {}", file, issue));
                                }
                            }
                        }
                    }
                    if let Ok(Some(rep)) = Git::check_file_comment_hygiene(root, file, false) {
                        if rep.is_excessive {
                            check.hygiene_warnings.push(format!(
                                "Excessive comment density in '{}': {}/{} added lines ({:.1}%) are comments. Keep code self-documenting per Comment Directive.",
                                rep.file_path, rep.comment_lines, rep.added_lines, rep.comment_ratio * 100.0
                            ));
                        }
                    }
                }

                if let Some(ref sess_id) = current_sess_id {
                    let first_file = files.first().map(|s| s.as_str()).unwrap_or("");
                    let _ = SessionManager::record_tool_call(conn, sess_id, "check_work", first_file, "{}");
                    for m in &check.matches {
                        if !m.suppressed {
                            let _ = SessionManager::record_risk_cited(conn, sess_id, first_file, &m.document.title);
                        }
                    }
                }

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

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "search", "", query);
                    let _ = SessionManager::record_search(conn, sess_id, query, hits.len());
                }

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
                let project = args.get("project").and_then(|v| v.as_str()).map(String::from);
                let kind = args.get("kind").and_then(|v| v.as_str()).map(String::from);
                let status = args.get("status").and_then(|v| v.as_str()).map(String::from);
                let topic = args.get("topic").and_then(|v| v.as_str()).map(String::from);
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(20);
                let offset = args.get("offset").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(0);

                if let Some(ref sess_id) = current_sess_id {
                    let log_param = project.as_deref().unwrap_or(&category);
                    let _ = SessionManager::record_tool_call(conn, sess_id, "browse", "", log_param);
                }

                let opts = BrowseOptions {
                    category,
                    collection_id: None,
                    project,
                    topic,
                    status,
                    kind,
                    recent: false,
                    limit,
                    offset,
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

            "list_projects" => {
                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "list_projects", "", "");
                }

                let projects = Queries::list_projects(conn, collection_id)
                    .map_err(|e| format!("Failed to list projects: {}", e))?;

                let serialized = serde_json::to_string_pretty(&projects)
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

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "get_document", doc_id, "{}");
                }

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

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "remember", "", title);
                }

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
                let grant_id = args
                    .get("grant_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "draft_decision", "", title);
                }

                let actor = GrantStore::resolve_actor(root, Some(author), grant_id, Some(author))
                    .map_err(|e| format!("Failed to resolve actor: {}", e))?;

                match DecisionWorkflow::draft_replacement(
                    root,
                    conn,
                    collection_id,
                    title,
                    rationale,
                    &actor,
                    supersedes,
                ) {
                    Ok(draft) => {
                        let reply = json!({
                            "path": draft.path,
                            "id": draft.id,
                            "status": draft.status,
                            "owner": actor.responsible_owner(),
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

            "accept_decision" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'path' for accept_decision".to_string())?;
                let grant_id_str = args
                    .get("grant_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'grant_id' (AuthorityGrant required for agent ratification)".to_string())?;
                let grant_id = uuid::Uuid::parse_str(grant_id_str)
                    .map_err(|_| "Invalid grant_id UUID format".to_string())?;
                let agent_id = args.get("agent_id").and_then(|v| v.as_str()).unwrap_or("agent");
                let supersedes = args.get("supersedes").and_then(|v| v.as_str());

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "accept_decision", path, "");
                }

                let actor = GrantStore::resolve_actor(root, Some(agent_id), Some(grant_id), None)?;

                match DecisionWorkflow::review_acceptance(root, conn, path, &actor, supersedes) {
                    Ok(review) => {
                        match DecisionWorkflow::accept_decision(root, conn, collection_id, &review, Some(&actor)) {
                            Ok(()) => {
                                let reply = json!({
                                    "path": path,
                                    "status": "accepted",
                                    "owner": actor.responsible_owner(),
                                    "delegated_agent": actor.name(),
                                    "grant_id": grant_id_str,
                                    "message": format!("Decision '{}' ratified under authority of human principal '{}'.", path, actor.responsible_owner())
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
                            Err(e) => Ok(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Refused decision acceptance: {}", e)
                                    }
                                ],
                                "isError": true
                            })),
                        }
                    }
                    Err(e) => Ok(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Refused decision review: {}", e)
                            }
                        ],
                        "isError": true
                    })),
                }
            }

            "draft_risk" => {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'title' for draft_risk".to_string())?;
                let rationale = args
                    .get("rationale")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'rationale' for draft_risk".to_string())?;
                let author = args
                    .get("author")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'author' for draft_risk".to_string())?;
                let paths: Vec<String> = args
                    .get("paths")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(|str_val| str_val.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                if paths.is_empty() {
                    return Err("Missing required argument 'paths' (must be non-empty array) for draft_risk".to_string());
                }

                if let Some(ref sess_id) = current_sess_id {
                    let target_path = paths.first().map(|s| s.as_str()).unwrap_or("");
                    let _ = SessionManager::record_tool_call(conn, sess_id, "draft_risk", target_path, title);
                }

                let versions: Vec<String> = args
                    .get("versions")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(|str_val| str_val.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let environments: Vec<String> = args
                    .get("environments")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(|str_val| str_val.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let grant_id = args
                    .get("grant_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());

                let actor = GrantStore::resolve_actor(root, Some(author), grant_id, Some(author))
                    .map_err(|e| format!("Failed to resolve actor: {}", e))?;

                match RiskWorkflow::draft_risk_with_actor(
                    root,
                    conn,
                    collection_id,
                    title,
                    rationale,
                    &actor,
                    paths,
                    versions,
                    environments,
                ) {
                    Ok(draft) => {
                        let reply = json!({
                            "path": draft.path,
                            "id": draft.id,
                            "status": draft.status,
                            "paths": draft.paths,
                            "owner": actor.responsible_owner(),
                            "message": format!("Draft risk saved to '{}' and indexed for pre-edit interception.", draft.path)
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
                                "text": format!("Refused draft risk: {}", err)
                            }
                        ],
                        "isError": true
                    })),
                }
            }

            "acknowledge_risk" => {
                let risk_id = args
                    .get("risk_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'risk_id' for acknowledge_risk".to_string())?;
                let rationale = args
                    .get("rationale")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'rationale' for acknowledge_risk".to_string())?;
                let grant_id_str = args
                    .get("grant_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'grant_id' (AuthorityGrant required for agent risk waiver)".to_string())?;
                let grant_id = uuid::Uuid::parse_str(grant_id_str)
                    .map_err(|_| "Invalid grant_id UUID format".to_string())?;
                let agent_id = args.get("agent_id").and_then(|v| v.as_str()).unwrap_or("agent");

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "acknowledge_risk", risk_id, rationale);
                    let _ = SessionManager::record_risk_prevented(conn, sess_id, risk_id, rationale);
                }

                let actor = GrantStore::resolve_actor(root, Some(agent_id), Some(grant_id), None)?;

                match RiskWorkflow::acknowledge_risk(root, conn, collection_id, risk_id, &actor, rationale) {
                    Ok(ack) => {
                        let reply = json!({
                            "risk_id": ack.id,
                            "path": ack.path,
                            "status": "acknowledged",
                            "owner": actor.responsible_owner(),
                            "delegated_agent": actor.name(),
                            "message": format!("Risk '{}' acknowledged under authority of human principal '{}'. It will no longer block pre-edit checks.", ack.path, actor.responsible_owner())
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
                                "text": format!("Refused risk acknowledgment: {}", err)
                            }
                        ],
                        "isError": true
                    })),
                }
            }

            "get_session_briefing" => {
                let grant_scope = args.get("grant_scope").and_then(|v| v.as_str());
                let briefing = SessionManager::generate_briefing(conn, collection_id, grant_scope)
                    .map_err(|e| format!("Failed to generate briefing: {}", e))?;

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "get_session_briefing", "", "{}");
                }

                let serialized = serde_json::to_string_pretty(&briefing)
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

            "get_session_scorecard" => {
                let target_id = args
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .or(current_sess_id.as_deref())
                    .ok_or_else(|| "No active session ID found".to_string())?;

                let scorecard = SessionManager::compute_scorecard(conn, target_id)
                    .map_err(|e| format!("Failed to compute scorecard: {}", e))?;

                let serialized = serde_json::to_string_pretty(&scorecard)
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

            "record_session_metric" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'path' for record_session_metric".to_string())?;

                let diff_lines = args
                    .get("diff_lines")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;

                let sess_id = current_sess_id
                    .as_deref()
                    .ok_or_else(|| "No active session available to record metrics".to_string())?;

                let is_loop = SessionManager::record_file_edit(conn, sess_id, path, diff_lines)
                    .map_err(|e| format!("Failed to record file edit: {}", e))?;

                let resp_payload = serde_json::json!({
                    "session_id": sess_id,
                    "path": path,
                    "diff_lines": diff_lines,
                    "review_oscillation_detected": is_loop
                });

                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&resp_payload).unwrap_or_default()
                        }
                    ],
                    "isError": false
                }))
            }

            "list_directives" => {
                let cat = args.get("category").and_then(|v| v.as_str());
                let status = args.get("status").and_then(|v| v.as_str());
                let paths: Vec<String> = args
                    .get("paths")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().filter_map(|s| s.as_str().map(|v| v.to_string())).collect()
                    })
                    .unwrap_or_default();

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "list_directives", "", cat.unwrap_or(""));
                }

                let dirs = if !paths.is_empty() {
                    Queries::get_active_directives_for_paths(conn, collection_id, &paths, 5)
                        .map_err(|e| format!("Database error: {}", e))?
                } else {
                    Queries::list_directives(conn, collection_id, cat, status)
                        .map_err(|e| format!("Database error: {}", e))?
                };

                let serialized = serde_json::to_string_pretty(&dirs)
                    .map_err(|e| format!("Serialization error: {}", e))?;

                Ok(json!({
                    "content": [{ "type": "text", "text": serialized }],
                    "isError": false
                }))
            }

            "draft_directive" => {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'title' for draft_directive".to_string())?;
                let category = args
                    .get("category")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'category' for draft_directive".to_string())?;
                let author = args.get("author").and_then(|v| v.as_str()).unwrap_or("agent");
                let scope: Vec<String> = args
                    .get("scope")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().filter_map(|s| s.as_str().map(|v| v.to_string())).collect()
                    })
                    .unwrap_or_else(|| vec!["*".to_string()]);
                let enforcement = args.get("enforcement").and_then(|v| v.as_str()).unwrap_or("check_work");
                let supersedes = args.get("supersedes").and_then(|v| v.as_str()).map(|s| s.to_string());
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "draft_directive", "", title);
                }

                match DirectiveWorkflow::draft_directive(
                    root,
                    conn,
                    collection_id,
                    title,
                    category,
                    author,
                    scope,
                    enforcement,
                    supersedes,
                    content,
                ) {
                    Ok(dir) => {
                        let serialized = serde_json::to_string_pretty(&dir)
                            .map_err(|e| format!("Serialization error: {}", e))?;
                        Ok(json!({
                            "content": [{ "type": "text", "text": serialized }],
                            "isError": false
                        }))
                    }
                    Err(err) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Error drafting directive: {}", err) }],
                        "isError": true
                    })),
                }
            }

            "retire_directive" => {
                let id = args
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'id' for retire_directive".to_string())?;

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "retire_directive", "", id);
                }

                match DirectiveWorkflow::retire_directive(root, conn, id) {
                    Ok(true) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Directive '{}' successfully retired.", id) }],
                        "isError": false
                    })),
                    Ok(false) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Directive '{}' not found.", id) }],
                        "isError": true
                    })),
                    Err(err) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Error retiring directive: {}", err) }],
                        "isError": true
                    })),
                }
            }

            "audit_directives" => {
                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "audit_directives", "", "");
                }

                match DirectiveWorkflow::audit_directives(root, conn, collection_id) {
                    Ok(rep) => {
                        let serialized = serde_json::to_string_pretty(&rep)
                            .map_err(|e| format!("Serialization error: {}", e))?;
                        Ok(json!({
                            "content": [{ "type": "text", "text": serialized }],
                            "isError": false
                        }))
                    }
                    Err(err) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Error running directive audit: {}", err) }],
                        "isError": true
                    })),
                }
            }

            "audit_kb" => {
                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "audit_kb", "", "");
                }

                let manifest = crate::domain::RepoManifest::load_or_default(root);
                let report = if let Some(sub_path) = args.get("path").and_then(|v| v.as_str()) {
                    let audit_dir = root.join(sub_path);
                    crate::core::KbLinter::audit_directory(&audit_dir)
                } else {
                    crate::core::KbLinter::audit_workspace(root, &manifest)
                };

                match report {
                    Ok(rep) => {
                        let serialized = serde_json::to_string_pretty(&rep)
                            .map_err(|e| format!("Serialization error: {}", e))?;
                        Ok(json!({
                            "content": [{ "type": "text", "text": serialized }],
                            "isError": false
                        }))
                    }
                    Err(err) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Error running KB audit: {}", err) }],
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
    use crate::domain::RecordMeta;
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
        assert!(tool_names.contains(&"draft_risk"));
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

    #[test]
    fn test_mcp_draft_risk_tool_call() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-mcp-risk-{}", uuid::Uuid::now_v7()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();

        let call_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "draft_risk",
                "arguments": {
                    "title": "Unbounded Channel Memory Leak",
                    "rationale": "Unbounded channels lead to memory exhaustion during high load.",
                    "author": "Antigravity Assistant",
                    "paths": ["src/transport/**"]
                }
            })),
        };

        let call_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_test", "prof_test", call_req).unwrap();
        let content_text = call_resp.result.unwrap()["content"][0]["text"].as_str().unwrap().to_string();
        assert!(content_text.contains("docs/risks/"));
        assert!(content_text.contains("open"));
    }

    #[test]
    fn test_mcp_accept_decision_with_grant() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-mcp-grant-dec-{}", uuid::Uuid::now_v7()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();

        // 1. Issue an AuthorityGrant from human principal "wiqar" to agent "opencode"
        let grant = GrantStore::issue_grant(
            &temp_dir,
            "opencode",
            "wiqar",
            vec![crate::domain::ActionKind::ProposeDecision, crate::domain::ActionKind::AcceptDecision],
            vec!["docs/decisions/**".to_string()],
            crate::domain::GrantConstraints::default(),
        )
        .unwrap();

        // 2. Agent drafts decision with grant
        let draft_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(6)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "draft_decision",
                "arguments": {
                    "title": "Adopt Delegated Authority Model",
                    "rationale": "Delegation ensures human accountability while empowering agents.",
                    "author": "opencode",
                    "grant_id": grant.grant_id.to_string()
                }
            })),
        };
        let draft_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_test", "prof_test", draft_req).unwrap();
        let draft_json: Value = serde_json::from_str(draft_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        let path = draft_json["path"].as_str().unwrap();
        assert_eq!(draft_json["owner"], "wiqar");

        // 3. Agent ratifies/accepts decision under grant
        let accept_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(7)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "accept_decision",
                "arguments": {
                    "path": path,
                    "grant_id": grant.grant_id.to_string(),
                    "agent_id": "opencode"
                }
            })),
        };
        let accept_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_test", "prof_test", accept_req).unwrap();
        let accept_json: Value = serde_json::from_str(accept_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(accept_json["status"], "accepted");
        assert_eq!(accept_json["owner"], "wiqar"); // Human principal is owner!
        assert_eq!(accept_json["delegated_agent"], "opencode");
    }

    #[test]
    fn test_mcp_acknowledge_risk_with_grant() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-mcp-grant-risk-{}", uuid::Uuid::now_v7()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();

        // 1. Issue grant to agent
        let grant = GrantStore::issue_grant(
            &temp_dir,
            "opencode",
            "wiqar",
            vec![crate::domain::ActionKind::AcknowledgeRisk],
            vec!["docs/risks/**".to_string()],
            crate::domain::GrantConstraints::default(),
        )
        .unwrap();

        // 2. Draft risk
        let draft = RiskWorkflow::draft_risk(
            &temp_dir,
            db.conn(),
            "coll_test",
            "Unindexed Foreign Key Contention",
            "May cause table locks on cascade delete.",
            "wiqar",
            vec!["src/storage/**".to_string()],
            vec![],
            vec![],
        )
        .unwrap();

        // 3. Agent acknowledges risk over MCP
        let ack_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(8)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "acknowledge_risk",
                "arguments": {
                    "risk_id": draft.id,
                    "rationale": "Foreign key index added in migration V04.",
                    "grant_id": grant.grant_id.to_string(),
                    "agent_id": "opencode"
                }
            })),
        };
        let ack_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_test", "prof_test", ack_req).unwrap();
        let ack_json: Value = serde_json::from_str(ack_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(ack_json["status"], "acknowledged");
        assert_eq!(ack_json["owner"], "wiqar"); // Human principal is owner!
        assert_eq!(ack_json["delegated_agent"], "opencode");
    }

    #[test]
    fn test_mcp_session_briefing_scorecard_and_metric() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-mcp-sess-{}", uuid::Uuid::now_v7()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();

        // 1. Request briefing
        let brief_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(9)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "get_session_briefing",
                "arguments": {
                    "grant_scope": "src/**"
                }
            })),
        };
        let brief_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_test", "prof_test", brief_req).unwrap();
        let brief_text = brief_resp.result.unwrap()["content"][0]["text"].as_str().unwrap().to_string();
        assert!(brief_text.contains("HyperKB Session Briefing"));
        assert!(brief_text.contains("src/**"));

        // 2. Record edits to src/db.rs (3 times to trigger oscillation detection)
        for i in 1..=3 {
            let metric_req = JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(10 + i)),
                method: "tools/call".into(),
                params: Some(json!({
                    "name": "record_session_metric",
                    "arguments": {
                        "path": "src/db.rs",
                        "diff_lines": 20
                    }
                })),
            };
            let metric_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_test", "prof_test", metric_req).unwrap();
            let metric_json: Value = serde_json::from_str(metric_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
            if i == 3 {
                assert_eq!(metric_json["review_oscillation_detected"], true);
            } else {
                assert_eq!(metric_json["review_oscillation_detected"], false);
            }
        }

        // 3. Request scorecard
        let scorecard_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(20)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "get_session_scorecard",
                "arguments": {}
            })),
        };
        let scorecard_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_test", "prof_test", scorecard_req).unwrap();
        let scorecard_json: Value = serde_json::from_str(scorecard_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(scorecard_json["review_loops_detected"], 1);
        let hotspots = scorecard_json["friction_hotspots"].as_array().unwrap();
        assert_eq!(hotspots[0].as_str().unwrap(), "src/db.rs");
    }

    #[test]
    fn test_mcp_directive_tools() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-mcp-dir-{}", uuid::Uuid::now_v7()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db = Database::open_in_memory("coll_dir", "prof_dir").unwrap();

        // 1. Draft directive via MCP
        let draft_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(31)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "draft_directive",
                "arguments": {
                    "title": "Prime Directive",
                    "category": "behavior",
                    "author": "agent_claud",
                    "scope": ["*"],
                    "enforcement": "check_work",
                    "content": "# Prime Directive\nNever corrupt user data."
                }
            })),
        };
        let draft_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_dir", "prof_dir", draft_req).unwrap();
        let draft_val: Value = serde_json::from_str(draft_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(draft_val["title"], "Prime Directive");
        let dir_id = draft_val["id"].as_str().unwrap().to_string();

        // 2. List directives via MCP
        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(32)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "list_directives",
                "arguments": {
                    "category": "behavior"
                }
            })),
        };
        let list_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_dir", "prof_dir", list_req).unwrap();
        let list_val: Value = serde_json::from_str(list_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(list_val.as_array().unwrap().len(), 1);

        // 3. Audit directives via MCP
        let audit_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(33)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "audit_directives",
                "arguments": {}
            })),
        };
        let audit_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_dir", "prof_dir", audit_req).unwrap();
        let audit_val: Value = serde_json::from_str(audit_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(audit_val["total_directives"], 1);
        assert_eq!(audit_val["global_count"], 1);

        // 4. Retire directive via MCP
        let retire_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(34)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "retire_directive",
                "arguments": {
                    "id": dir_id
                }
            })),
        };
        let retire_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_dir", "prof_dir", retire_req).unwrap();
        let retire_text = retire_resp.result.unwrap()["content"][0]["text"].as_str().unwrap().to_string();
        assert!(retire_text.contains("successfully retired"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_mcp_audit_kb_tool() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-mcp-kb-{}", uuid::Uuid::now_v7()));
        let docs_dir = temp_dir.join("docs").join("decisions");
        let _ = std::fs::create_dir_all(&docs_dir);
        let db = Database::open_in_memory("coll_kb", "prof_kb").unwrap();

        // 1. Create a valid concise document
        let doc1 = docs_dir.join("dec-001.md");
        std::fs::write(&doc1, "---hyperkb\n{\n  \"id\": \"DEC-2026-001\",\n  \"title\": \"Use SQLite\",\n  \"status\": \"accepted\",\n  \"kind\": \"decision\",\n  \"owner\": \"Developer\"\n}\n---\n# Architecture\nShort and clean.\n").unwrap();

        // 2. Call audit_kb via MCP
        let audit_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(41)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "audit_kb",
                "arguments": {
                    "path": "docs"
                }
            })),
        };
        let audit_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_kb", "prof_kb", audit_req).unwrap();
        let audit_val: Value = serde_json::from_str(audit_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(audit_val["total_documents"], 1);
        assert_eq!(audit_val["valid_documents"], 1);
        assert!(audit_val["bloat_warnings"].as_array().unwrap().is_empty());

        // 3. Create a bloated document (>250 lines)
        let doc2 = docs_dir.join("dec-002-bloat.md");
        let mut bloated_content = String::from("---hyperkb\n{\n  \"id\": \"DEC-2026-002\",\n  \"title\": \"Bloated Spec\",\n  \"status\": \"proposed\",\n  \"kind\": \"decision\",\n  \"owner\": \"Developer\"\n}\n---\n");
        for i in 0..270 {
            bloated_content.push_str(&format!("Line {} of excessive text that should be kept concise.\n", i));
        }
        std::fs::write(&doc2, &bloated_content).unwrap();

        // 4. Audit again and assert bloat warning
        let audit_req2 = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(42)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "audit_kb",
                "arguments": {}
            })),
        };
        let audit_resp2 = McpServer::handle_request(&temp_dir, db.conn(), "coll_kb", "prof_kb", audit_req2).unwrap();
        let audit_val2: Value = serde_json::from_str(audit_resp2.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(audit_val2["total_documents"], 2);
        assert_eq!(audit_val2["bloat_warnings"].as_array().unwrap().len(), 2);

        // 5. Test check_work with this bloated markdown doc
        let check_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(43)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "check_work",
                "arguments": {
                    "files": ["docs/decisions/dec-002-bloat.md"]
                }
            })),
        };
        let check_resp = McpServer::handle_request(&temp_dir, db.conn(), "coll_kb", "prof_kb", check_req).unwrap();
        let check_val: Value = serde_json::from_str(check_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        let warnings = check_val["hygiene_warnings"].as_array().unwrap();
        assert!(!warnings.is_empty());
        assert!(warnings[0].as_str().unwrap().contains("KB Linter"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_mcp_list_projects_and_browse_enhancements() {
        let db = Database::open_in_memory("coll_mcp_proj", "prof_mcp_proj").unwrap();

        let meta = RecordMeta {
            paths: vec![],
            versions: vec![],
            environments: vec![],
            supersedes: None,
            issue: None,
            ..Default::default()
        };

        Queries::upsert_document(
            db.conn(),
            "doc-proj-1",
            "coll_mcp_proj",
            "projects/hyper-cli/tasks/task-01.md",
            "task",
            "Build CLI Parser",
            "Task content",
            "Task content",
            &meta,
            "chk-1",
        ).unwrap();
        db.conn().execute(
            "UPDATE documents SET kind = 'task', status = 'pending' WHERE source_id = 'doc-proj-1';",
            [],
        ).unwrap();

        Queries::upsert_document(
            db.conn(),
            "doc-proj-2",
            "coll_mcp_proj",
            "projects/hyper-cli/status.md",
            "status",
            "Hyper CLI Status",
            "Status content",
            "Status content",
            &meta,
            "chk-2",
        ).unwrap();

        let list_proj_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(50)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "list_projects",
                "arguments": {}
            })),
        };

        let list_proj_resp = McpServer::handle_request(".", db.conn(), "coll_mcp_proj", "prof_mcp_proj", list_proj_req).unwrap();
        let list_proj_val: Value = serde_json::from_str(list_proj_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        let proj_arr = list_proj_val.as_array().unwrap();
        assert_eq!(proj_arr.len(), 1);
        assert_eq!(proj_arr[0]["name"], "hyper-cli");
        assert_eq!(proj_arr[0]["tasks_pending"], 1);
        assert_eq!(proj_arr[0]["has_status_doc"], true);

        let browse_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(51)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "browse",
                "arguments": {
                    "project": "hyper-cli",
                    "kind": "task",
                    "status": "pending"
                }
            })),
        };

        let browse_resp = McpServer::handle_request(".", db.conn(), "coll_mcp_proj", "prof_mcp_proj", browse_req).unwrap();
        let browse_val: Value = serde_json::from_str(browse_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(browse_val["total"], 1);
        assert_eq!(browse_val["documents"][0]["title"], "Build CLI Parser");
        assert_eq!(browse_val["documents"][0]["kind"], "task");
    }
}
