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

## 📦 Installation & Quick Start

HyperKB is available as a zero-cargo, multi-platform precompiled distribution via **GitHub Packages** (`@hypermodo/hyperkb`), standalone release archives on [GitHub Releases](https://github.com/hypermodo/hyperkb/releases), or via Cargo.

### Method 1: Zero-Cargo via GitHub Packages (Recommended)
1. **One-Time Setup (`~/.npmrc`)**:
   ```bash
   echo "@hypermodo:registry=https://npm.pkg.github.com" >> ~/.npmrc
   npm config set //npm.pkg.github.com/:_authToken $(gh auth token)
   ```
2. **Add to Project or Run with NPX**:
   ```bash
   npm install -D @hypermodo/hyperkb   # Save as project dependency
   npx -y @hypermodo/hyperkb brief      # Instant context briefing (<35 lines)
   npx -y @hypermodo/hyperkb status     # Active project status & task lock
   npx -y @hypermodo/hyperkb mcp        # Run as MCP stdio server
   ```

### Method 2: Standalone Release Binaries
Pre-compiled binaries for **macOS (Apple Silicon & Intel)**, **Linux (x64 & ARM64 MUSL)**, and **Windows (x64)** are attached to every [GitHub Release](https://github.com/hypermodo/hyperkb/releases):
```bash
# macOS Apple Silicon
curl -sL https://github.com/hypermodo/hyperkb/releases/download/v0.1.1/hyperkb-v0.1.1-aarch64-apple-darwin.tar.gz | tar -xz && sudo mv hyperkb /usr/local/bin/
```

### Method 3: Build from Source (Cargo)
```bash
cargo install --path .
```

---

## 🤖 AI Harness Setup & Deployment Topologies

HyperKB supports three generic topologies across all AI coding harnesses (**OpenCode, Claude Code, Google Antigravity, Cursor, VS Code, Codex, Aider**):
- **1. Embedded (Per-Repo)**: `hyperkb.json` in repository root. Omit `--root` (defaults to `.`).
- **2. Multi-Repo Suite (Parent Directory Hub)**: Place configuration in the parent directory (`workspace-group/opencode.json` or `.mcp.json`) pointing `--root` to the shared `system-kb/`. All child repos automatically inherit it with zero per-repo configuration!
- **3. Global Machine Hub**: Configure in global user settings with `--root /path/to/global-system-kb`.

*(See [docs/MCP_GUIDE.md](docs/MCP_GUIDE.md) for complete multi-repo diagrams and deep architectural details).*

### OpenCode
Add to `opencode.json` (or parent folder `opencode.json` for multi-repo suites):
```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "hyperkb": {
        "type": "local",
        "command": ["npx", "-y", "@hypermodo/hyperkb", "mcp"]
      }
    }
  }
}
```
*(For shared multi-repo parent directories or global hubs, append `"--root", "/path/to/system-kb"` to `command`).*

### Claude Code CLI
Add to `.mcp.json` or run:
```bash
claude mcp add hyperkb npx -y @hypermodo/hyperkb mcp
# Or with shared parent hub: claude mcp add hyperkb npx -y @hypermodo/hyperkb mcp --root /path/to/system-kb
```

### Cursor & VS Code
Add to `.cursor/mcp.json` (or workspace root for multi-repo):
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

### Google Antigravity IDE
Add to `.agents/mcp_config.json`:
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

## 💬 In-Harness Ergonomic Commands (MCP Prompts)

All connected harnesses gain instant access to HyperKB commands prefixed with `hkb-` (designed in kebab-case so typing requires no Shift key and prevents namespace collisions with harness-native slash commands):

| Command | Aliases | Description |
| :--- | :--- | :--- |
| `/hkb-brief` | `/hkb_brief`, `/brief` | Inject thin warm-start context briefing (<35 lines) enforcing Rule of 5 and critical path lock. |
| `/hkb-status` | `/hkb_status`, `/status` | Render active project health, task lock (`🔒 <task>`), blockers, and exit criteria. |
| `/hkb-verify` | `/hkb_verify`, `/verify` | Run deterministic exit criteria verification script for active project. |
| `/hkb-task-next` | `/hkb-next`, `/task_next` | Progress active critical path task directly from conversation. |
| `/hkb-defer` | `/hkb_defer`, `/defer` | Formally jail tangential side-quests into project `BACKLOG.md` with rationale. |
| `/hkb-metrics` | `/hkb_metrics`, `/metrics` | Display 4-phase velocity scorecard, friction warnings, and review loop oscillations. |
| `/hkb-claude` | `/claude` | Query Anthropic Claude peer model with active repo governance context. |
| `/hkb-chatgpt` | `/chatgpt` | Query OpenAI ChatGPT peer model with active repo governance context. |
| `/hkb-gemini` | `/gemini` | Query Google Gemini peer model with active repo governance context. |

---

## 🚀 CLI Commands

HyperKB includes a high-performance CLI for CI/CD pipelines, pre-commit hooks, and terminal workflows:

```bash
# Check working tree against active risks and directives
hyperkb check-work [--diff]

# Audit knowledge base and directive hygiene
hyperkb audit [--kb] [--directives]

# Manage directives and invariants
hyperkb directive list

# Manage authority grants for autonomous agents
hyperkb grant issue <grantee> --scopes "src/**" --actions ProposeDecision,AcceptDecision
hyperkb grant revoke <grant-id>

# Agent telemetry & session briefings
hyperkb session briefing

# Launch Model Context Protocol (MCP) server
hyperkb mcp
```

---

## 🛠 Building & Testing

```bash
# Check code hygiene and compilation
cargo check

# Run complete test suite (111 tests across macOS, Linux, and Windows)
cargo test

# Build optimized production binary (< 3.2 MB)
cargo build --release
```

HyperKB is released under the Apache-2.0 license.
