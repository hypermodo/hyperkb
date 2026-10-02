use crate::ui::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

pub struct HelpModal;

impl HelpModal {
    pub fn modal_area(area: Rect) -> Rect {
        let modal_width = (area.width * 86 / 100).clamp(65, 115);
        let modal_height = (area.height * 90 / 100).clamp(20, 46);

        let horiz_pad = (area.width.saturating_sub(modal_width)) / 2;
        let vert_pad = (area.height.saturating_sub(modal_height)) / 2;

        Rect::new(
            area.x + horiz_pad,
            area.y + vert_pad,
            modal_width,
            modal_height,
        )
    }

    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let modal_area = Self::modal_area(area);

        let text = vec![
            Line::from(vec![
                Span::styled("HyperKB", t.title()),
                Span::styled(
                    " — Architecture Invariants, Knowledge Base & AI Agent Control Plane",
                    Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("1. SEMANTIC TABS", t.section_header())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [ 1 ] Work          ", t.key_badge()),
                Span::styled("Working tree diffs, pre-commit risk verification gate, diagnostic output stream", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ 2 ] Knowledge     ", t.key_badge()),
                Span::styled("Architecture Decision Records (ADRs), specs, and repo docs in Tree/List view", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ 3 ] Directives    ", t.key_badge()),
                Span::styled("Standing repo invariants, architectural policies, and pre-commit guardrails", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ 4 ] Agents        ", t.key_badge()),
                Span::styled("Autonomous agent run telemetry, harness filters, scorecards, and authority grants", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ 5 ] Settings      ", t.key_badge()),
                Span::styled("Local AI harness detection (Claude, OpenCode, Codex, Ollama), FTS5 index, DB options", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("2. UNIVERSAL COMMAND DOCK & AI HARNESS", t.section_header())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [ / ] or [ Space ]  ", t.key_badge()),
                Span::styled("Focus the anchored Universal Command Dock (interactive slash menu & LLM prompt)", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  /check              ", t.key_badge()),
                Span::styled("Audit staged and changed files against known risks and active directives", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  /audit              ", t.key_badge()),
                Span::styled("Run KB anti-bloat, taxonomy consistency, and directive decay audit", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  /new                ", t.key_badge()),
                Span::styled("Draft a new policy directive or repo invariant", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  /reindex            ", t.key_badge()),
                Span::styled("Re-index markdown documents and frontmatter into SQLite full-text search", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  /bootstrap          ", t.key_badge()),
                Span::styled("Mine git log history to discover regression hotspots and draft risk cards", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  /backup /compact    ", t.key_badge()),
                Span::styled("Create point-in-time database snapshot / VACUUM SQLite WAL journal", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  /claude, /opencode  ", t.key_badge()),
                Span::styled("Ask AI harnesses with active tab context automatically injected", Style::default().fg(t.accent())),
            ]),
            Line::from(vec![
                Span::styled("  Shift+Enter         ", t.key_badge()),
                Span::styled("Insert newline for multi-line prompts (or end line with '\\')", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ Esc ]             ", t.key_badge()),
                Span::styled("Unfocus Command Dock and return to active tab navigation", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("3. NAVIGATION & WORKFLOW KEYS", t.section_header())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [ Tab ]             ", t.key_badge()),
                Span::styled("Toggle focus between list pane and detail preview pane", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ j ] / [ k ]       ", t.key_badge()),
                Span::styled("Move selection up / down (or Arrow keys)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ PgUp ] / [ PgDn ] ", t.key_badge()),
                Span::styled("Scroll detail content, reader, or help documentation", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ Enter ]           ", t.key_badge()),
                Span::styled("Open selected document in Reader view or confirm selection", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ o ]               ", t.key_badge()),
                Span::styled("Open active document or directive in external editor ($EDITOR / VS Code / Cursor)", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  [ y ]               ", t.key_badge()),
                Span::styled("Yank / Copy active doc text, directive markdown, or session scorecard to clipboard", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ m ]               ", t.key_badge()),
                Span::styled("Toggle Mouse Mode: ON (Click Nav & Visual Drag-Copy) / OFF (Native Text Selection)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ h ] (Agents tab)  ", t.key_badge()),
                Span::styled("Cycle harness filter: [ALL] → [CLAUDE] → [OPENCODE] → [CODEX] → [OTHER]", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ e ] (Agents tab)  ", t.key_badge()),
                Span::styled("View Session Quality Score breakdown and methodology", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ g ] (Agents tab)  ", t.key_badge()),
                Span::styled("Toggle Agents view between Agent Sessions and Authority Grants", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ n ]               ", t.key_badge()),
                Span::styled("Create: New Directive (Tab 3) or Issue Authority Grant (Tab 4)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ r ]               ", t.key_badge()),
                Span::styled("Lifecycle: Toggle Active ↔ Retired (Directives) or Revoke Grant (Agents)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ c ]               ", t.key_badge()),
                Span::styled("Cycle category filter pills in Knowledge or Directives", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ t ]               ", t.key_badge()),
                Span::styled("Toggle Tree View vs flat List View in Knowledge tab", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ v ]               ", t.key_badge()),
                Span::styled("Toggle formatted Markdown preview vs Raw text in Reader tab", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ T ]               ", t.key_badge()),
                Span::styled("Cycle visual theme (Cyberpunk, Modern, Nord, Tokyo Night, Light)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ q ] / Ctrl+C      ", t.key_badge()),
                Span::styled("Exit HyperKB safely", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("4. SESSION QUALITY SCORE HEURISTIC", t.section_header())),
            Line::from(""),
            Line::from(Span::styled(
                "  HyperKB tracks an honest, pragmatic 0–100 heuristic score to identify thrashing or regressions in agent sessions:",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  • Baseline Score (100): ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled("Clean, first-pass execution without defect.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  • Review Loops (-15% each): ", Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)),
                Span::styled("Penalizes round-trip rework where human review had to reject changes.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  • Friction (-15%): ", Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)),
                Span::styled("Deduction if run failed unit tests, build checks, or syntax validation.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  • Tool Thrash: ", Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)),
                Span::styled("Deductions when file inspection-to-edit ratio exceeds 8:1 without progress.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  • Invariant Guardrail (+10%): ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled("Bonus when active repo directives prevent a known regression.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
        ];

        let total_lines = text.len();
        let current_line = (app.help_scroll + 1).min(total_lines);
        let scroll_pct = ((current_line as f64 / total_lines as f64) * 100.0) as usize;

        frame.render_widget(Clear, modal_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(3, 3, 1, 1))
            .title(Span::styled(
                " [?] HyperKB System Documentation & Quick Reference ",
                t.title(),
            ))
            .title_bottom(Line::from(vec![
                Span::styled(format!(" [Line {}/{} • {}%] ", current_line, total_lines, scroll_pct), Style::default().fg(t.text_muted())),
                Span::styled(" Scroll: ", Style::default().fg(t.text_muted())),
                Span::styled("[j / k / PgDn] ", t.key_badge()),
                Span::styled("• Copy: ", Style::default().fg(t.text_muted())),
                Span::styled("[y] ", t.key_badge()),
                Span::styled("• Close: ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] or [?] ", t.key_badge()),
            ]));

        let p = Paragraph::new(text)
            .block(block)
            .scroll((app.help_scroll as u16, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(p, modal_area);
    }

    pub fn help_text_plain() -> String {
        let lines = [
            "HyperKB — Architecture Invariants, Knowledge Base & AI Agent Control Plane",
            "==========================================================================",
            "",
            "1. SEMANTIC TABS",
            "----------------",
            "  [ 1 ] Work          Working tree diffs, pre-commit risk verification gate, diagnostic stream",
            "  [ 2 ] Knowledge     Architecture Decision Records (ADRs), specs, and repo docs in Tree/List view",
            "  [ 3 ] Directives    Standing repo invariants, architectural policies, and pre-commit guardrails",
            "  [ 4 ] Agents        Autonomous agent run telemetry, harness filters, scorecards, and authority grants",
            "  [ 5 ] Settings      Local AI harness detection (Claude, OpenCode, Codex, Ollama), FTS5 index, DB options",
            "",
            "2. UNIVERSAL COMMAND DOCK & AI HARNESS",
            "--------------------------------------",
            "  [ / ] or [ Space ]  Focus the anchored Universal Command Dock (slash commands & LLM queries)",
            "  /check              Audit staged and changed files against known risks and active directives",
            "  /audit              Run KB anti-bloat, taxonomy consistency, and directive decay audit",
            "  /new                Draft a new policy directive or repo invariant",
            "  /reindex            Re-index markdown documents and frontmatter into SQLite full-text search",
            "  /bootstrap          Mine git log history to discover regression hotspots and draft risk cards",
            "  /backup /compact    Create point-in-time database snapshot / VACUUM SQLite WAL journal",
            "  /claude, /opencode  Ask AI harnesses with active tab context automatically injected",
            "  Shift+Enter         Insert newline for multi-line prompts (or end line with '\\')",
            "  [ Esc ]             Unfocus Command Dock and return to active tab navigation",
            "",
            "3. NAVIGATION & WORKFLOW KEYS",
            "-----------------------------",
            "  [ Tab ]             Toggle focus between list pane and detail preview pane",
            "  [ j ] / [ k ]       Move selection up / down (or Arrow keys)",
            "  [ PgUp ] / [ PgDn ] Scroll detail content, reader, or help documentation",
            "  [ Enter ]           Open selected document in Reader view or confirm selection",
            "  [ o ]               Open active document or directive in external editor ($EDITOR / VS Code / Cursor)",
            "  [ y ]               Yank / Copy active doc text, directive markdown, or session scorecard to clipboard",
            "  [ m ]               Toggle Mouse Mode: ON (Click Nav & Visual Drag-Copy) / OFF (Native Text Selection)",
            "  [ h ] (Agents tab)  Cycle harness filter: [ALL] -> [CLAUDE] -> [OPENCODE] -> [CODEX] -> [OTHER]",
            "  [ e ] (Agents tab)  View Session Quality Score breakdown and methodology",
            "  [ g ] (Agents tab)  Toggle Agents view between Agent Sessions and Authority Grants",
            "  [ n ]               Create: New Directive (Tab 3) or Issue Authority Grant (Tab 4)",
            "  [ r ]               Lifecycle: Toggle Active <-> Retired (Directives) or Revoke Grant (Agents)",
            "  [ c ]               Cycle category filter pills in Knowledge or Directives",
            "  [ t ]               Toggle Tree View vs flat List View in Knowledge tab",
            "  [ v ]               Toggle formatted Markdown preview vs Raw text in Reader tab",
            "  [ T ]               Cycle visual theme (Cyberpunk, Modern, Nord, Tokyo Night, Light)",
            "  [ q ] / Ctrl+C      Exit HyperKB safely",
            "",
            "4. SESSION QUALITY SCORE HEURISTIC",
            "----------------------------------",
            "  Score: 0 to 100 quality heuristic tracking coding agent efficiency:",
            "  • Baseline Score (100): Clean, first-pass execution without defect.",
            "  • Review Loops (-15% each): Penalizes rework cycles where review rejected changes.",
            "  • Friction (-15%): Deduction if run failed unit tests, build checks, or syntax validation.",
            "  • Tool Thrash: Deductions when file inspection-to-edit ratio exceeds 8:1 without progress.",
            "  • Invariant Guardrail (+10%): Bonus when active repo directives prevent a known regression.",
        ];
        lines.join("\n")
    }
}
