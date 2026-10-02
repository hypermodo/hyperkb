# HyperKB

> **Local Architecture Invariants, Knowledge Base & AI Agent Control Plane**  
> *Zero-overhead TUI and CLI paired with your favorite IDE and AI coding agents*

HyperKB is a standalone, terminal-agnostic, zero-latency control plane designed to bridge human developers and collaborating AI coding agents. It provides real-time invariant enforcement, standing policy directives, bounded agent authority delegation, risk verification gates, and transparent session quality telemetry.

---

## 🏛 Product Responsibility Boundaries

HyperKB is explicitly architected with clear boundaries across the development lifecycle:

| Plane | Tool | Primary Purpose & Responsibilities |
| :--- | :--- | :--- |
| **Local Control Plane** | **HyperKB** | **Invariants & Guardrails**: Standing directives, scoped agent authority grants, pre-commit risk gates (`check-work`), session quality telemetry, fast metadata lookup. Zero context switching, <5ms query response, 0.0% idle CPU. |
| **Authoring Plane** | **External IDE** (`[o]`) | **Prose & Implementation**: Long-form markdown ADR drafting, architecture prose authoring, source code implementation. Pressing `[o]` inside HyperKB opens the active document in `$EDITOR` (VS Code, Cursor, Zed, Neovim). |
| **Macro Plane** | **HyperControl** | **Enterprise Governance**: Multi-repo compliance, organizational security audit policies, and executive oversight. |

---

## ⚡ Core Semantic Tabs

HyperKB organizes your repo's operational state into 5 grounded, single-noun tabs:

1. **`[1] Work` (The Present)**: Inspect working tree diffs, stage/unstage status, pre-commit risk verification gate, and live diagnostic output stream.
2. **`[2] Knowledge` (The Decisions & Specs)**: Architecture Decision Records (ADRs), specs, and project documentation in hierarchical Tree View (`[t]`) or flat List View.
3. **`[3] Directives` (The Invariants)**: Standing repository policies, architectural invariants, and pre-commit guardrails. Toggle active status with `[r]` or draft a new rule with `[n]`.
4. **`[4] Agents` (The Actors)**: Autonomous agent run telemetry, harness filters (`[h]`: `[ALL]`, `[CLAUDE]`, `[OPENCODE]`, `[CODEX]`), session quality scorecards (`[e]`), and authority grants (`[g]`).
5. **`[5] Settings` (The Environment)**: Discovered AI harnesses, FTS5 index maintenance, database snapshot backups, and theme selection.

---

## ⌨ Universal Command Dock & AI Integration

HyperKB eliminates floating modal palettes in favor of a permanently anchored **Universal Command Dock** across all tabs:

- **Quick Entry (`[/]` or `[Space]` or `Ctrl+P`)**: Focuses the dock with interactive slash command autocomplete.
- **Context-Aware AI Dispatch**: The dock automatically injects the active tab's selected risk, document, directive, or agent run into your prompt.
- **Slash Commands**:
  - `/check`: Audit staged and changed files against known risks and active directives.
  - `/audit`: Run KB anti-bloat, taxonomy consistency, and directive decay audit.
  - `/new`: Draft a new policy directive or repo invariant.
  - `/reindex`: Re-index markdown documents and frontmatter into SQLite full-text search.
  - `/bootstrap`: Mine git log history to discover regression hotspots and draft risk cards.
  - `/backup` / `/compact`: Create point-in-time database snapshot / VACUUM SQLite WAL journal.
  - `/claude`, `/opencode`, `/openai`, `/codex`, `/agent`: Dispatch prompt to specific harness with active repo context.
  - `/directives`, `/risks`, `/grants`, `/sessions`: Direct navigation shorthands.
  - `/clear`, `/help`: Stream maintenance and in-app quick reference.
- **Multi-line Prompts**: Press `Shift+Enter` (or end any line with `\`) to insert newlines without submitting.

---

## 📊 Session Quality Score Heuristic

HyperKB replaces subjective or opaque ratings with a transparent 0–100 heuristic scoring coding agent runs:

$$\text{Score} = \text{clamp}(100 - P_{\text{loops}} - P_{\text{friction}} - P_{\text{thrash}} + B_{\text{invariant}}, 5, 100)$$

- **Baseline Score (100)**: Clean, first-pass execution without defect.
- **Review Loops ($P_{\text{loops}}$, -15% each)**: Penalizes rework cycles where review rejected proposed changes.
- **Friction ($P_{\text{friction}}$, -15%)**: Deducted if execution failed unit tests, build checks, or syntax validation.
- **Tool Thrash ($P_{\text{thrash}}$)**: Deductions when file inspection-to-edit ratio exceeds 8:1 without making progress.
- **Invariant Compliance ($B_{\text{invariant}}$, +10%)**: Rewarded when active repo directives prevent a known regression.

---

## ⌨ Keyboard Navigation Reference

| Key | Context | Action |
| :--- | :--- | :--- |
| `[/]` or `[Space]` or `Ctrl+P` | Global | Focus Universal Command Dock & slash menu |
| `[1] - [5]` | Global | Switch tabs (`Work`, `Knowledge`, `Directives`, `Agents`, `Settings`) |
| `[Tab]` | Global | Toggle focus between List selection and Detail preview |
| `[j]` / `[k]` or `[↑]` / `[↓]` | Global | Navigate records, tree items, or settings |
| `[PgDn]` / `[PgUp]` | Global | Scroll detailed content preview, reader, or help |
| `[Enter]` | Global | Open selected item in Reader or drill down |
| `[o]` | Global | Open active document or directive in external editor (`$EDITOR` / VS Code / Cursor) |
| `[y]` | Global | Yank / Copy active document, directive markdown, or session scorecard |
| `[m]` | Global | Toggle Mouse Mode (ON: Click Nav / OFF: Native Terminal Drag-Select) |
| `[T]` | Global | Cycle visual theme (`Cyberpunk`, `Modern`, `Nord`, `Tokyo Night`, `Light`) |
| `[?]` or `[F1]` | Global | Open Quick Reference modal (`Esc` to close) |
| `[Esc]` | Global | Unfocus dock, dismiss modal, or return to list view |
| `[h]` | Tab 4 (Agents) | Cycle harness filter: `[ALL]` → `[CLAUDE]` → `[OPENCODE]` → `[CODEX]` → `[OTHER]` |
| `[e]` | Tab 4 (Agents) | Toggle Session Quality Score breakdown |
| `[g]` | Tab 4 (Agents) | Toggle between Agent Sessions and Authority Grants |
| `[n]` | Tab 3 / Tab 4 | Draft New Directive (Tab 3) or Issue Authority Grant (Tab 4) |
| `[r]` | Tab 3 / Tab 4 | Toggle Directive status (`● ACTIVE` ↔ `✕ RETIRED`) or Revoke Grant |
| `[c]` | Tab 2 / Tab 3 | Cycle taxonomy category filter pills |
| `[t]` | Tab 2 (Knowledge) | Toggle hierarchical Tree View vs flat List View |
| `[v]` | Reader View | Toggle formatted Markdown preview vs Raw text view |
| `[q]` or `Ctrl+C` | Global | Exit HyperKB cleanly, restoring terminal state |

---

## 🚀 CLI Commands

HyperKB includes a high-performance CLI for CI/CD pipelines, pre-commit hooks, and terminal workflows:

```bash
# Check working tree against active risks and directives
hyperkb check-work [--diff]

# Audit knowledge base and directive hygiene
hyperkb audit [--kb] [--directives]

# Manage directives
hyperkb directive list
hyperkb directive new

# Manage authority grants
hyperkb grant list
hyperkb grant issue <grantee> --scopes "src/**" --actions ProposeDecision,AcceptDecision
hyperkb grant revoke <grant-id>

# Agent telemetry & session briefings
hyperkb session briefing
hyperkb session list

# Launch Model Context Protocol (MCP) server
hyperkb mcp
```

---

## 🤖 Model Context Protocol (MCP) Integration

HyperKB includes a native MCP stdio server. Configure it in Claude Desktop, Cursor, or Antigravity:

```json
{
  "mcpServers": {
    "hyperkb": {
      "command": "hyperkb",
      "args": ["mcp"],
      "env": {
        "HYPERKB_COLLECTION": "default",
        "HYPERKB_PROFILE": "local"
      }
    }
  }
}
```

---

## 🛠 Building & Testing

```bash
# Check code hygiene and compilation
cargo check

# Run complete test suite (81 tests)
cargo test

# Build optimized production binary (< 3.2 MB)
cargo build --release
```

HyperKB is released under the Apache-2.0 license.
