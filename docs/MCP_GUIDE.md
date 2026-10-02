# Model Context Protocol (MCP) Guide

HyperKB includes a native **Model Context Protocol (MCP)** stdio server built directly into the binary. It allows AI coding agents (Claude Code, Claude Desktop, Cursor, Antigravity, OpenCode) to query repository knowledge, check active risks, draft decisions, and track session quality over JSON-RPC 2.0.

---

## 🚀 Configuration Examples

### 1. Claude Code CLI
Add to `~/.claude/mcp.json` or `.mcp.json` in your repository root:

```json
{
  "mcpServers": {
    "hyperkb": {
      "command": "hyperkb",
      "args": ["mcp", "--root", "."]
    }
  }
}
```

### 2. Antigravity & Agentic IDEs
Add to `.agents/mcp_config.json`:

```json
{
  "mcpServers": {
    "hyperkb": {
      "command": "hyperkb",
      "args": ["--root", ".", "mcp"]
    }
  }
}
```

### 3. Centralized Knowledge Hub (Cross-Repo Access)
If your knowledge base lives in a central repo (e.g. `SYSTEM-KB`) while your code lives in a separate project repo, point `--root` to the central KB:

```json
{
  "mcpServers": {
    "hyperkb": {
      "command": "hyperkb",
      "args": ["mcp", "--root", "/path/to/central-system-kb"]
    }
  }
}
```

---

## 🛠 Available MCP Tools

| Tool | Description |
| :--- | :--- |
| `check_work` | Validates planned file edits against active architectural risks before code modification. |
| `search` | BM25 ranked full-text search across repository specs, decisions, and documentation. |
| `browse` | Lists indexed documents filtered by category (`decisions`, `risks`, `specs`) or `project`. |
| `get_document` | Retrieves the complete Markdown content and frontmatter for a specific document path. |
| `draft_decision` | Creates a proposed Architecture Decision Record (ADR) for human owner review. |
| `accept_decision` | Ratifies a proposed decision document using a valid AuthorityGrant. |
| `draft_risk` | Proposes a new architectural risk citing affected file path glob patterns. |
| `acknowledge_risk` | Records mitigating rationale so an open risk no longer halts pre-commit checks. |
| `list_directives` | Returns standing repository policy directives enforcing the Rule of 5. |
| `draft_directive` | Proposes a new repository invariant. |
| `retire_directive` | Marks an outdated directive as retired. |
| `get_session_briefing` | Delivers warm-start context (invariants, risks, churn hotspots) for new agent sessions. |
| `get_session_scorecard` | Returns the real-time 0–100 session quality score, duration, and thrashing metrics. |
| `audit_kb` | Audits documentation for nesting depth, word bloat, and stale content. |
