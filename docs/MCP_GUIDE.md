# Model Context Protocol (MCP) Guide

HyperKB includes a native **Model Context Protocol (MCP)** stdio server built directly into the binary. It allows AI coding agents (Claude Code, Google Antigravity, OpenCode, Cursor, GitHub Copilot) to query repository knowledge, discover segregated project tasks, check active architectural risks, draft decisions, and track session quality over JSON-RPC 2.0.

---

## 🌐 Supported Deployment Topologies

HyperKB supports three generic deployment topologies across all AI coding harnesses (**OpenCode, Claude Code, Google Antigravity, Cursor, VS Code, Codex, and Aider**):

```
Topology 1: Embedded (Per-Repo)       Topology 2: Multi-Repo Suite (Parent Hub)      Topology 3: Global Machine Hub
my-repo/                              workspace-group/                               ~/.config/ (or global path)
├── hyperkb.json                      ├── system-kb/ (shared hub)                    ├── opencode/opencode.json
├── decisions/                        ├── service-a/ (child repo 1)                  └── ~/.claude/mcp.json
├── directives/                       └── service-b/ (child repo 2)                            │
└── src/                                        │                                              ▼
        │                                       ▼                             /shared/global-system-kb/
        ▼                     Inherits parent config with --root system-kb    Universal machine-wide invariants
Self-contained in git
```

---

### Scenario 1: Embedded Topology (One KB per Repository)
*Best for standalone repositories, monolithic applications, or open-source projects where ADRs and invariants travel in git alongside the code.*

Leave `--root` omitted (defaults to current directory `.`):

- **OpenCode** (`my-repo/opencode.json`):
  ```json
  {
    "$schema": "https://opencode.ai/config.json",
    "mcp": {
      "hyperkb": {
        "type": "local",
        "command": ["npx", "-y", "@hypermodo/hyperkb", "mcp"]
      }
    }
  }
  ```
- **Claude Code CLI** (run in repo or add to `my-repo/.mcp.json`):
  ```bash
  claude mcp add hyperkb npx -y @hypermodo/hyperkb mcp
  ```
- **Cursor & VS Code** (`my-repo/.cursor/mcp.json`):
  ```json
  {
    "mcpServers": {
      "hyperkb": {
        "command": "npx",
        "args": ["-y", "@hypermodo/hyperkb", "mcp"]
      }
    }
  }
  ```
- **Google Antigravity IDE** (`my-repo/.agents/mcp_config.json`):
  ```json
  {
    "mcpServers": {
      "hyperkb": {
        "command": "npx",
        "args": ["-y", "@hypermodo/hyperkb", "mcp"]
      }
    }
  }
  ```

---

### Scenario 2: Multi-Repo Suite Topology (Shared Parent Hub)
*Best for microservice ecosystems or product suites where multiple child repositories share architectural invariants, decisions, and risks, while keeping different organizational groups strictly segregated.*

Inside `system-kb/projects/<repo-name>`, HyperKB automatically scopes tasks and status to the active child repo, while directives and risks remain shared across the entire suite.

Place the configuration in the **group parent directory** (`workspace-group/`):

- **OpenCode** (`workspace-group/opencode.json`):
  *(OpenCode automatically walks up parent directories; all child repos inherit this configuration with zero config needed inside child repos).*
  ```json
  {
    "$schema": "https://opencode.ai/config.json",
    "mcp": {
      "hyperkb": {
        "type": "local",
        "command": [
          "npx",
          "-y",
          "@hypermodo/hyperkb",
          "mcp",
          "--root",
          "/absolute/path/to/workspace-group/system-kb"
        ]
      }
    }
  }
  ```
- **Claude Code CLI** (`workspace-group/.mcp.json`):
  ```json
  {
    "mcpServers": {
      "hyperkb": {
        "command": "npx",
        "args": [
          "-y",
          "@hypermodo/hyperkb",
          "mcp",
          "--root",
          "/absolute/path/to/workspace-group/system-kb"
        ]
      }
    }
  }
  ```
- **Cursor / VS Code & Google Antigravity** (in workspace root `.cursor/mcp.json` or `.agents/mcp_config.json`):
  ```json
  {
    "mcpServers": {
      "hyperkb": {
        "command": "npx",
        "args": [
          "-y",
          "@hypermodo/hyperkb",
          "mcp",
          "--root",
          "/absolute/path/to/workspace-group/system-kb"
        ]
      }
    }
  }
  ```
- **Codex / Aider / Headless Agents**:
  Run `hyperkb init-harness all` inside child repos to generate thin pointer files (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`) that point to `../system-kb`. HyperKB's native root resolver also automatically auto-discovers sibling hubs named `*-SYSTEM-KB`, `*-KB`, or `kb/`.

---

### Scenario 3: Global Machine Hub (One KB for All Repos)
*Best for solo developers or architects wanting unified personal knowledge, private notes, and global rules across every folder on their workstation.*

Place the configuration in your **global user config**:

- **OpenCode** (`~/.config/opencode/opencode.json`):
  ```json
  {
    "$schema": "https://opencode.ai/config.json",
    "mcp": {
      "hyperkb": {
        "type": "local",
        "command": [
          "npx",
          "-y",
          "@hypermodo/hyperkb",
          "mcp",
          "--root",
          "/path/to/global-system-kb"
        ]
      }
    }
  }
  ```
- **Claude Code CLI**: Add globally via `~/.claude/mcp.json`:
  ```bash
  claude mcp add hyperkb npx -y @hypermodo/hyperkb mcp --root /path/to/global-system-kb
  ```
- **Cursor / VS Code**: Add to Global User Settings (`settings.json`).
- **Antigravity IDE**: Add to global Antigravity MCP settings.

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
| `transition_task` | `project: string`, `task_id: string`, `status: string`, `reason?: string` | Transitions a project task status (`in_progress`, `completed`, `blocked`, `pending`) and syncs status.md active critical path lock. |
| `update_status` | `project: string`, `health: string`, `reason?: string`, `blocker?: string` | Updates project health (`healthy`, `at_risk`, `blocked`) and appends blocker notes. |
| `get_project_status` | `project: string` | Retrieves structured status, active task lock, blockers, and exit criteria for a project. |
| `defer_finding` | `project: string`, `title: string`, `details: string`, `severity?: string` | Jails tangential discoveries into project BACKLOG.md to prevent critical path context thrashing. |
| `verify_exit_criteria` | `project: string` | Executes the project's exit criteria command and automatically marks the project completed upon matching exit code. |
| `consult_peer_model` | `peer: string`, `prompt: string`, `context_files?: string[]` | Consults a peer AI model (Claude, ChatGPT, Gemini) as middleware with centralized audit logging and token latency tracking. |

---

## 💬 In-Harness Slash Commands & Universal MCP Prompts

HyperKB 2.1 implements the official Model Context Protocol **Prompts specification** (`prompts/list` and `prompts/get`). In harnesses that support MCP Prompts (OpenCode, Claude Code, Cursor), these automatically appear as **native slash commands**:

| Prompt / Slash Command | Arguments | In-Harness Behavior |
| :--- | :--- | :--- |
| `hkb-brief` (or `/hkb-brief`) | `scope?: string` | Delivers warm-start context briefing clamped under 35 lines with Rule of 5 directives and locked critical path. *(Aliases: `hkb_brief`, `brief`)* |
| `hkb-status` (or `/hkb-status`) | `project?: string` | Renders active project health, active task lock (`🔒 task-02`), blockers, and exit criteria. *(Aliases: `hkb_status`, `status`)* |
| `hkb-verify` (or `/hkb-verify`) | `project?: string` | Runs deterministic exit criteria verification; reports exit code, stdout, and pass/fail. *(Aliases: `hkb_verify`, `verify`)* |
| `hkb-task-next` (or `/hkb-task-next`) | `project?: string`, `task_id?: string`, `status?: string`, `reason?: string` | Advances active task to completed and locks next sequential task without leaving the harness. *(Aliases: `hkb_task_next`, `task_next`)* |
| `hkb-defer` (or `/hkb-defer`) | `title: string`, `details?: string`, `project?: string` | Jails a side finding into `BACKLOG.md` with strict return-to-path instruction. *(Aliases: `hkb_defer`, `defer`)* |
| `hkb-metrics` (or `/hkb-metrics`) | `session_id?: string` | Renders tool-to-edit ratio, review loop oscillations, and coding effectiveness scorecard. *(Aliases: `hkb_metrics`, `metrics`)* |
| `hkb-claude` (or `/hkb-claude`) | `prompt: string`, `context_files?: string` | Consults Anthropic Claude peer model directly from your current harness (e.g. from Antigravity/Gemini or OpenCode). *(Aliases: `hkb_claude`, `claude`)* |
| `hkb-chatgpt` (or `/hkb-chatgpt`) | `prompt: string`, `context_files?: string` | Consults OpenAI ChatGPT peer model directly from your current harness. *(Aliases: `hkb_chatgpt`, `chatgpt`)* |
| `hkb-gemini` (or `/hkb-gemini`) | `prompt: string`, `context_files?: string` | Consults Google Gemini peer model directly from your current harness. *(Aliases: `hkb_gemini`, `gemini`)* |

