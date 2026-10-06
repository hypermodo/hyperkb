# HyperKB (Universal NPM Distribution)

Universal cross-platform distribution package for **HyperKB** — the local developer knowledge hub, anti-churn governor, and MCP server for AI coding agents.

This package automatically bundles and executes the pre-compiled, statically linked native binary for your operating system and architecture. **Zero Rust, Cargo, or compiler toolchains are required.**

---

## 🚀 Quick Start

### 1. Run Directly with NPX (No Installation Required)

```bash
# Print warm-start context briefing (clamped <35 lines)
npx -y hyperkb brief

# View active project health and critical path lock
npx -y hyperkb status

# Run as Model Context Protocol (MCP) server
npx -y hyperkb mcp
```

### 2. Install Globally as a System CLI

```bash
npm install -g hyperkb
```

Once installed, use `hyperkb` from any directory or terminal:

```bash
hyperkb brief
hyperkb status
hyperkb check-work
hyperkb mcp
```

---

## 🤖 AI Harness Setup

### OpenCode
Add to `opencode.json` (or `~/.config/opencode/opencode.json`):

```json
{
  "mcp": {
    "hyperkb": {
      "command": "hyperkb",
      "args": ["mcp"]
    }
  }
}
```
*(Or use `"command": "npx", "args": ["-y", "hyperkb", "mcp"]` if not globally installed).*

### Claude Code
Add to `.mcp.json` or run:
```bash
claude mcp add hyperkb npx -y hyperkb mcp
```

### Cursor & VS Code
Add to `.cursor/mcp.json`:
```json
{
  "mcpServers": {
    "hyperkb": {
      "command": "npx",
      "args": ["-y", "hyperkb", "mcp"]
    }
  }
}
```

### Google Antigravity IDE
Add to `mcp_config.json`:
```json
{
  "mcpServers": {
    "hyperkb": {
      "command": "npx",
      "args": ["-y", "hyperkb", "mcp"]
    }
  }
}
```

---

## 💬 Supported In-Harness Slash Commands (MCP Prompts)

When connected to any harness supporting standard MCP Prompts, HyperKB automatically provides (prefixed with `hkb-` to prevent collisions with harness-native commands and allow effortless typing without the shift key):

- `/hkb-brief`: Injects thin warm-start briefing (<35 lines) enforcing Rule of 5 and critical path lock. *(Aliases: `/hkb_brief`, `/brief`)*
- `/hkb-status`: Renders active project health, active task lock (`🔒 <task>`), blockers, and exit criteria. *(Aliases: `/hkb_status`, `/status`)*
- `/hkb-verify`: Runs deterministic project exit criteria verification. *(Aliases: `/hkb_verify`, `/verify`)*
- `/hkb-task-next`: Progresses current critical path task without leaving chat. *(Aliases: `/hkb-next`, `/hkb_task_next`, `/task_next`)*
- `/hkb-defer`: Jails tangential side-quests into project `BACKLOG.md`. *(Aliases: `/hkb_defer`, `/defer`)*
- `/hkb-metrics`: Displays session effectiveness and review loop scorecard. *(Aliases: `/hkb_metrics`, `/metrics`)*
- `/hkb-claude`: Queries Anthropic Claude peer model directly from your current harness. *(Aliases: `/hkb_claude`, `/claude`)*
- `/hkb-chatgpt`: Queries OpenAI ChatGPT peer model directly from your current harness. *(Aliases: `/hkb_chatgpt`, `/chatgpt`)*
- `/hkb-gemini`: Queries Google Gemini peer model directly from your current harness. *(Aliases: `/hkb_gemini`, `/gemini`)*
