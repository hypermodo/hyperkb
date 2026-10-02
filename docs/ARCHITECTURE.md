# HyperKB Architecture

> Fast, resource-efficient local developer knowledge hub & proactive risk engine

HyperKB is a standalone, local-first knowledge hub and governance control plane bridging human developers and autonomous AI coding agents. It provides sub-millisecond retrieval, active pre-commit risk gates, dynamic multi-project segmentation, and transparent session telemetry without external SaaS or cloud dependencies.

---

## 🏛 System Architecture

HyperKB is organized into four decoupled architectural layers backed by a single 2.2MB native Rust binary and embedded SQLite FTS5:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                             Transport Layer                             │
│     • Interactive Terminal TUI (Ratatui + Crossterm)                    │
│     • Headless MCP Server (JSON-RPC 2.0 over stdio)                     │
│     • Command Line Interface (Clap: projects, init-harness, check-work) │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼────────────────────────────────────┐
│                              Domain Layer                               │
│     • ProjectSummary & Multi-Project Aggregation                        │
│     • Work Items Typing (DocumentKind: Task, Audit; Status: Pending,..) │
│     • Policy Directives & Rule-of-5 Engine (<50 Lines Ceiling)          │
│     • Architectural Risk Engine & Dynamic Glob Path Matching            │
│     • Session Telemetry & 0–100 Quality Score Heuristic                 │
│     • Actor & AuthorityGrant (Bounded Agent Delegation)                 │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼────────────────────────────────────┐
│                               Core Layer                                │
│     • Multi-Root Scanner: Concurrent Walks across Multi-Roots           │
│     • HarnessInit: Universal Thin Pointer Generator (~11 lines)         │
│     • Archeology: Git History Mining & Hotspot Discovery                │
│     • Linter: KB Bloat, Depth & Schema Enforcement                      │
│     • Git: Dynamic Repo-Root Pre-Commit Interception Hook               │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼────────────────────────────────────┐
│                              Storage Layer                              │
│     • Embedded SQLite 3 with FTS5 Full-Text Search                      │
│     • Dynamic Project Segmentation via Substring Aggregation            │
│     • Write-Ahead Logging (WAL) & In-Memory Fast Fallback               │
│     • Point-in-time Snapshot Backups & Automated Rotation               │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 📁 Multi-Root Knowledge Architecture & Manifest Schema

HyperKB treats large-scale codebases and monorepos as unified knowledge graphs partitioned into distinct operational roots:

```json
{
  "name": "ZDP-SYSTEM-KB",
  "collection_id": "zdp-system-kb",
  "knowledge_roots": ["projects", "shared", "docs"],
  "directives_path": "directives",
  "risks_path": "risks",
  "decisions_path": "decisions",
  "memory_path": "memory",
  "settings": {
    "max_briefing_directives": 5,
    "audit_max_lines": 250,
    "audit_max_depth": 4
  }
}
```

### Operational Separation
- **`projects/`**: Segregated project folders (e.g. `projects/<project-name>/STATUS.md`, `projects/<project-name>/tasks/`).
- **`shared/` & `docs/`**: Shared architecture, contracts, specifications, and runbooks.
- **`directives/`**: Standing policy invariants (e.g., zero code comments, memory caps). Invariants are stored in a dedicated `directives` database table and strictly segregated from general documentation.
- **`risks/`**: Machine-readable hazard declarations citing path globs (`src/storage/**`).
- **`decisions/`**: Architecture Decision Records (ADRs) tracking architectural evolution.

---

## 🧭 Dynamic Project Segmentation & Cardinality Scaling

In complex enterprise environments (such as 60+ projects with thousands of tasks), manual catalog maintenance leads to registry decay. HyperKB solves cardinality scaling via zero-cost SQLite aggregation:

```sql
SELECT
    substr(path, 10, instr(substr(path, 10), '/') - 1) AS proj_name,
    count(*) AS total_docs,
    count(CASE WHEN kind = 'task' AND status IN ('pending', 'todo', 'open') THEN 1 END) AS pending_tasks,
    count(CASE WHEN kind = 'task' AND status IN ('in_progress', 'active') THEN 1 END) AS in_progress_tasks,
    count(CASE WHEN kind = 'task' AND status IN ('completed', 'done', 'resolved') THEN 1 END) AS completed_tasks,
    count(CASE WHEN kind = 'task' AND status = 'blocked' THEN 1 END) AS blocked_tasks,
    count(CASE WHEN kind = 'risk' AND status = 'open' THEN 1 END) AS open_risks,
    count(CASE WHEN kind = 'decision' THEN 1 END) AS decisions_count,
    max(CASE WHEN lower(path) = 'projects/' || lower(substr(path, 10, instr(substr(path, 10), '/') - 1)) || '/status.md' THEN 1 ELSE 0 END) AS has_status
FROM documents
WHERE collection_id = ?1 AND path LIKE 'projects/%/%'
GROUP BY proj_name
HAVING length(proj_name) > 0
ORDER BY proj_name ASC;
```

### Benefits
- **Zero Config**: Any directory created under `projects/<new-project>/` is immediately recognized.
- **Single-Roundtrip Discovery**: AI agents call `list_projects` via MCP or developers run `hyperkb projects` to view the health of all projects in <2ms.

---

## 🪶 The Thin Pointer Model & Rule of 5

HyperKB eliminates prompt bureaucracy. Instead of stuffing thousands of lines of prompt instructions into `CLAUDE.md`, `GEMINI.md`, or system prompts, HyperKB generates a generic, ~11-line **thin pointer**:

```markdown
# Agent Governance & Architectural Boundaries

This repository is governed by HyperKB (Deterministic Governance & Knowledge Hub).

Before planning or executing changes:
1. Always run `check_work` via MCP (or CLI `hyperkb check-work <files>`) with planned file paths.
2. Adhere strictly to any returned Policy Directives (Rule of 5) and mitigate Cited Risks.
3. Discover architecture, schemas, and tasks via `browse` and `search` instead of guessing.
4. Record significant architectural decisions with `draft_decision` and hazards with `draft_risk`.

All code edits are mechanically verified on pre-commit via local Git hooks.
```

### Rule of 5 Session Briefing Ceiling
When agents start, they invoke `get_session_briefing`. HyperKB enforces a strict **Rule of 5** and a **hard 48-line ceiling**:
- Maximum 5 active policy directives.
- Maximum 5 cited open risks.
- Maximum 5 recent churn hotspots.
- Hard clamp preventing prompt bloat and model attention degradation.

---

## 🛡 Dynamic Path Normalization & Pre-Commit Interception

HyperKB guarantees deterministic pre-commit risk and invariant gating across all environments:

1. **Path Normalization (`RiskEngine::normalize_path`)**:
   - Strips leading `./` and redundant `.` path segments.
   - Normalizes Windows backslashes `\` to standard `/`.
   - Strips absolute workspace prefixes when paths are passed by IDEs or full-path harness tools.
2. **Multi-Root Glob Matching (`RiskEngine::matches_path`)**:
   - **Generic Unrooted Patterns**: A pattern like `src/**` or `*.rs` automatically matches files inside subprojects (e.g., `projects/alpha/src/lib.rs`).
   - **Rooted Patterns**: A pattern defined as `projects/alpha/src/**` matches when an agent passes a subproject-relative target `src/lib.rs`.
   - **Full Glob Expressiveness**: Supports `**`, `*`, prefix (`foo*`), suffix (`*.rs`), and exact segment matching.
3. **Resilient Git Hook**:
   - The `.git/hooks/pre-commit` script resolves `REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"` and executes `hyperkb -r "$REPO_ROOT" check-work --staged`.
   - Halts commits that violate unacknowledged risks or policy directives regardless of the subdirectory from which `git commit` is invoked.

---

## 📊 Honest 0–100 Session Quality Score

HyperKB replaces subjective AI evaluations with a transparent, deterministic heuristic tracking agent execution efficiency:

$$\text{Score} = \text{clamp}(100 - P_{\text{loops}} - P_{\text{friction}} - P_{\text{thrash}} + B_{\text{invariant}}, 5, 100)$$

* **Review Loops ($P_{\text{loops}}$, -15% each)**: Penalizes cycles where review rejected changes.
* **Friction ($P_{\text{friction}}$, -15%)**: Deducted if execution broke unit tests, build checks, or syntax.
* **Tool Thrash ($P_{\text{thrash}}$)**: Deducted when tool calls to file edits ratio exceeds 8:1 without making progress.
* **Invariant Compliance ($B_{\text{invariant}}$, +10%)**: Rewarded when active repo directives prevent a known regression.

---

## ⚡ Empirical Performance & Resource Footprint

Tested against real monorepos containing **1,172 documents across 62 segregated projects**:

| Operation | Latency | Resource Consumption |
| :--- | :--- | :--- |
| **Full Multi-Root Indexing** | ~1.1 seconds | Single CPU thread, <45MB RSS |
| **Incremental Re-Index** | ~40 milliseconds | Zero allocation, SHA-256 cache |
| **BM25 FTS5 Search** | ~8 milliseconds | Embedded SQLite index |
| **Multi-Project Aggregation (`list_projects`)** | ~2 milliseconds | Single SQL GROUP BY query |
| **Pre-Commit Path Gating (`check_work`)** | <10 milliseconds | In-memory glob matcher |
| **Idle TUI Process** | 0.0% CPU | <25 MB RSS |
| **Standalone Binary Size** | 2.2 MB | Native Mach-O / ELF binary (zero dependencies) |
