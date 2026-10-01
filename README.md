# HyperKB (`hyperkb-rs`)

> **Autonomous Knowledge Base, Governance & Architecture Telemetry Cockpit**  
> *Engineered for Modern IDE Developers & AI-Native "Vibe Coders"*

HyperKB is a standalone, terminal-agnostic, zero-latency governance cockpit designed to bridge human developers and co-collaborating AI coding agents. It provides real-time invariant enforcement, standing policy directives, bounded agent authority delegation, risk verification gates, and empirical coding effectiveness telemetry.

---

## 🏛 Product Responsibility Boundaries

HyperKB is explicitly architected with clear boundaries across the development and governance lifecycle:

| Plane | Tool | Primary Purpose & Responsibilities |
| :--- | :--- | :--- |
| **Local Cockpit** | **HyperKB** | **Governance & Guardrails**: Standing directives, zero-trust agent authority grants, pre-commit risk gates (`check-work`), empirical session scoring telemetry, fast metadata lookup. Zero context switching, <5ms query response. |
| **Authoring Plane** | **External IDE** (`[o]`) | **Prose & Implementation**: Long-form markdown ADR drafting, architecture prose authoring, source code implementation. Pressing `[o]` inside HyperKB opens the active document in `$EDITOR` (VS Code, Cursor, Zed, Neovim). |
| **Macro Plane** | **HyperControl** | **Enterprise Governance**: Multi-repo compliance, organizational security audit policies, SOC2/ISO guardrails, and executive CISO oversight. |

---

## ⚡ Key Features & Workflows

### 1. In-TUI Directive Lifecycle Management (`[3] Directives`)
Directives enforce standing invariant rules and coding standards that autonomous agents must adhere to during task execution.
- **Fast Inline Creation (`[n]`)**: Draft standing directives directly inside the TUI with Title, Category, Scope pattern, Enforcement level (`mandatory` vs. `advisory`), and Invariant Statement.
- **Instant Status Toggling (`[r]`)**: Toggle directives between `● ACTIVE` and `✕ RETIRED` in real-time. Retired directives are immediately excluded from agent briefings and pre-commit checks.
- **Taxonomy Filtering (`[c]`)**: Filter directives across core architectural categories (`architecture`, `behavior`, `deployment`, `security`).
- **One-Key Markdown Yank (`[y]`)**: Copy complete directive markdown prompts (including YAML frontmatter) directly to your clipboard for instant agent prompt injection.

### 2. Autonomous Agent & Sub-Agent Authority Grants (`[4] Governance & Sessions`)
HyperKB provides a harness- and LLM-agnostic delegation system. Delegate authority with mathematical and capability bounds stored as atomic JSON in `.hyperkb/grants/<id>.json`.
- **Mode Toggle (`[g]`)**: Switch instantly between Agent Sessions Telemetry and Authority Grants Management.
- **Issue Grant Wizard (`[n]`)**: Issue bounded authority to agent delegates (e.g. `frontend-subagent`, `code-reviewer`, `security-auditor`) with presets:
  - *Frontend & UI Specialist* (`src/ui/**`, ProposeDecision, AutoRepair, max diff 250)
  - *Documentation & Governance* (`docs/**`, ProposeDecision, AcceptDecision, max diff 400)
  - *Full Workspace Autonomy* (`*`, all capabilities, max diff 500)
  - *Conservative Reviewer* (`src/**`, ProposeDecision only, diff 200)
- **Time-to-Live (TTL)**: Configurable expiration (1h, 4h, 8h, 24h, or permanent).
- **Instant Revocation (`[r]`)**: Immediately revoke any grant from disk.
- **UUID & Token Yank (`[y]`)**: Auto-copies grant token UUID or JSON specification to system clipboard.

### 3. Modern Action Palette (`[Space]` or `[Ctrl+P]`)
An instant launcher modal inspired by modern IDE command palettes and Raycast:
- **`check-work`**: Audit git changes against active risks and standing directives.
- **`new-directive`**: Launch inline directive creation wizard.
- **`issue-grant`**: Launch agent authority delegation wizard.
- **`audit-kb`**: Audit repository knowledge base for bloat, broken links, and schema hygiene.
- **`open-editor`**: Launch external IDE (`$EDITOR`) on the currently selected document.
- **`backup`**: Create atomic snapshot backup in `.hyperkb/backups/`.
- **`compact`**: Run database `VACUUM` and truncate WAL journals.
- **`toggle-theme`**: Cycle through visual themes (Cyberpunk, Modern, Nord, Tokyo Night, Light).
- **`toggle-mouse`**: Toggle between native terminal text selection and in-TUI click navigation.

### 4. External Editor Jump (`[o]`)
HyperKB keeps the cockpit lightweight and focused on governance. Press `[o]` on any decision, directive, risk, or session to spawn your preferred editor (`$VISUAL`, `$EDITOR`, or `code`) without tearing down the TUI.

### 5. Universal Terminal Text Selection & OSC 52 Clipboard
Designed to work across all terminal emulators (macOS Terminal, iTerm2, Alacritty, Kitty, Windows Terminal, tmux, SSH):
- **Native Drag Selection (Default)**: Terminal mouse capture is OFF by default. Simply drag your mouse and copy text (`Cmd+C` / `Ctrl+Shift+C`) without holding modifier keys.
- **In-TUI Visual Drag & Auto-Copy**: Press `[m]` to turn Mouse Mode ON. Dragging produces visual highlight boxes; releasing auto-copies snippet via universal OSC 52.
- **Instant 1-Key Yank (`[y]`)**: Press `[y]` on any view to copy markdown content, grant tokens, scorecards, or documentation.

---

## ⌨ Keyboard Shortcuts Reference

| Key | Context | Action |
| :--- | :--- | :--- |
| `[Space]` or `Ctrl+P` | Global | Open Action Palette & Tool Launcher |
| `[o]` | Global | Open active document in external editor / IDE |
| `[1] - [5]` | Global | Switch tabs (Work, Explore, Directives, Governance & Sessions, Settings) |
| `[Tab]` | Global | Toggle focus between List selection and Detail preview |
| `[j]` / `[k]` or `[↑]` / `[↓]` | Global | Navigate records, tree items, or action palette |
| `[PgDn]` / `[PgUp]` | Global | Scroll detailed content preview, reader, or help |
| `[y]` | Global | Yank / Copy active content, markdown prompt, or token |
| `[m]` | Global | Toggle Mouse Mode (ON: Click Nav / OFF: Native Selection) |
| `[T]` | Global | Cycle visual theme (Cyberpunk, Modern, Nord, Tokyo Night, Light) |
| `[?]` or `[F1]` | Global | Open System Documentation modal |
| `[Esc]` | Global | Dismiss modal, exit search, or return to list view |
| `[n]` | Tab 3 (Directives) | Draft new directive wizard |
| `[r]` | Tab 3 (Directives) | Toggle directive status (`● ACTIVE` ↔ `✕ RETIRED`) |
| `[c]` | Tab 3 (Directives) | Cycle directive taxonomy category |
| `[g]` | Tab 4 (Sessions) | Toggle between Sessions Telemetry and Authority Grants |
| `[n]` | Tab 4 (Grants) | Issue new agent authority grant wizard |
| `[r]` | Tab 4 (Grants) | Revoke selected authority grant |
| `[t]` | Tab 2 (Explore) | Toggle directory Tree View vs flat List View |
| `[/]` | Tab 2 (Explore) | Real-time full-text search across knowledge base |

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

## 🤖 MCP Server Integration

HyperKB includes a native Model Context Protocol (MCP) stdio server. Configure it in Claude Desktop, Cursor, or Antigravity:

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

# Run complete test suite (72+ tests)
cargo test

# Build optimized production binary (< 3.2 MB)
cargo build --release
```

HyperKB is released under the Apache-2.0 license.
