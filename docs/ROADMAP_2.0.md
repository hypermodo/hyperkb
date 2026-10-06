# HyperKB Evolution 2.0: Anti-Churn & Typed Governance Roadmap

> **Status**: Approved for Implementation  
> **Source Grounding**: Live analysis of `/Volumes/ExtSSD/Workspace/ZDP/ZDP-SYSTEM-KB` (62 projects, 3,460 docs, 47 handoffs in `adaptive-resource-tuning`, 373 archived docs in `_archive`).  
> **Core Objective**: Transform HyperKB from a read-heavy markdown indexer into a deterministic, anti-churn agentic control plane with typed governance and zero cloud dependencies.

---

## 🔬 Empirical Ground Truth & The 4 Failure Modes

From direct sampling of `ZDP-SYSTEM-KB`, four systemic failure modes were identified:

1. **The "Endless Project" / Yak-Shaving Trap**:
   - *Evidence*: `projects/adaptive-resource-tuning/STATUS.md` accumulated 44-task tables, 47 handoff files (~1.2MB of uncurated prose), and 450 passing unit tests over weeks—yet the core promote cancellation bug stayed unclosed.
   - *Cause*: Agents treat tangential discoveries (e.g. "found 22 other statement sites") with the same priority as the core blocker. No locked critical path; no falsifiable exit test.

2. **The "Prose Drift" Trap**:
   - *Evidence*: `projects/jwt-hotreload/plans/` contains a 17KB implementation plan from June 30 with obsolete Go snippets that lingers in search queries months after work was merged.
   - *Cause*: Specs and plans are written as long-form narrative prose. When code diverges at Step 2, the remaining prose becomes stale fiction that pollutes LLM context.

3. **The "ASCII Table Fragility" Trap**:
   - *Evidence*: `jwt-hotreload/STATUS.md` uses hand-maintained pipe tables (`| Field | Value |`, `▓▓▓▓▓▓▓░░░ 67%`, `✓`/`—`).
   - *Cause*: No machine-enforced schema (`schema: hyperkb/*`). Agents and humans must perform fragile regex/text surgery on markdown tables.

4. **The "Digital Landfill" / Handoff Sprawl**:
   - *Evidence*: 373 documents across 27 projects manually moved to `projects/_archive/`. 47 separate session handoffs in one project because session resets lack an epoch compression mechanism.
   - *Cause*: Git has no context garbage collector. FTS5 indexes dead files without a tombstone filter.

---

## 🗺️ Phased Implementation Plan

```
┌────────────────────────────────────────────────────────────────────────┐
│                   HyperKB Evolution 2.0: Roadmap                       │
├────────────────────────────────────────────────────────────────────────┤
│  Phase 6: Native Tombstoning & Clean Context Partitioning             │
│  Phase 7: Typed Frontmatter Schemas & Atomic Mutation Engine          │
│  Phase 8: Anti-Churn Control Plane (Critical Path & Side-Quest Jail)   │
│  Phase 9: Multi-Repo Sibling Hook Propagation & Auto-Diff              │
│  Phase 10: Cockpit Convergence Dashboard & Velocity Telemetry          │
└────────────────────────────────────────────────────────────────────────┘
```

---

### Phase 6: Native Tombstoning & Clean Context Partitioning
**Goal**: Immediately eliminate context bloat by hiding `_archive/`, `_draft/`, and completed plans from FTS5 and briefings, without moving files in Git.

- [x] **Storage Layer (`src/storage/schema.sql`, `src/storage/db.rs`)**:
  - Add `is_tombstone INTEGER DEFAULT 0` column to `documents` table with non-destructive auto-migration and backfill.
  - Create index: `CREATE INDEX IF NOT EXISTS documents_tombstone ON documents(collection_id, is_tombstone);`.
- [x] **Scanner Layer (`src/core/scanner.rs`)**:
  - Detect tombstones during directory walk:
    - Path matches `**/_archive/**`, `**/_draft/**`, `**/refuted/**`, `**/_trash/**`.
    - YAML frontmatter has `status: completed` (plans), `status: archived`, `status: refuted`, or `tombstone: true`.
    - Automatically syncs tombstone status even for unchanged files.
- [x] **Query Layer (`src/storage/queries.rs`, `src/transport/mcp.rs`, `src/main.rs`)**:
  - Default `search`, `browse`, and `list_projects` to `WHERE is_tombstone = 0` (excluding `_archive` & `_draft`).
  - Add `--include-archived` flag to CLI and `include_archived: bool` parameter to MCP search, browse, and list_projects tools.
- [x] **Verification**:
  - Run `hyperkb search`, `hyperkb browse`, and `hyperkb projects` on `ZDP-SYSTEM-KB`: verified 0 hits from `_archive` by default; verified all 373 archived documents cleanly segregated and accessible with `--include-archived`.

---

### Phase 7: Typed Frontmatter Schemas & Atomic Mutation Engine
**Goal**: Replace fragile ASCII pipe tables and markdown surgery with strongly-typed schemas and atomic frontmatter mutations.

- [x] **Domain Schema Module (`src/domain/schema.rs`)**:
  - `StatusDocument`: `status` enum (`planned`, `active`, `blocked`, `completed`, `archived`), `health` (`healthy`, `at_risk`, `blocked`), `goal`, `baseline`, `blockers: Vec<BlockerItem>`, `milestones: Vec<MilestoneItem>`, `active_task`, `exit_criteria`.
  - `SpecDocument`: `invariants: Vec<String>`, `non_goals: Vec<String>`, `contracts: Vec<ContractItem>`.
  - `PlanDocument`: `ttl_days: u32`, `checkpoints: Vec<CheckpointItem>`.
  - `AuditDocument`: `commit_sha`, `verdict`, `rules_evaluated`, `violations: Vec<ViolationItem>`.
- [x] **Frontmatter Splicer (`src/domain/frontmatter.rs`)**:
  - Parse and isolate the YAML frontmatter block (`--- ... ---`).
  - Mutate frontmatter fields and rewrite atomically to disk while keeping the body markdown byte-identical.
- [x] **CLI & MCP Mutation Commands (`src/core/status_engine.rs`, `src/main.rs`, `src/transport/mcp.rs`)**:
  - CLI: `hyperkb status get <project>`, `hyperkb status transition <project> --health <healthy|at_risk|blocked> [--reason <str>] [--blocker <str>]`.
  - CLI: `hyperkb task transition <project> <task_id> --to <pending|in_progress|completed|blocked> [--reason <str>]`.
  - MCP: `transition_task(project, task_id, status, reason)`.
  - MCP: `update_status(project, health, blocker, reason)`.
  - MCP: `get_project_status(project)`.
- [x] **Verification**:
  - Tested on `ZDP-SYSTEM-KB` (extracted real-world goal from 29KB STATUS.md).
  - Verified atomic git diff: 100% byte-identical markdown body preservation under task and status mutations.

---

### Phase 8: Anti-Churn Control Plane (Critical Path & Side-Quest Jail)
**Goal**: Prevent agent yak-shaving by locking context to a single critical path item, capturing side-quests into a backlog, and providing executable exit criteria.

- [x] **Single-Slot Critical Path in Briefings (`src/core/session_manager.rs`)**:
  - Extract active unblocked milestone item or in_progress task from project's `status.md`.
  - Format `get_session_briefing` with a hard anti-churn directive (`🔒 LOCKED CRITICAL PATH`), respecting the strict 48-line briefing ceiling.
- [x] **Side-Quest Jail (`src/transport/mcp.rs`, `src/core/status_engine.rs`)**:
  - Implement MCP tool `defer_finding(project, title, details, severity)`.
  - Atomically appends finding to `projects/<project>/BACKLOG.md` without interrupting active workflow.
  - Returns strict anti-churn instruction: *"Finding recorded to project backlog. You are strictly prohibited from addressing this now. Return immediately to the active Critical Path task."*
- [x] **Executable Exit Criteria (`src/core/status_engine.rs`, `src/main.rs`, `src/transport/mcp.rs`)**:
  - Typed `ExitCriteria { command, expected_exit_code, verified_at }` in `StatusDocument`.
  - CLI command `hyperkb verify-exit <project>` executes command via `sh -c` inside project directory and atomically transitions project to `completed` and `healthy` on exit code match.
  - MCP tool `verify_exit_criteria(project)` for autonomous agent verification and project closure.
- [x] **Verification**:
  - Verified `get_session_briefing` cleanly locks critical path slot.
  - Verified `defer_finding` diverts tangential findings safely to `BACKLOG.md`.
  - Verified `verify-exit` executes command and transitions project health and status cleanly.
  - All 99 unit/integration tests passing.

---

### Phase 9: Multi-Repo Sibling Hook Propagation & Auto-Diff
**Goal**: Automate pre-commit hook deployment across 62 sibling repositories and enable auto-diff inspection in MCP.

- [x] **Sibling Hook Installer (`src/core/git.rs`, `src/main.rs`)**:
  - Commands: `hyperkb install-hook [--all-siblings]` and `hyperkb hook install [--all-siblings]`.
  - Resolves sibling root directories (`../*`), identifies `.git/` directories, and installs/updates `.git/hooks/pre-commit` to call `hyperkb check-work -r "$REPO_ROOT" --staged` with embedded binary fallback.
- [x] **Auto-Diff Aware MCP `check_work` (`src/transport/mcp.rs`, `src/main.rs`)**:
  - Made `files` parameter optional in MCP schema and CLI command.
  - When omitted/empty, HyperKB automatically queries Git staged, modified, and untracked files via `Git::get_modified_and_untracked_files`.
  - Returns clean message when working tree has no changes; evaluates open risks and comment hygiene on detected files.
- [x] **Verification**:
  - Ran `hyperkb install-hook --all-siblings`; verified hooks deployed cleanly across repositories (`hyperkb`, `hypercontrol`).
  - Ran `hyperkb check-work` with no arguments; verified automatic detection and validation of changed working tree files.
  - All 101 tests passing.

---

### Phase 10: Cockpit Convergence Dashboard & Velocity Telemetry
**Goal**: Provide real-time operator visibility into project convergence, blockers, and churn warnings directly in the Ratatui TUI.

- [x] **TUI Work Tab Cockpit (`src/ui/views/work.rs`, `src/ui/views/task_transition.rs`, `src/ui/mod.rs`)**:
  - Rendered Critical Path badge (`🔒 <active_task>`) on Project Overview Card.
  - Rendered Exit Criteria status indicator (`🎯 [✔ VERIFIED]` / `🎯 [○ PENDING]`).
  - Implemented interactive `TaskTransitionModal` with hotkey `[t]` to transition task status (`in_progress`, `completed`, `blocked`, `pending`) directly inside Ratatui TUI.
- [x] **Convergence Velocity Heuristic (`src/domain/telemetry.rs`)**:
  - Implemented `ProjectVelocityTelemetry` and `compute_project_churn`.
  - Computes session-to-milestone churn ratio; displays bold `[▲ CHURN WARNING]` in Project Overview Card and `[▲ CHURN]` badge on project list items when threshold is exceeded without milestone progression.
- [x] **Verification**:
  - Validated across `ZDP-SYSTEM-KB` (60 projects cleanly segregated).
  - Verified interactive task transition modal flow and state transitions.
  - All 103 unit and integration tests passing.

---

### Phase 11: Extended MCP Protocol, Universal Prompts & Peer Model Middleware
**Goal**: Bring zero-context-switch tactical control directly into AI coding harnesses (OpenCode, Claude Code, Cursor, Antigravity) via official MCP Prompts and enable cross-harness peer model consultation.

- [x] **Universal MCP Prompts Specification (`src/transport/mcp.rs`)**:
  - Advertised `prompts: { "listChanged": false }` capability in `initialize`.
  - Implemented `prompts/list` exposing 9 in-harness slash commands (prefixed with ergonomic `hkb-`):
    - `/hkb-brief`: Clamped warm-start context briefing (<35 lines) enforcing Rule of 5 and critical path lock.
    - `/hkb-status`: Real-time project health, critical path lock (`🔒 <task>`), blockers, and exit criteria.
    - `/hkb-verify`: Executable deterministic exit criteria verification.
    - `/hkb-task-next`: In-harness critical path progression and task completion.
    - `/hkb-defer`: Backlog jailing of tangential findings with return-to-path constraint.
    - `/hkb-metrics`: Session effectiveness, tool-to-edit ratio, and loop oscillation scorecard.
    - `/hkb-claude`, `/hkb-chatgpt`, `/hkb-gemini`: Direct peer model consultation prompts.
  - Implemented `prompts/get` handler with automatic project discovery and Markdown formatting.
- [x] **Peer Model Middleware Router (`src/core/model_router.rs`)**:
  - Implemented `ModelRouter::consult` supporting Anthropic (Claude), OpenAI (ChatGPT), and Google (Gemini).
  - Supports API keys (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`/`GOOGLE_API_KEY`) and local CLI fallbacks (`claude -p`).
  - Added automatic file context attachment (`context_files`) with safety truncation (<150 lines/file).
  - Exposed `consult_peer_model` MCP tool and `/claude`, `/chatgpt`, `/gemini` prompts with Action Ledger audit tracking.
- [x] **Verification**:
  - Unit tests: `test_mcp_prompts_list_and_get`, `test_model_router_unsupported_peer`, `test_model_router_unconfigured_graceful_response`, `test_model_router_context_file_attachment`.
  - All 111 unit and integration tests passing.

---

### Phase 12: Zero-Cargo Universal Distribution & Lifecycle Hook Engine
**Goal**: Solve the developer distribution and workflow friction problem so users never need Rust/Cargo to run HyperKB, and enable automated session start context injection and pre-tool risk guards across OpenCode, Claude Code, Cursor, and Antigravity.

- [x] **Universal NPM Distribution Package (`npm/`)**:
  - Light cross-platform npm package (`hyperkb`) wrapping pre-compiled native binaries (`hyperkb-darwin-arm64`, `hyperkb-linux-x64`, etc.).
  - Zero-cargo execution via `npx -y hyperkb [cmd]` and global CLI via `npm install -g hyperkb`.
  - Automatic architecture/platform detection, fallback order (env override -> bundled platform binary -> local workspace dev target -> system PATH), and macOS provenance attribute clearing.
  - Packaged tarball (`hyperkb-0.1.0.tgz`, 1.5 MB compressed).
- [x] **Generic Lifecycle Hook Engine (`src/main.rs`)**:
  - Implemented `hyperkb hook session-start [--scope <scope>] [--json]`: emits rule-of-5 clamped warm-start briefing (<35 lines) directly into harness output.
  - Implemented `hyperkb hook pre-tool-call [--tool <tool>] [--path <path>] [--strict] [--json]`: proactively evaluates open risks and comment hygiene before tool execution with deterministic blocker codes.
- [x] **OpenCode Native Harness Integration**:
  - Created reference OpenCode plugin (`examples/opencode-plugin/index.ts`).
  - Tested live in `hypercontrol` via both MCP (`tools.hyperkb.*`) and lifecycle hooks (`session.created`, `tool.execute.before`).
  - Verified live in OpenCode: `get_session_briefing`, `check_work`, and peer model consultation routing.

