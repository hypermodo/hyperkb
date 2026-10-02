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

- [ ] **Storage Layer (`src/storage/sqlite.rs`)**:
  - Add `is_tombstone INTEGER DEFAULT 0` column to `documents` table.
  - Create index: `CREATE INDEX idx_documents_tombstone ON documents(is_tombstone);`.
- [ ] **Scanner Layer (`src/core/scanner.rs`)**:
  - Detect tombstones during directory walk:
    - Path matches `**/_archive/**` or `**/_draft/**`.
    - YAML frontmatter has `status: completed`, `status: archived`, or `tombstone: true`.
    - Plan document exceeds TTL with all checkpoints done.
- [ ] **Query Layer (`src/core/queries.rs`)**:
  - Default `search`, `browse`, and `get_session_briefing` to `WHERE is_tombstone = 0`.
  - Add `--include-archived` flag to CLI and `include_archived: bool` parameter to MCP search tools.
- [ ] **Verification**:
  - Run `hyperkb search` on `ZDP-SYSTEM-KB`: verify 0 hits from `_archive` by default; verify hits return with `--include-archived`.

---

### Phase 7: Typed Frontmatter Schemas & Atomic Mutation Engine
**Goal**: Replace fragile ASCII pipe tables and markdown surgery with strongly-typed schemas and atomic frontmatter mutations.

- [ ] **Domain Schema Module (`src/domain/schema.rs`)**:
  - `StatusDocument`: `status` enum (`planned`, `active`, `blocked`, `completed`, `archived`), `goal`, `baseline`, `blockers: Vec<Blocker>`, `milestones: Vec<Milestone>`.
  - `SpecDocument`: `invariants: Vec<Invariant>`, `non_goals: Vec<String>`, `contracts`.
  - `PlanDocument`: `ttl_days: u32`, `checkpoints: Vec<Checkpoint>`.
  - `AuditDocument`: `commit_sha`, `rules_evaluated`, `violations`.
- [ ] **Frontmatter Splicer (`src/domain/document.rs`)**:
  - Parse and isolate the YAML frontmatter block (`--- ... ---`).
  - Mutate frontmatter fields and rewrite to disk while keeping the body markdown byte-identical.
- [ ] **CLI & MCP Mutation Commands**:
  - CLI: `hyperkb status transition <project> --health <healthy|blocked> [--reason <str>]`.
  - CLI: `hyperkb task transition <project> <task_id> --to <in_progress|completed|blocked>`.
  - MCP: `transition_task(project, task_id, status, reason)`.
  - MCP: `update_status(project, health, blocker)`.
- [ ] **Verification**:
  - Mutate a project's status via CLI and MCP; verify git diff modifies only YAML frontmatter without breaking markdown syntax.

---

### Phase 8: Anti-Churn Control Plane (Critical Path & Side-Quest Jail)
**Goal**: Prevent agent yak-shaving by locking context to a single critical path item, capturing side-quests into a backlog, and providing executable exit criteria.

- [ ] **Single-Slot Critical Path in Briefings (`src/core/queries.rs`)**:
  - Extract active unblocked milestone item from project's `status.md`.
  - Format `get_session_briefing` with a hard anti-churn directive:
    ```
    LOCKED CRITICAL PATH: Task #<id> (<title>)
    CONSTRAINT: You are prohibited from refactoring other files or addressing 
                adjacent bugs until Task #<id> passes verification.
    ```
- [ ] **Side-Quest Jail (`src/transport/mcp.rs`)**:
  - Implement MCP tool `defer_finding(project, title, details, severity)`.
  - Writes to `projects/<project>/BACKLOG.md` or SQLite `findings` table.
  - Returns strict instruction: *"Finding recorded to project backlog. Return immediately to the active Critical Path task."*
- [ ] **Executable Exit Criteria**:
  - Add `exit_criteria: { command: "...", expected_exit_code: 0 }` to `StatusDocument`.
  - Add CLI command: `hyperkb verify-exit <project>` to execute verification and transition project to `completed` upon success.
- [ ] **Verification**:
  - Verify `get_session_briefing` strictly locks critical path; verify `defer_finding` safely diverts tangential work.

---

### Phase 9: Multi-Repo Sibling Hook Propagation & Auto-Diff
**Goal**: Automate pre-commit hook deployment across 62 sibling repositories and enable auto-diff inspection in MCP.

- [ ] **Sibling Hook Installer (`src/commands/hook.rs`)**:
  - Command: `hyperkb hook install [--all-siblings]`.
  - Resolves sibling root directories (`../*`), identifies `.git/` directories, and installs/updates `.git/hooks/pre-commit` to call `hyperkb check-work -r "$REPO_ROOT"`.
- [ ] **Auto-Diff Aware MCP `check_work` (`src/transport/mcp.rs`)**:
  - Make `files` parameter optional.
  - When omitted/empty, HyperKB automatically queries `git status --porcelain` and `git diff --name-only` inside the active repository.
- [ ] **Verification**:
  - Run `hyperkb hook install --all-siblings`; verify hooks are active in sibling repositories.

---

### Phase 10: Cockpit Convergence Dashboard & Velocity Telemetry
**Goal**: Provide real-time operator visibility into project convergence, blockers, and churn warnings directly in the Ratatui TUI.

- [ ] **TUI Work Tab Cockpit (`src/ui/screens/work.rs`)**:
  - Render Critical Path badge on Project Overview Card.
  - Render Exit Criteria status indicator.
  - Hotkey `[t]`: Quick-action popup to transition task status directly in TUI.
- [ ] **Convergence Velocity Heuristic (`src/domain/telemetry.rs`)**:
  - Compute churn ratio: sessions vs milestone transitions.
  - Display `[▲ CHURN WARNING]` in TUI if session count exceeds threshold without milestone progression.
- [ ] **Verification**:
  - Run TUI across `ZDP-SYSTEM-KB`; verify interactive task transitions and churn warnings render cleanly.
