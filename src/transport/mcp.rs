use crate::core::{DecisionWorkflow, DirectiveWorkflow, Git, GrantStore, RiskWorkflow, SessionManager};
use crate::domain::BrowseOptions;
use crate::storage::Queries;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
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

struct SessionGuard<'a> {
    conn: &'a Connection,
    session_id: Option<String>,
}

impl<'a> Drop for SessionGuard<'a> {
    fn drop(&mut self) {
        if let Some(ref id) = self.session_id {
            // Check if this connection performed 0 tool calls
            if let Ok(Some(sess)) = Queries::get_session(self.conn, id) {
                if sess.total_tool_calls == 0 {
                    // Ephemeral connection with 0 tool calls: prune ghost session
                    let _ = self.conn.execute("DELETE FROM session_events WHERE session_id = ?1;", [id]);
                    let _ = self.conn.execute("DELETE FROM agent_sessions WHERE id = ?1;", [id]);
                    return;
                }
            }
            // Mark session as idle with ended_at = now, keeping it ready for resumption if agent reconnects
            let now = chrono::Utc::now().to_rfc3339();
            let _ = self.conn.execute(
                "UPDATE agent_sessions SET status = 'idle', ended_at = ?2 WHERE id = ?1;",
                rusqlite::params![id, now],
            );
        }
    }
}

fn detect_project_from_path(path: &str) -> Option<String> {
    let clean = path.trim_start_matches("./").trim_start_matches('/');
    if let Some(rest) = clean.strip_prefix("projects/") {
        let seg = rest.split('/').next()?;
        if !seg.is_empty() {
            return Some(seg.to_string());
        }
    }
    None
}

impl McpServer {
    /// Runs the stdio MCP server loop, processing JSON-RPC 2.0 messages from stdin and replying on stdout.
    pub fn run_stdio<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        profile_id: &str,
    ) -> std::io::Result<()> {
        Self::run_stdio_with_agent(root, conn, collection_id, profile_id, None, None, None)
    }

    pub fn run_stdio_with_agent<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        profile_id: &str,
        cli_agent: Option<&str>,
        cli_model: Option<&str>,
        cli_project: Option<&str>,
    ) -> std::io::Result<()> {
        let root = root.as_ref();
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let reader = std::io::BufReader::new(stdin.lock());

        let initial_agent = Self::resolve_initial_agent(cli_agent, cli_model);

        // Resume active/recent session (10m sliding window) or initialize new one
        let active_session = SessionManager::start_or_resume_session(
            conn,
            collection_id,
            profile_id,
            &initial_agent,
            None,
            cli_project.map(|s| s.to_string()),
            600,
        ).ok();
        let active_session_id = active_session.as_ref().map(|s| s.id.clone());

        let _guard = SessionGuard {
            conn,
            session_id: active_session_id.clone(),
        };

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
                if let Some(resp) = Self::handle_request_with_session(root, conn, collection_id, profile_id, active_session_id.as_deref(), req) {
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

    fn resolve_initial_agent(cli_agent: Option<&str>, cli_model: Option<&str>) -> String {
        let agent_name = cli_agent
            .map(|s| s.to_string())
            .or_else(|| std::env::var("HYPERKB_AGENT").ok())
            .or_else(|| std::env::var("OPENCODE_CLIENT").ok())
            .unwrap_or_else(|| "opencode".to_string());

        let model = cli_model
            .map(|s| s.to_string())
            .or_else(|| std::env::var("OPENCODE_MODEL").ok())
            .or_else(|| std::env::var("HYPERKB_MODEL").ok())
            .or_else(|| std::env::var("MODEL").ok())
            .or_else(|| std::env::var("LLM_MODEL").ok())
            .or_else(|| std::env::var("ANTHROPIC_MODEL").ok())
            .or_else(|| std::env::var("OPENAI_MODEL").ok());

        match model {
            Some(m) if !m.is_empty() => format!("{} ({})", agent_name, m),
            _ => agent_name,
        }
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
            "initialize" => {
                if let Some(sess_id) = session_id {
                    let client_name = req.params.as_ref()
                        .and_then(|p| p.get("clientInfo"))
                        .and_then(|c| c.get("name"))
                        .and_then(|n| n.as_str());
                    if let Some(cname) = client_name {
                        let model = std::env::var("OPENCODE_MODEL")
                            .or_else(|_| std::env::var("HYPERKB_MODEL"))
                            .or_else(|_| std::env::var("MODEL"))
                            .or_else(|_| std::env::var("LLM_MODEL"))
                            .or_else(|_| std::env::var("ANTHROPIC_MODEL"))
                            .or_else(|_| std::env::var("OPENAI_MODEL"))
                            .ok();
                        let agent_tag = match model {
                            Some(m) if !m.is_empty() => format!("{} ({})", cname, m),
                            _ => cname.to_string(),
                        };
                        let _ = SessionManager::set_agent_id(conn, sess_id, &agent_tag);
                    }
                }
                Some(JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: Some(json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {},
                            "prompts": {
                                "listChanged": false
                            }
                        },
                        "serverInfo": {
                            "name": "hyperkb",
                            "version": "0.1.0"
                        }
                    })),
                    error: None,
                })
            }

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

            "prompts/list" => Some(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({
                    "prompts": Self::prompt_definitions()
                })),
                error: None,
            }),

            "prompts/get" => {
                let result = Self::handle_prompt_get(root, conn, collection_id, profile_id, session_id, req.params);
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
                "description": "Before editing or proposing changes, check planned files against cited open risks and architectural boundaries. If 'files' is omitted or empty, HyperKB automatically inspects modified, staged, and untracked files via Git diff and status.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional list of root-relative file paths to check. If omitted or empty, automatically inspects modified, staged, and untracked files in the Git working tree."
                        },
                        "version": {
                            "type": "string",
                            "description": "Optional exact target version label (e.g. v2.0)"
                        },
                        "environment": {
                            "type": "string",
                            "description": "Optional target deployment environment (e.g. production, staging)"
                        }
                    }
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
                        },
                        "include_archived": {
                            "type": "boolean",
                            "description": "Include tombstoned and archived documents (default: false)"
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
                        },
                        "include_archived": {
                            "type": "boolean",
                            "description": "Include tombstoned and archived documents (default: false)"
                        }
                    }
                }
            }),
            json!({
                "name": "list_projects",
                "description": "List all segregated projects in the repository with health, task counts, open risks, and status documentation coverage.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "include_archived": {
                            "type": "boolean",
                            "description": "Include archived projects and tombstoned directories (default: false)"
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
                        },
                        "model": {
                            "type": "string",
                            "description": "Optional LLM model identifier (e.g. 'claude-3-7-sonnet', 'gpt-4o') to register in the session taxonomy"
                        },
                        "agent_id": {
                            "type": "string",
                            "description": "Optional agent or harness identifier (e.g. 'opencode', 'cursor')"
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
            json!({
                "name": "transition_task",
                "description": "Atomically transition the state of a project task (pending, in_progress, completed, blocked) and update the project's active critical path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Project name (e.g. data-load-throughput)" },
                        "task_id": { "type": "string", "description": "Task identifier (e.g. task-01)" },
                        "status": { "type": "string", "description": "Target status: pending, in_progress, completed, blocked" },
                        "reason": { "type": "string", "description": "Optional rationale or evidence for transition" }
                    },
                    "required": ["project", "task_id", "status"]
                }
            }),
            json!({
                "name": "update_status",
                "description": "Atomically transition a project's health (healthy, at_risk, blocked), record blockers, and update status.md without modifying markdown body.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Project name (e.g. adaptive-resource-tuning)" },
                        "health": { "type": "string", "description": "Health state: healthy, at_risk, blocked" },
                        "blocker": { "type": "string", "description": "Optional description of a blocker" },
                        "reason": { "type": "string", "description": "Optional rationale for the health change" }
                    },
                    "required": ["project", "health"]
                }
            }),
            json!({
                "name": "get_project_status",
                "description": "Get strongly-typed status document for a project, including health, goal, active task, blockers, and exit criteria.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Project name (e.g. jwt-hotreload)" }
                    },
                    "required": ["project"]
                }
            }),
            json!({
                "name": "defer_finding",
                "description": "Divert an adjacent bug, technical debt, or side-quest finding into the project's BACKLOG.md to prevent context thrashing and stay on the Critical Path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Project name (e.g. data-load-throughput)" },
                        "title": { "type": "string", "description": "Title of the deferred finding or technical debt" },
                        "details": { "type": "string", "description": "Specific details or code citations" },
                        "severity": { "type": "string", "description": "Optional severity: low, medium, high, debt (default: debt)" }
                    },
                    "required": ["project", "title", "details"]
                }
            }),
            json!({
                "name": "verify_exit_criteria",
                "description": "Execute the project's exit criteria command and automatically transition the project to completed if exit code matches.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Project name" }
                    },
                    "required": ["project"]
                }
            }),
            json!({
                "name": "consult_peer_model",
                "description": "Consult a peer AI model (Claude, ChatGPT, or Gemini) as middleware for cross-checks, specialized queries, or second opinions. Returns the peer's response, token latency, and records the interaction in the HyperKB session audit ledger.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "peer": {
                            "type": "string",
                            "description": "Target peer model: 'claude', 'chatgpt', or 'gemini'"
                        },
                        "prompt": {
                            "type": "string",
                            "description": "Question, instructions, or critique request for the peer model"
                        },
                        "context_files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional list of root-relative file paths to attach as context (e.g. ['src/storage/db.rs'])"
                        }
                    },
                    "required": ["peer", "prompt"]
                }
            }),
        ]
    }

    pub fn prompt_definitions() -> Vec<Value> {
        vec![
            json!({
                "name": "hkb_brief",
                "description": "Warm-start context briefing with rule-of-5 guardrails and locked critical path",
                "arguments": [
                    {
                        "name": "scope",
                        "description": "Optional project scope or grant pattern (e.g. projects/platform-shell)",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "hkb_status",
                "description": "Current project health, active task lock, blockers, and exit criteria status",
                "arguments": [
                    {
                        "name": "project",
                        "description": "Project directory name (e.g. platform-shell). Defaults to active project if omitted.",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "hkb_verify",
                "description": "Execute deterministic exit criteria verification for a project",
                "arguments": [
                    {
                        "name": "project",
                        "description": "Project directory name (e.g. platform-shell). Defaults to active project if omitted.",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "hkb_task_next",
                "description": "Advance or transition the critical path task for a project (e.g. mark completed, in_progress, or blocked)",
                "arguments": [
                    {
                        "name": "project",
                        "description": "Project directory name. Defaults to active project if omitted.",
                        "required": false
                    },
                    {
                        "name": "task_id",
                        "description": "Task ID to transition (e.g. task-01). Defaults to current active task if omitted.",
                        "required": false
                    },
                    {
                        "name": "status",
                        "description": "Target status: 'completed', 'in_progress', or 'blocked'. Defaults to 'completed'.",
                        "required": false
                    },
                    {
                        "name": "reason",
                        "description": "Optional transition rationale or progress note.",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "hkb_defer",
                "description": "Jail a tangential finding or bug into the project backlog to protect the critical path",
                "arguments": [
                    {
                        "name": "title",
                        "description": "Title or summary of the finding to defer",
                        "required": true
                    },
                    {
                        "name": "details",
                        "description": "Details or reproduction steps",
                        "required": false
                    },
                    {
                        "name": "project",
                        "description": "Target project directory. Defaults to active project if omitted.",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "hkb_metrics",
                "description": "View session effectiveness scorecard, coding velocity, tool-to-edit ratio, and review loops",
                "arguments": [
                    {
                        "name": "session_id",
                        "description": "Optional specific session ID. Defaults to current active session.",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "hkb_claude",
                "description": "Consult Anthropic Claude peer model for a second opinion, architecture critique, or design feedback",
                "arguments": [
                    {
                        "name": "prompt",
                        "description": "Question, instructions, or critique request for Claude",
                        "required": true
                    },
                    {
                        "name": "context_files",
                        "description": "Optional file paths to attach as context (comma-separated or array)",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "hkb_chatgpt",
                "description": "Consult OpenAI ChatGPT peer model for a second opinion, code generation, or review",
                "arguments": [
                    {
                        "name": "prompt",
                        "description": "Question, instructions, or critique request for ChatGPT",
                        "required": true
                    },
                    {
                        "name": "context_files",
                        "description": "Optional file paths to attach as context (comma-separated or array)",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "hkb_gemini",
                "description": "Consult Google Gemini peer model for multimodal analysis, long-context reasoning, or review",
                "arguments": [
                    {
                        "name": "prompt",
                        "description": "Question, instructions, or critique request for Gemini",
                        "required": true
                    },
                    {
                        "name": "context_files",
                        "description": "Optional file paths to attach as context (comma-separated or array)",
                        "required": false
                    }
                ]
            }),
        ]
    }

    fn resolve_project(
        root: &Path,
        conn: &Connection,
        collection_id: &str,
        session_id: Option<&str>,
        explicit: Option<&str>,
    ) -> Result<String, String> {
        if let Some(p) = explicit {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                let clean = trimmed.strip_prefix("projects/").unwrap_or(trimmed);
                let clean = clean.trim_matches('/');
                return Ok(clean.to_string());
            }
        }
        if let Some(sess_id) = session_id {
            if let Ok(Some(sess)) = Queries::get_session(conn, sess_id) {
                if let Some(ref proj) = sess.project {
                    if !proj.is_empty() && proj != "Unscoped" {
                        return Ok(proj.clone());
                    }
                }
            }
        }
        let projects = Queries::list_projects(conn, collection_id, false).unwrap_or_default();
        if projects.len() == 1 {
            return Ok(projects[0].name.clone());
        }
        let active: Vec<_> = projects
            .iter()
            .filter(|p| p.tasks_in_progress > 0 || p.tasks_pending > 0 || p.active_task.is_some())
            .collect();
        if active.len() == 1 {
            return Ok(active[0].name.clone());
        }
        if let Some(first) = projects.first() {
            return Ok(first.name.clone());
        }
        let projects_dir = root.join("projects");
        if projects_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&projects_dir) {
                let subdirs: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|name| !name.starts_with('.') && !name.starts_with('_'))
                    .collect();
                if subdirs.len() == 1 {
                    return Ok(subdirs[0].clone());
                }
            }
        }
        Err("No project specified and no active project discovered in workspace. Pass project argument (e.g. project: 'platform-shell').".to_string())
    }

    fn handle_prompt_get(
        root: &Path,
        conn: &Connection,
        collection_id: &str,
        _profile_id: &str,
        session_id: Option<&str>,
        params: Option<Value>,
    ) -> Result<Value, String> {
        let params = params.ok_or_else(|| "Missing params for prompts/get".to_string())?;
        let raw_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter 'name' for prompts/get".to_string())?;
        let name = raw_name
            .strip_prefix("hkb_")
            .or_else(|| raw_name.strip_prefix("hkb-"))
            .unwrap_or(raw_name);
        let args = params.get("arguments").and_then(|v| v.as_object());

        match name {
            "brief" => {
                let scope = args
                    .and_then(|a| a.get("scope"))
                    .and_then(|v| v.as_str());
                let manifest = crate::domain::RepoManifest::load_or_default(root);
                let briefing = SessionManager::generate_briefing_with_limit(
                    conn,
                    collection_id,
                    scope,
                    manifest.settings.max_briefing_directives,
                ).map_err(|e| format!("Failed to generate briefing: {}", e))?;

                if let Some(sess_id) = session_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "prompt/brief", "", "{}");
                }

                Ok(json!({
                    "description": "Warm-start context briefing with rule-of-5 guardrails and locked critical path",
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": briefing.formatted_markdown
                            }
                        }
                    ]
                }))
            }

            "status" => {
                let explicit_project = args
                    .and_then(|a| a.get("project"))
                    .and_then(|v| v.as_str());
                let project = Self::resolve_project(root, conn, collection_id, session_id, explicit_project)?;

                if let Some(sess_id) = session_id {
                    let _ = SessionManager::set_project(conn, sess_id, &project);
                    let _ = SessionManager::record_tool_call(conn, sess_id, "prompt/status", &format!("projects/{}/status.md", project), "");
                }

                let st = crate::core::StatusEngine::get_project_status(root, &project)
                    .map_err(|e| format!("Error reading status for project '{}': {}", project, e))?;

                let mut text = format!("# Project Status: {}\n\n", project);
                text.push_str(&format!("- **Status**: `{}`\n", st.status.as_str()));
                text.push_str(&format!("- **Health**: `{}`\n", st.health.as_str()));
                if let Some(ref at) = st.active_task {
                    text.push_str(&format!("- **Active Critical Path Task**: 🔒 `{}`\n", at));
                } else {
                    text.push_str("- **Active Critical Path Task**: *(None active / unblocked)*\n");
                }
                text.push_str(&format!("- **Goal**: {}\n", st.goal));
                if let Some(ref ec) = st.exit_criteria {
                    text.push_str(&format!("- **Exit Criteria**: `{}` (expected exit code: {})\n", ec.command, ec.expected_exit_code));
                }
                if !st.blockers.is_empty() {
                    text.push_str("\n### Blockers:\n");
                    for b in &st.blockers {
                        let icon = if b.resolved { "✔ [RESOLVED]" } else { "✖ [OPEN]" };
                        text.push_str(&format!("- {} `{}`: {}\n", icon, b.id, b.description));
                    }
                }

                Ok(json!({
                    "description": format!("Current status, critical path, and health for project '{}'", project),
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": text
                            }
                        }
                    ]
                }))
            }

            "verify" => {
                let explicit_project = args
                    .and_then(|a| a.get("project"))
                    .and_then(|v| v.as_str());
                let project = Self::resolve_project(root, conn, collection_id, session_id, explicit_project)?;

                if let Some(sess_id) = session_id {
                    let _ = SessionManager::set_project(conn, sess_id, &project);
                    let _ = SessionManager::record_tool_call(conn, sess_id, "prompt/verify", &format!("projects/{}", project), "");
                }

                let res = crate::core::StatusEngine::verify_exit_criteria(root, &project)
                    .map_err(|e| format!("Exit criteria verification error for '{}': {}", project, e))?;

                let mut text = format!("# Exit Criteria Verification: {}\n\n", project);
                if res.passed {
                    text.push_str(&format!("✅ **PASSED** (Exit code: {})\n", res.exit_code));
                } else {
                    text.push_str(&format!("❌ **FAILED** (Exit code: {})\n", res.exit_code));
                }
                text.push_str(&format!("- **Command Executed**: `{}`\n", res.command));
                if !res.stdout.trim().is_empty() {
                    text.push_str(&format!("\n**Standard Output**:\n```\n{}\n```\n", res.stdout.trim()));
                }
                if !res.stderr.trim().is_empty() {
                    text.push_str(&format!("\n**Standard Error**:\n```\n{}\n```\n", res.stderr.trim()));
                }

                Ok(json!({
                    "description": format!("Exit criteria verification outcome for '{}'", project),
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": text
                            }
                        }
                    ]
                }))
            }

            "task_next" => {
                let explicit_project = args
                    .and_then(|a| a.get("project"))
                    .and_then(|v| v.as_str());
                let project = Self::resolve_project(root, conn, collection_id, session_id, explicit_project)?;
                let status_arg = args
                    .and_then(|a| a.get("status"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("completed");
                let explicit_task_id = args
                    .and_then(|a| a.get("task_id"))
                    .and_then(|v| v.as_str());
                let reason = args
                    .and_then(|a| a.get("reason"))
                    .and_then(|v| v.as_str());

                let st = crate::core::StatusEngine::get_project_status(root, &project)
                    .map_err(|e| format!("Error reading status for '{}': {}", project, e))?;

                let target_task_id = match explicit_task_id {
                    Some(id) if !id.trim().is_empty() => id.trim().to_string(),
                    _ => match st.active_task {
                        Some(ref at) => at.clone(),
                        None => return Err(format!("No active task currently found in project '{}' to transition. Provide 'task_id'.", project)),
                    },
                };

                let target_status = match status_arg.to_lowercase().as_str() {
                    "completed" | "complete" | "done" => crate::domain::schema::TaskState::Completed,
                    "in_progress" | "progress" | "start" => crate::domain::schema::TaskState::InProgress,
                    "blocked" | "block" => crate::domain::schema::TaskState::Blocked,
                    "pending" | "reset" => crate::domain::schema::TaskState::Pending,
                    other => return Err(format!("Invalid task status '{}'. Supported: completed, in_progress, blocked, pending", other)),
                };

                let updated_task = crate::core::StatusEngine::transition_task(
                    root,
                    &project,
                    &target_task_id,
                    target_status,
                    reason,
                ).map_err(|e| format!("Failed to transition task: {}", e))?;

                let manifest = crate::domain::RepoManifest::load_or_default(root);
                let _ = crate::core::Scanner::index_workspace(conn, root, &manifest);

                if let Some(sess_id) = session_id {
                    let _ = SessionManager::set_project(conn, sess_id, &project);
                    let _ = SessionManager::record_tool_call(
                        conn,
                        sess_id,
                        "prompt/task_next",
                        &format!("projects/{}/tasks/{}.md", project, target_task_id),
                        status_arg,
                    );
                }

                let mut text = format!("# Task Transition: {}\n\n", project);
                text.push_str(&format!("- **Task ID**: `{}`\n", updated_task.id));
                text.push_str(&format!("- **New Status**: `{}`\n", updated_task.status.as_str()));
                if let Some(ref r) = updated_task.reason {
                    text.push_str(&format!("- **Reason**: {}\n", r));
                }
                if let Ok(updated_st) = crate::core::StatusEngine::get_project_status(root, &project) {
                    if let Some(ref next_task) = updated_st.active_task {
                        text.push_str(&format!("- **Next Critical Path Lock**: 🔒 `{}`\n", next_task));
                    } else {
                        text.push_str("- **Next Critical Path Lock**: *(None remaining / All tasks completed)*\n");
                    }
                }

                Ok(json!({
                    "description": format!("Transitioned task '{}' to '{}' in project '{}'", target_task_id, updated_task.status.as_str(), project),
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": text
                            }
                        }
                    ]
                }))
            }

            "defer" => {
                let explicit_project = args
                    .and_then(|a| a.get("project"))
                    .and_then(|v| v.as_str());
                let project = Self::resolve_project(root, conn, collection_id, session_id, explicit_project)?;
                let title = args
                    .and_then(|a| a.get("title"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'title' for defer prompt".to_string())?;
                let details = args
                    .and_then(|a| a.get("details"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let severity = args
                    .and_then(|a| a.get("severity"))
                    .and_then(|v| v.as_str());

                let result = crate::core::StatusEngine::defer_finding(
                    root,
                    &project,
                    title,
                    details,
                    severity,
                ).map_err(|e| format!("Failed to defer finding: {}", e))?;

                if let Some(sess_id) = session_id {
                    let _ = SessionManager::set_project(conn, sess_id, &project);
                    let _ = SessionManager::record_tool_call(
                        conn,
                        sess_id,
                        "prompt/defer",
                        &format!("projects/{}/BACKLOG.md", project),
                        title,
                    );
                }

                let instruction = format!(
                    "Finding '{}' ({}) recorded in 'projects/{}/BACKLOG.md'. Tangential scope creep is strictly prohibited: you MUST return immediately to the single-slot critical path task.",
                    title, result, project
                );

                let mut text = format!("# Finding Deferred to Backlog: {}\n\n", project);
                text.push_str(&format!("- **Title**: {}\n", title));
                text.push_str(&format!("- **Backlog File**: `projects/{}/BACKLOG.md`\n", project));
                text.push_str(&format!("- **Instruction**: {}\n", instruction));

                Ok(json!({
                    "description": format!("Deferred tangential finding '{}' to {} backlog", title, project),
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": text
                            }
                        }
                    ]
                }))
            }

            "metrics" => {
                let target_id = args
                    .and_then(|a| a.get("session_id"))
                    .and_then(|v| v.as_str())
                    .or(session_id)
                    .ok_or_else(|| "No active session ID found to generate metrics".to_string())?;

                let sc = SessionManager::compute_scorecard(conn, target_id)
                    .map_err(|e| format!("Failed to compute scorecard: {}", e))?;

                let mut text = format!("# Session Scorecard: `{}`\n\n", sc.session_id);
                text.push_str(&format!("- **Coding Effectiveness**: {:.0}%\n", sc.coding_effectiveness * 100.0));
                text.push_str(&format!("- **Knowledge Hit Rate**: {:.0}%\n", sc.knowledge_hit_rate * 100.0));
                text.push_str(&format!("- **Tool-to-Edit Ratio**: {:.1} (Healthy benchmark: 2.0 – 4.0)\n", sc.tool_to_edit_ratio));
                text.push_str(&format!("- **Review Loops Detected**: {}\n", sc.review_loops_detected));
                text.push_str(&format!("- **Risks Handled / Prevented**: {}\n", sc.risks_handled));
                text.push_str(&format!("- **Session Duration**: {}s\n", sc.duration_seconds));
                if !sc.friction_hotspots.is_empty() {
                    text.push_str("\n### Friction Hotspots (Re-edited files):\n");
                    for p in &sc.friction_hotspots {
                        text.push_str(&format!("- `{}`\n", p));
                    }
                }

                Ok(json!({
                    "description": format!("Telemetry and quality scorecard for session '{}'", target_id),
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": text
                            }
                        }
                    ]
                }))
            }

            "claude" | "chatgpt" | "gemini" => {
                let prompt = args
                    .and_then(|a| a.get("prompt"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("Missing required argument 'prompt' for {} prompt", name))?;

                let context_files: Option<Vec<String>> = args.and_then(|a| a.get("context_files")).and_then(|v| {
                    if let Some(arr) = v.as_array() {
                        Some(arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                    } else if let Some(s) = v.as_str() {
                        Some(s.split(',').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect())
                    } else {
                        None
                    }
                });

                if let Some(sess_id) = session_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, &format!("prompt/{}", name), "", prompt);
                }

                let res = crate::core::ModelRouter::consult(root, name, prompt, context_files.as_deref());
                let mut text = format!("### {} Peer Consultation ({})\n\n", name.to_uppercase(), res.model);
                if res.success {
                    text.push_str(&res.response);
                    text.push_str(&format!("\n\n*Latency: {}ms | Source: {}*", res.elapsed_ms, res.source));
                } else {
                    text.push_str(&format!("⚠️ **Notice**: {}\n", res.response));
                }

                Ok(json!({
                    "description": format!("Peer consultation response from {}", name),
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": text
                            }
                        }
                    ]
                }))
            }

            other => Err(format!("Unknown prompt '{}'. Available: brief, status, verify, task_next, defer, metrics, claude, chatgpt, gemini", other)),
        }
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
        SessionManager::start_session(conn, collection_id, profile_id, "agent_mcp", None, None)
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
                let mut files: Vec<String> = args
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

                let mut auto_detected = false;
                if files.is_empty() {
                    // Auto-diff: automatically query git for modified, staged, and untracked files
                    if let Ok(git_files) = Git::get_modified_and_untracked_files(root) {
                        files = git_files
                            .into_iter()
                            .map(|f| crate::core::RiskEngine::normalize_path(&f, Some(root)))
                            .filter(|f| !f.is_empty())
                            .collect();
                        files.sort();
                        files.dedup();
                        auto_detected = true;
                    }
                }

                if files.is_empty() {
                    return Ok(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": "{\"clean\": true, \"message\": \"No files provided and no modified, staged, or untracked files detected in Git working tree.\", \"applicable_directives\": [], \"matches\": [], \"hygiene_warnings\": []}"
                            }
                        ],
                        "isError": false
                    }));
                }

                let version = args.get("version").and_then(|v| v.as_str());
                let env = args.get("environment").and_then(|v| v.as_str());

                let mut check = Queries::check_work(conn, collection_id, &files, version, env)
                    .map_err(|e| format!("Failed to check work: {}", e))?;

                if auto_detected {
                    for m in &mut check.matches {
                        if !m.acknowledged {
                            let all_trivial = m.matched_paths.iter().all(|path| {
                                Git::file_diff_is_trivial(root, path, false).unwrap_or(false)
                            });
                            if all_trivial && !m.matched_paths.is_empty() {
                                m.suppressed = true;
                                m.suppression_reason = Some(
                                    "Diff contains only comments or whitespace (suppressed to prevent alert fatigue)".to_string(),
                                );
                            }
                        }
                    }
                }

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
                    if let Some(proj) = files.iter().find_map(|f| detect_project_from_path(f)) {
                        let _ = SessionManager::set_project(conn, sess_id, &proj);
                    }
                    let _ = SessionManager::record_tool_call(conn, sess_id, "check_work", first_file, "{}");
                    for m in &check.matches {
                        if !m.suppressed {
                            let _ = SessionManager::record_risk_cited(conn, sess_id, first_file, &m.document.title);
                        }
                    }

                    // Automatically sync real Git working tree diff volume
                    let (added, deleted) = Git::get_working_tree_diff_lines(root);
                    let diff_lines = added + deleted;
                    if diff_lines > 0 {
                        let _ = SessionManager::sync_diff_volume(conn, sess_id, diff_lines, files.len() as u32);
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

                let include_archived = args.get("include_archived").and_then(|v| v.as_bool()).unwrap_or(false);

                // Note: include_private = false strictly withholds private memory from agents!
                let hits = Queries::search(
                    conn,
                    &[collection_id.to_string()],
                    profile_id,
                    query,
                    limit,
                    false,
                    include_archived,
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
                let include_archived = args.get("include_archived").and_then(|v| v.as_bool()).unwrap_or(false);

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
                    include_archived,
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
                let include_archived = args.get("include_archived").and_then(|v| v.as_bool()).unwrap_or(false);

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(conn, sess_id, "list_projects", "", "");
                }

                let projects = Queries::list_projects(conn, collection_id, include_archived)
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
                    if let Some(project) = args.get("project").and_then(|v| v.as_str()) {
                        let _ = SessionManager::set_project(conn, sess_id, project);
                    }
                    if let Some(model) = args.get("model").and_then(|v| v.as_str()) {
                        if let Ok(Some(existing)) = Queries::get_session(conn, sess_id) {
                            let (harness, _) = existing.parse_agent_taxonomy();
                            let new_tag = format!("{} ({})", harness, model);
                            let _ = SessionManager::set_agent_id(conn, sess_id, &new_tag);
                        }
                    } else if let Some(agent) = args.get("agent_id").and_then(|v| v.as_str()) {
                        let _ = SessionManager::set_agent_id(conn, sess_id, agent);
                    }
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

                if let Some(proj) = detect_project_from_path(path) {
                    let _ = SessionManager::set_project(conn, sess_id, &proj);
                }

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

            "transition_task" => {
                let project = args
                    .get("project")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'project'".to_string())?;
                let task_id = args
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'task_id'".to_string())?;
                let status_str = args
                    .get("status")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'status'".to_string())?;
                let reason = args.get("reason").and_then(|v| v.as_str());

                let to_state = crate::domain::TaskState::from_str_loose(status_str);

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(
                        conn,
                        sess_id,
                        "transition_task",
                        &format!("projects/{}/tasks/{}", project, task_id),
                        &format!("status={}", status_str),
                    );
                }

                match crate::core::StatusEngine::transition_task(
                    root,
                    project,
                    task_id,
                    to_state,
                    reason,
                ) {
                    Ok(task) => {
                        let manifest = crate::domain::RepoManifest::load_or_default(root);
                        let _ = crate::core::Scanner::index_workspace(conn, root, &manifest);
                        let serialized = serde_json::to_string_pretty(&task)
                            .map_err(|e| format!("Serialization error: {}", e))?;
                        Ok(json!({
                            "content": [{ "type": "text", "text": serialized }],
                            "isError": false
                        }))
                    }
                    Err(err) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Error transitioning task: {}", err) }],
                        "isError": true
                    })),
                }
            }

            "update_status" => {
                let project = args
                    .get("project")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'project'".to_string())?;
                let health_str = args
                    .get("health")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'health'".to_string())?;
                let blocker = args.get("blocker").and_then(|v| v.as_str());
                let reason = args.get("reason").and_then(|v| v.as_str());

                let health_state = crate::domain::HealthState::from_str_loose(health_str);

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(
                        conn,
                        sess_id,
                        "update_status",
                        &format!("projects/{}/status.md", project),
                        &format!("health={}", health_str),
                    );
                }

                match crate::core::StatusEngine::transition_project_health(
                    root,
                    project,
                    health_state,
                    reason,
                    blocker,
                ) {
                    Ok(st) => {
                        let manifest = crate::domain::RepoManifest::load_or_default(root);
                        let _ = crate::core::Scanner::index_workspace(conn, root, &manifest);
                        let serialized = serde_json::to_string_pretty(&st)
                            .map_err(|e| format!("Serialization error: {}", e))?;
                        Ok(json!({
                            "content": [{ "type": "text", "text": serialized }],
                            "isError": false
                        }))
                    }
                    Err(err) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Error updating status: {}", err) }],
                        "isError": true
                    })),
                }
            }

            "get_project_status" => {
                let project = args
                    .get("project")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'project'".to_string())?;

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::set_project(conn, sess_id, project);
                    let _ = SessionManager::record_tool_call(
                        conn,
                        sess_id,
                        "get_project_status",
                        &format!("projects/{}/status.md", project),
                        "",
                    );
                }

                match crate::core::StatusEngine::get_project_status(root, project) {
                    Ok(st) => {
                        let serialized = serde_json::to_string_pretty(&st)
                            .map_err(|e| format!("Serialization error: {}", e))?;
                        Ok(json!({
                            "content": [{ "type": "text", "text": serialized }],
                            "isError": false
                        }))
                    }
                    Err(err) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Error reading project status: {}", err) }],
                        "isError": true
                    })),
                }
            }

            "defer_finding" => {
                let project = args
                    .get("project")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'project'".to_string())?;
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'title'".to_string())?;
                let details = args
                    .get("details")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'details'".to_string())?;
                let severity = args.get("severity").and_then(|v| v.as_str());

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::set_project(conn, sess_id, project);
                    let _ = SessionManager::record_tool_call(
                        conn,
                        sess_id,
                        "defer_finding",
                        &format!("projects/{}/BACKLOG.md", project),
                        title,
                    );
                }

                match crate::core::StatusEngine::defer_finding(root, project, title, details, severity) {
                    Ok(finding_id) => {
                        let reply = json!({
                            "finding_id": finding_id,
                            "project": project,
                            "file": format!("projects/{}/BACKLOG.md", project),
                            "instruction": "Finding recorded to project backlog. You are strictly prohibited from addressing this now. Return immediately to the active Critical Path task."
                        });
                        Ok(json!({
                            "content": [{ "type": "text", "text": serde_json::to_string_pretty(&reply).unwrap_or_default() }],
                            "isError": false
                        }))
                    }
                    Err(err) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Error deferring finding: {}", err) }],
                        "isError": true
                    })),
                }
            }

            "verify_exit_criteria" => {
                let project = args
                    .get("project")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'project'".to_string())?;

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::set_project(conn, sess_id, project);
                    let _ = SessionManager::record_tool_call(
                        conn,
                        sess_id,
                        "verify_exit_criteria",
                        &format!("projects/{}/status.md", project),
                        "",
                    );
                }

                match crate::core::StatusEngine::verify_exit_criteria(root, project) {
                    Ok(res) => {
                        if res.passed {
                            let manifest = crate::domain::RepoManifest::load_or_default(root);
                            let _ = crate::core::Scanner::index_workspace(conn, root, &manifest);
                        }
                        let serialized = serde_json::to_string_pretty(&res)
                            .map_err(|e| format!("Serialization error: {}", e))?;
                        Ok(json!({
                            "content": [{ "type": "text", "text": serialized }],
                            "isError": !res.passed
                        }))
                    }
                    Err(err) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("Error verifying exit criteria: {}", err) }],
                        "isError": true
                    })),
                }
            }

            "consult_peer_model" => {
                let peer = args
                    .get("peer")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'peer' for consult_peer_model".to_string())?;
                let prompt = args
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required argument 'prompt' for consult_peer_model".to_string())?;
                let context_files: Option<Vec<String>> = args
                    .get("context_files")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    });

                if let Some(ref sess_id) = current_sess_id {
                    let _ = SessionManager::record_tool_call(
                        conn,
                        sess_id,
                        &format!("consult_peer_model:{}", peer),
                        "",
                        prompt,
                    );
                }

                let result = crate::core::ModelRouter::consult(
                    root,
                    peer,
                    prompt,
                    context_files.as_deref(),
                );

                let serialized = serde_json::to_string_pretty(&result)
                    .map_err(|e| format!("Serialization error: {}", e))?;

                Ok(json!({
                    "content": [{ "type": "text", "text": serialized }],
                    "isError": !result.success
                }))
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
    fn test_mcp_check_work_auto_diff() {
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();

        // 1. In a repo with modified files, check_work with no files auto-detects diffs
        let temp_git = std::env::temp_dir().join(format!("hyperkb-git-diff-{}", uuid::Uuid::now_v7()));
        let _ = std::fs::create_dir_all(&temp_git);
        let _ = std::process::Command::new("git").arg("init").current_dir(&temp_git).output();
        let _ = std::process::Command::new("git").args(["config", "user.name", "Test"]).current_dir(&temp_git).output();
        let _ = std::process::Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(&temp_git).output();
        let dummy_file = temp_git.join("modified.txt");
        let _ = std::fs::write(&dummy_file, "initial content\n");
        let _ = std::process::Command::new("git").args(["add", "."]).current_dir(&temp_git).output();
        let _ = std::process::Command::new("git").args(["commit", "-m", "initial"]).current_dir(&temp_git).output();
        let _ = std::fs::write(&dummy_file, "modified content\n");

        let call_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(31)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "check_work",
                "arguments": {}
            })),
        };

        let call_resp = McpServer::handle_request(&temp_git, db.conn(), "coll_test", "prof_test", call_req.clone()).unwrap();
        let content_text = call_resp.result.unwrap()["content"][0]["text"].as_str().unwrap().to_string();
        assert!(content_text.contains("checked_paths"));
        let _ = std::fs::remove_dir_all(&temp_git);

        // 2. In a clean directory with no git changes, check_work returns clean message
        let temp_clean = std::env::temp_dir().join(format!("hyperkb-clean-{}", uuid::Uuid::now_v7()));
        let _ = std::fs::create_dir_all(&temp_clean);
        let call_resp_clean = McpServer::handle_request(&temp_clean, db.conn(), "coll_test", "prof_test", call_req).unwrap();
        let content_clean = call_resp_clean.result.unwrap()["content"][0]["text"].as_str().unwrap().to_string();
        assert!(content_clean.contains("No files provided"));
        let _ = std::fs::remove_dir_all(&temp_clean);
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
            false,
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
            false,
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

    #[test]
    fn test_mcp_task_and_status_tools() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb_test_mcp_status_{}", uuid::Uuid::now_v7()));
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();
        let root_str = temp_dir.to_str().unwrap();

        // 1. update_status tool
        let update_status_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(60)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "update_status",
                "arguments": {
                    "project": "agent-gateway",
                    "health": "blocked",
                    "blocker": "Missing upstream TLS certificate",
                    "reason": "Security requirement"
                }
            })),
        };

        let status_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", update_status_req).unwrap();
        let status_val: Value = serde_json::from_str(status_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(status_val["health"], "blocked");
        assert_eq!(status_val["status"], "blocked");
        assert_eq!(status_val["blockers"][0]["description"], "Missing upstream TLS certificate");

        // 2. transition_task tool
        let task_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(61)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "transition_task",
                "arguments": {
                    "project": "agent-gateway",
                    "task_id": "task-01",
                    "status": "in_progress",
                    "reason": "Commencing implementation of TLS cert rotation"
                }
            })),
        };

        let task_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", task_req).unwrap();
        let task_val: Value = serde_json::from_str(task_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(task_val["id"], "task-01");
        assert_eq!(task_val["status"], "in_progress");

        // 3. get_project_status tool
        let get_status_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(62)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "get_project_status",
                "arguments": {
                    "project": "agent-gateway"
                }
            })),
        };

        let get_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", get_status_req).unwrap();
        let get_val: Value = serde_json::from_str(get_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(get_val["active_task"], "task-01");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_mcp_defer_finding_and_verify_exit_criteria() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb_test_mcp_exit_{}", uuid::Uuid::now_v7()));
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();
        let root_str = temp_dir.to_str().unwrap();

        // 1. defer_finding tool
        let defer_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(70)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "defer_finding",
                "arguments": {
                    "project": "core-engine",
                    "title": "Audit OpenSSL Bindings",
                    "details": "Static analysis memory safety check",
                    "severity": "medium"
                }
            })),
        };

        let defer_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", defer_req).unwrap();
        let defer_val: Value = serde_json::from_str(defer_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert!(defer_val["instruction"].as_str().unwrap().contains("prohibited"));
        assert!(temp_dir.join("projects/core-engine/BACKLOG.md").exists());

        // 2. verify_exit_criteria tool
        let project_dir = temp_dir.join("projects/core-engine");
        let status_md = "---\nid: status-core-engine\nstatus: active\nhealth: healthy\nexit_criteria:\n  command: \"echo exit_ok\"\n  expected_exit_code: 0\n---\n# Status\n";
        std::fs::write(project_dir.join("status.md"), status_md).unwrap();

        let verify_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(71)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "verify_exit_criteria",
                "arguments": {
                    "project": "core-engine"
                }
            })),
        };

        let verify_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", verify_req).unwrap();
        let verify_val: Value = serde_json::from_str(verify_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(verify_val["passed"], true);
        assert_eq!(verify_val["exit_code"], 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_mcp_prompts_list_and_get() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb_test_mcp_prompts_{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db = Database::open_in_memory("coll_test", "prof_test").unwrap();
        let root_str = temp_dir.to_str().unwrap();

        // 1. prompts/list
        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(80)),
            method: "prompts/list".into(),
            params: None,
        };
        let list_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", list_req).unwrap();
        let prompts = list_resp.result.unwrap()["prompts"].as_array().unwrap().clone();
        let prompt_names: Vec<&str> = prompts.iter().map(|p| p["name"].as_str().unwrap()).collect();
        assert!(prompt_names.contains(&"hkb_brief"));
        assert!(prompt_names.contains(&"hkb_status"));
        assert!(prompt_names.contains(&"hkb_verify"));
        assert!(prompt_names.contains(&"hkb_task_next"));
        assert!(prompt_names.contains(&"hkb_defer"));
        assert!(prompt_names.contains(&"hkb_metrics"));
        assert!(prompt_names.contains(&"hkb_claude"));
        assert!(prompt_names.contains(&"hkb_chatgpt"));
        assert!(prompt_names.contains(&"hkb_gemini"));

        // 2. prompts/get hkb_brief (and verify legacy 'brief' fallback)
        let get_brief_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(81)),
            method: "prompts/get".into(),
            params: Some(json!({
                "name": "hkb_brief",
                "arguments": {}
            })),
        };
        let brief_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", get_brief_req).unwrap();
        let brief_msgs = brief_resp.result.unwrap()["messages"].as_array().unwrap().clone();
        assert_eq!(brief_msgs.len(), 1);
        let brief_text = brief_msgs[0]["content"]["text"].as_str().unwrap();
        assert!(brief_text.contains("Session Briefing"));

        // Also test bare "brief" fallback
        let get_legacy_brief_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(810)),
            method: "prompts/get".into(),
            params: Some(json!({
                "name": "brief",
                "arguments": {}
            })),
        };
        let leg_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", get_legacy_brief_req).unwrap();
        assert!(leg_resp.result.is_some());

        // 3. Setup project and test prompts/get hkb_status
        let proj_dir = temp_dir.join("projects/demo-proj");
        std::fs::create_dir_all(&proj_dir).unwrap();
        let status_md = "---\nid: status-demo-proj\nstatus: active\nhealth: healthy\nactive_task: task-01\ngoal: Test prompt status\n---\n# Status\n";
        std::fs::write(proj_dir.join("status.md"), status_md).unwrap();

        let get_status_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(82)),
            method: "prompts/get".into(),
            params: Some(json!({
                "name": "hkb_status",
                "arguments": {
                    "project": "demo-proj"
                }
            })),
        };
        let status_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", get_status_req).unwrap();
        let status_msgs = status_resp.result.unwrap()["messages"].as_array().unwrap().clone();
        let status_text = status_msgs[0]["content"]["text"].as_str().unwrap();
        assert!(status_text.contains("Project Status: demo-proj"));
        assert!(status_text.contains("task-01"));

        // 4. prompts/get task_next
        let task_next_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(83)),
            method: "prompts/get".into(),
            params: Some(json!({
                "name": "task_next",
                "arguments": {
                    "project": "demo-proj",
                    "task_id": "task-01",
                    "status": "completed",
                    "reason": "Shipped successfully"
                }
            })),
        };
        let task_next_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", task_next_req).unwrap();
        let task_next_msgs = task_next_resp.result.unwrap()["messages"].as_array().unwrap().clone();
        let task_next_text = task_next_msgs[0]["content"]["text"].as_str().unwrap();
        assert!(task_next_text.contains("Task Transition: demo-proj"));
        assert!(task_next_text.contains("completed"));

        // 5. prompts/get defer
        let defer_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(84)),
            method: "prompts/get".into(),
            params: Some(json!({
                "name": "defer",
                "arguments": {
                    "project": "demo-proj",
                    "title": "Minor styling glitch"
                }
            })),
        };
        let defer_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", defer_req).unwrap();
        let defer_msgs = defer_resp.result.unwrap()["messages"].as_array().unwrap().clone();
        let defer_text = defer_msgs[0]["content"]["text"].as_str().unwrap();
        assert!(defer_text.contains("Finding Deferred to Backlog"));
        assert!(temp_dir.join("projects/demo-proj/BACKLOG.md").exists());

        // 6. consult_peer_model tool call
        let consult_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(85)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "consult_peer_model",
                "arguments": {
                    "peer": "claude",
                    "prompt": "Evaluate this architecture pattern"
                }
            })),
        };
        let consult_resp = McpServer::handle_request(root_str, db.conn(), "coll_test", "prof_test", consult_req).unwrap();
        assert!(consult_resp.result.is_some());
        let consult_val: Value = serde_json::from_str(consult_resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(consult_val["peer"], "claude");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

