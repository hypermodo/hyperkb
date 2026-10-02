# HyperKB Architecture

> Fast, resource-efficient local developer knowledge & proactive risk engine

HyperKB is a standalone, local-first control plane bridging human developers and autonomous AI coding agents. It provides sub-millisecond retrieval, active pre-commit risk gates, bounded authority delegation, and transparent session telemetry without external service dependencies.

---

## 🏛 System Layers

HyperKB is organized into four decoupled architectural layers:

```
┌─────────────────────────────────────────────────────────────┐
│                       Transport Layer                       │
│     • Interactive Terminal TUI (Ratatui + Crossterm)        │
│     • Headless MCP Server (JSON-RPC 2.0 over stdio)         │
│     • Headless CLI Commands (Clap)                          │
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                        Domain Layer                         │
│     • Actor & AuthorityGrant (Bounded Agent Delegation)     │
│     • Policy Directives & Rule-of-5 Engine                  │
│     • Architectural Risk Engine & Path Hazard Matching      │
│     • Session Telemetry & 0–100 Quality Score Heuristic     │
│     • Hybrid Logical Clock (HLC) & Document Provenance      │
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                         Core Layer                          │
│     • Scanner: Fast Markdown & Frontmatter Indexing         │
│     • Archeology: Git History Mining & Hotspot Discovery    │
│     • Linter: KB Bloat, Depth & Schema Enforcement          │
│     • Git: Pre-commit Hook & Working Tree Diff Analysis     │
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                       Storage Layer                         │
│     • Embedded SQLite 3 with FTS5 Full-Text Search          │
│     • Write-Ahead Logging (WAL) & In-Memory Fast Fallback   │
│     • Point-in-time Snapshot Backups & Automated Rotation   │
└─────────────────────────────────────────────────────────────┘
```

---

## ⚡ Zero-Overhead Reactive TUI

Unlike polling loops that consume CPU cycles in the background, HyperKB’s user interface implements an **event-driven reactive loop** (`src/ui/mod.rs`):

* **Event Wait with Tick Timeout**: The UI blocks until a terminal key, mouse, or resize event arrives, or until a low-frequency 250ms tick occurs for animations.
* **Dirty State Tracking**: Frames are only rendered when state actually changes (`app.is_dirty`).
* **Idle Resource Usage**: Consistently measures **0.0% CPU** and **<25 MB RSS** in idle terminal tabs.

---

## 🛡 Proactive Risk Prevention (`check_work`)

HyperKB shifts architectural governance from reactive post-commit code review to active pre-commit interception:

1. **Risk Registry**: Risk cards cite file path glob patterns (e.g., `src/storage/**`, `crates/ado/**`).
2. **Path Matching**: Before an agent or developer commits changes, `check-work --staged` matches git diff paths against active risks.
3. **Pre-commit Gate**: A native `.git/hooks/pre-commit` script halts commits touching cited hazard areas unless explicitly acknowledged with a documented rationale (`hyperkb ack <risk-id>`).

---

## 📊 0–100 Session Quality Score

HyperKB replaces opaque ratings with an honest, deterministic heuristic tracking agent execution efficiency:

$$\text{Score} = \text{clamp}(100 - P_{\text{loops}} - P_{\text{friction}} - P_{\text{thrash}} + B_{\text{invariant}}, 5, 100)$$

* **Review Loops ($P_{\text{loops}}$, -15% each)**: Penalizes cycles where review rejected changes.
* **Friction ($P_{\text{friction}}$, -15%)**: Deducted if execution broke unit tests, build checks, or syntax.
* **Tool Thrash ($P_{\text{thrash}}$)**: Deducted when inspection-to-edit ratio exceeds 8:1 without making progress.
* **Invariant Compliance ($B_{\text{invariant}}$, +10%)**: Rewarded when active repo directives prevent a known regression.
