# Model Context Protocol (MCP) Guide

HyperKB includes a native **Model Context Protocol (MCP)** stdio server built directly into the binary. It allows AI coding agents (Claude Code, Google Antigravity, OpenCode, Cursor, GitHub Copilot) to query repository knowledge, discover segregated project tasks, check active architectural risks, draft decisions, and track session quality over JSON-RPC 2.0.

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

### 3. Cursor & OpenCode
Add to Cursor's MCP configuration (`settings > MCP`):
- **Name**: `hyperkb`
- **Type**: `command`
- **Command**: `hyperkb mcp --root .`

### 4. Centralized Knowledge Hub (Monorepos & Multi-Repo)
If your knowledge base lives in a central repo (e.g. `/Volumes/ExtSSD/Workspace/ZDP/ZDP-SYSTEM-KB`) while your code lives in a separate subproject, point `--root` to the knowledge hub:

```json
{
  "mcpServers": {
    "hyperkb": {
      "command": "hyperkb",
      "args": ["mcp", "--root", "/Volumes/ExtSSD/Workspace/ZDP/ZDP-SYSTEM-KB"]
    }
  }
}
```

---

## 🪶 The Thin Pointer Integration

To eliminate prompt bloat and keep agent context windows clean, run:

```bash
hyperkb init-harness all
```

This generates concise (~11-line) instruction files:
- `AGENTS.md` (Universal)
- `CLAUDE.md` (Claude Code)
- `GEMINI.md` (Google Antigravity / Gemini)
- `.cursorrules` (Cursor)

### Standard Agent Workflow
1. **Pre-flight**: Agent starts and calls `get_session_briefing` (clamped to <50 lines via the Rule of 5).
2. **Project Discovery**: Agent calls `list_projects` to discover all segregated projects, task counts, and status documents.
3. **Task & Document Browsing**: Agent calls `browse` with `project`, `kind: "task"`, and `status: "pending"` to inspect specific work items.
4. **Pre-Edit Risk Check**: Before editing files, agent calls `check_work` with planned file paths (`["src/storage/db.rs"]`).
5. **Decisions & Hazards**: Agent drafts decisions with `draft_decision` or hazards with `draft_risk`.
6. **Pre-Commit Gate**: When changes are committed, the local Git hook evaluates git diff paths against active invariants and risks.

---

## 🛠 Available MCP Tools

| Tool | Parameters | Description |
| :--- | :--- | :--- |
| `check_work` | `files: string[]`, `version?: string`, `environment?: string` | Validates planned file edits against active architectural risks and policy invariants before code modification. Normalizes paths (strips leading `./`, absolute prefixes). |
| `list_projects` | *(none)* | Returns a complete breakdown of all segregated projects under `projects/<name>/...` with document totals, task status counts (`pending`, `in_progress`, `completed`, `blocked`), open risks, and status doc indicators. |
| `browse` | `category?: string`, `project?: string`, `kind?: string`, `status?: string`, `topic?: string`, `limit?: number`, `offset?: number` | Lists indexed documents filtered by category, project name, document kind (`task`, `decision`, `risk`, `spec`), and status (`pending`, `in_progress`, `completed`, `blocked`). |
| `search` | `query: string`, `limit?: number`, `include_private?: boolean` | BM25 ranked full-text search across repository specs, decisions, tasks, and documentation with snippet highlighting. |
| `get_document` | `path: string` | Retrieves the complete Markdown content, frontmatter metadata, and effective status for a specific document path. |
| `get_session_briefing` | `grant_scope?: string` | Delivers warm-start context (invariants, risks, churn hotspots) clamped to under 48 lines enforcing the Rule of 5. |
| `draft_decision` | `title: string`, `rationale: string`, `owner?: string`, `supersedes?: string` | Creates a proposed Architecture Decision Record (ADR) for human owner review. |
| `accept_decision` | `path: string`, `grant_id: string`, `owner?: string`, `agent_id?: string` | Ratifies a proposed decision document using a valid AuthorityGrant. |
| `draft_risk` | `title: string`, `rationale: string`, `paths: string[]`, `owner?: string` | Proposes a new architectural risk citing affected file path glob patterns. |
| `acknowledge_risk` | `risk_id: string`, `rationale: string`, `grant_id: string`, `owner?: string` | Records mitigating rationale so an open risk no longer halts pre-commit checks. |
| `list_directives` | `category?: string`, `status?: string` | Returns standing repository policy directives enforcing the Rule of 5. |
| `draft_directive` | `title: string`, `category: string`, `author?: string`, `scope?: string[]`, `enforcement?: string`, `content: string` | Proposes a new repository invariant. |
| `retire_directive` | `id: string` | Marks an outdated directive as retired. |
| `audit_directives` | *(none)* | Audits active directives for category bloat, dormancy, and stale path patterns. |
| `audit_kb` | `path?: string` | Audits documentation for nesting depth, word bloat, and stale content. |
| `get_session_scorecard`| `session_id?: string` | Returns the real-time 0–100 session quality score, duration, and thrashing metrics. |
| `record_session_metric`| `session_id: string`, `metric_name: string`, `value: number` | Emits custom agent velocity and execution metrics into the session telemetry log. |
| `remember` | `title: string`, `content: string`, `kind?: string` | Stores private, local agent memory and scratchpad notes. |
