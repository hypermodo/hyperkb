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
                    " — Autonomous Knowledge Base, Governance & Architecture Telemetry",
                    Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("1. KEYBOARD NAVIGATION & TABS", t.section_header())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [ 1 ] - [ 5 ]        ", t.key_badge()),
                Span::styled("Switch tabs (Work, Explore, Directives, Governance & Sessions, Settings)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ Space ] / Ctrl+P   ", t.key_badge()),
                Span::styled("Open KB Lifecycle & Operations Palette (check-work, directives, grants, index, backup, compact)", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  [ o ]                ", t.key_badge()),
                Span::styled("Jump to External Editor: Open active doc in $EDITOR, VS Code, Cursor, or IDE", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  [ Tab ]              ", t.key_badge()),
                Span::styled("Toggle focus between List selection and Detail preview", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ j ] / [ k ]        ", t.key_badge()),
                Span::styled("Navigate records and tree items up / down (or Arrow keys)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ PgUp ] / [ PgDn ]  ", t.key_badge()),
                Span::styled("Scroll detailed content preview, reader, and documentation", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ Enter ]            ", t.key_badge()),
                Span::styled("Open document in Reader, drill down into folders, or confirm", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ Esc ]              ", t.key_badge()),
                Span::styled("Go back to previous view / dismiss active search or modal", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("2. WORKFLOW ACTIONS & SHORTCUTS", t.section_header())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [ Space ] / Ctrl+P   ", t.key_badge()),
                Span::styled("KB Lifecycle Palette: Instant modal launcher for policy directives, grants, audits, indexing & DB operations", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ o ]                ", t.key_badge()),
                Span::styled("External Editor Jump: Launch external IDE ($EDITOR) for heavy prose editing", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ y ]                ", t.key_badge()),
                Span::styled("Yank / Copy active doc, directive prompt, grant token/JSON, or scorecard", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ m ]                ", t.key_badge()),
                Span::styled("Toggle Mouse Mode: ON (Click Nav & Visual Drag-Copy) / OFF (Native Text Selection)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ n ]                ", t.key_badge()),
                Span::styled("Create / Issue: Fast inline wizard for New Directive (Tab 3) or Authority Grant (Tab 4)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ r ]                ", t.key_badge()),
                Span::styled("Lifecycle Status: Toggle Active ↔ Retired (Directives) or Revoke Grant (Governance)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ g ]                ", t.key_badge()),
                Span::styled("Toggle Governance Mode: Switch between Agent Sessions and Authority Grants (Tab 4)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ c ]                ", t.key_badge()),
                Span::styled("Cycle taxonomy filters: Categories in Explore (Decisions/Risks) or Directives", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ t ]                ", t.key_badge()),
                Span::styled("Toggle Tree View vs flat List View in Explore tab", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ / ]                ", t.key_badge()),
                Span::styled("Search knowledge base documents by title, keyword, or path", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ v ]                ", t.key_badge()),
                Span::styled("Toggle formatted Markdown preview vs RAW file view (Reader tab)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ T ]                ", t.key_badge()),
                Span::styled("Cycle visual theme (Cyberpunk, Modern, Nord, Tokyo Night, Light)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ q ] / Ctrl+C       ", t.key_badge()),
                Span::styled("Exit HyperKB safely", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("3. PRODUCT RESPONSIBILITY BOUNDARIES", t.section_header())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  • HyperKB (Developer Cockpit): ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled("Zero-latency governance for devs and co-collaborating agents.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(Span::styled(
                "    Manages standing directives, invariant guardrails, agent authority grants, verification",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(Span::styled(
                "    risk gates (check-work), and empirical telemetry without context switching.",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  • External IDE / Editor ([o]): ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled("Prose authoring & rich implementation.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(Span::styled(
                "    Long-form markdown writing, ADR drafting, and code implementation belong in your favored",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(Span::styled(
                "    IDE (VS Code, Cursor, Zed, Neovim, etc.). Press [o] on any document to launch it instantly.",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  • HyperControl (Future Enterprise): ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                Span::styled("Macro-governance plane for CISO & business leadership.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(Span::styled(
                "    Multi-repo compliance, organizational security audit policies, and executive dashboards.",
                Style::default().fg(t.text_muted()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("4. UNIVERSAL TEXT SELECTION & CLIPBOARD COPYING", t.section_header())),
            Line::from(""),
            Line::from(Span::styled(
                "  HyperKB is a standalone, terminal-agnostic tool designed to work effortlessly across",
                Style::default().fg(t.text_muted()),
            )),
            Line::from(Span::styled(
                "  all terminal emulators (macOS Terminal, iTerm2, Alacritty, Kitty, Windows Terminal, SSH, tmux, etc.).",
                Style::default().fg(t.text_muted()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  • Method 1 (Native Drag Selection - Default): ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled("Simply click and drag your mouse", Style::default().fg(t.text_primary())),
            ]),
            Line::from(Span::styled(
                "    By default, terminal mouse capture is OFF. You can highlight any text anywhere on",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(Span::styled(
                "    screen with your mouse and copy it (Cmd+C / Ctrl+Shift+C) without requiring Shift or Option keys.",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  • Method 2 (In-TUI Visual Drag & Auto-Copy): ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled("Press ", Style::default().fg(t.text_primary())),
                Span::styled("[ m ]", t.key_badge()),
                Span::styled(" to enable Mouse Mode", Style::default().fg(t.text_primary())),
            ]),
            Line::from(Span::styled(
                "    When Mouse Mode is toggled ON, clicking navigates lists/tabs, and dragging your mouse",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(Span::styled(
                "    over any text displays an instant visual highlight. Releasing the mouse automatically",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(Span::styled(
                "    copies the selected snippet to your clipboard via native clipboard and universal OSC 52.",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  • Method 3 (Instant 1-Key Yank): ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled("Press ", Style::default().fg(t.text_primary())),
                Span::styled("[ y ]", t.key_badge()),
                Span::styled(" to copy full document, directive markdown, or grant token", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("5. SCIENTIFIC TELEMETRY & CODING EFFECTIVENESS", t.section_header())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Formula: ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                Span::styled("Score = clamp(100 - P_loops - P_friction - P_thrash + B_hazard, 5, 100)", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  • Baseline Score: ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled("100% clean baseline execution without defect.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  • Review Loops (P_loops): ", Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)),
                Span::styled("-15% per review oscillation (heavily penalizes rework thrash).", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  • Iteration Friction (P_friction): ", Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)),
                Span::styled("-15% deduction if initial execution failed tests or compilation.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  • Tool Thrash (P_thrash): ", Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)),
                Span::styled("Deductions applied when inspection-to-edit ratio exceeds 8:1.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  • Hazard Prevention (B_hazard): ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled("+10% bonus when active directives intercept known regressions.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("6. CLI COMMANDS & MCP SERVER", t.section_header())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  hyperkb check-work [--diff]       ", t.key_badge()),
                Span::styled("Audit git changes against active risks & directives", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  hyperkb audit [--kb/--directives] ", t.key_badge()),
                Span::styled("Verify document bloat, depth, and schema validity", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  hyperkb directive list/new        ", t.key_badge()),
                Span::styled("Manage team policies and standing guardrails", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  hyperkb session briefing          ", t.key_badge()),
                Span::styled("Inspect agent telemetry and generate briefings", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  hyperkb mcp                       ", t.key_badge()),
                Span::styled("Launch stdio MCP server for AI coding agents", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
        ];

        let total_lines = text.len();
        let current_line = (app.help_scroll + 1).min(total_lines);
        let scroll_pct = ((current_line as f64 / total_lines as f64) * 100.0) as usize;

        // 1. Clear background behind modal
        frame.render_widget(Clear, modal_area);

        // 2. Render Modal Block with theme styling and generous padding
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(3, 3, 1, 1))
            .title(Span::styled(
                " [?] HyperKB System Documentation & Shortcuts ",
                t.title(),
            ))
            .title_bottom(Line::from(vec![
                Span::styled(format!(" [Line {}/{} • {}%] ", current_line, total_lines, scroll_pct), Style::default().fg(t.text_muted())),
                Span::styled(" Scroll: ", Style::default().fg(t.text_muted())),
                Span::styled("[j / k / PgDn] ", t.key_badge()),
                Span::styled("• Copy All: ", Style::default().fg(t.text_muted())),
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
            "HyperKB — Autonomous Knowledge Base, Governance & Architecture Telemetry",
            "==========================================================================",
            "",
            "1. KEYBOARD NAVIGATION & TABS",
            "-----------------------------",
            "  [ 1 ] - [ 5 ]        Switch tabs (Work, Explore, Directives, Governance & Sessions, Settings)",
            "  [ Space ] / Ctrl+P   Open KB Lifecycle & Operations Palette (check-work, directives, grants, index, backup, compact)",
            "  [ o ]                Jump to External Editor: Open active file in $EDITOR / VS Code / Cursor",
            "  [ Tab ]              Toggle focus between List selection and Detail preview",
            "  [ j ] / [ k ]        Navigate records and tree items up / down (or Arrow keys)",
            "  [ PgUp ] / [ PgDn ]  Scroll detailed content preview, reader, and documentation",
            "  [ Enter ]            Open document in Reader, drill down into folders, or confirm",
            "  [ Esc ]              Go back to previous view / dismiss active search or modal",
            "  [ / ]                Search knowledge base documents by title, keyword, or path",
            "  [ v ]                Toggle formatted Markdown preview vs RAW file view",
            "  [ T ]                Cycle visual theme (Cyberpunk, Modern, Nord, Tokyo Night, Light)",
            "  [ q ] / Ctrl+C       Exit HyperKB safely",
            "",
            "2. WORKFLOW ACTIONS & SHORTCUTS",
            "-------------------------------",
            "  [ Space ] / Ctrl+P   KB Lifecycle Palette: Instant modal launcher for policy directives, grants, audits, indexing & DB operations",
            "  [ o ]                External Editor Jump: Launch external IDE ($EDITOR) for heavy prose editing",
            "  [ y ]                Yank / Copy active doc, directive prompt, grant token/JSON, or scorecard",
            "  [ m ]                Toggle Mouse Mode: ON (Click Nav & Visual Drag-Copy) / OFF (Native Text Selection)",
            "  [ n ]                Create / Issue: Fast inline wizard for New Directive (Tab 3) or Authority Grant (Tab 4)",
            "  [ r ]                Lifecycle Status: Toggle Active ↔ Retired (Directives) or Revoke Grant (Governance)",
            "  [ g ]                Toggle Governance Mode: Switch between Agent Sessions and Authority Grants (Tab 4)",
            "  [ c ]                Cycle taxonomy filters: Categories in Explore (Decisions/Risks) or Directives",
            "  [ t ]                Toggle Tree View vs flat List View in Explore tab",
            "",
            "3. PRODUCT RESPONSIBILITY BOUNDARIES",
            "------------------------------------",
            "  • HyperKB (Developer Cockpit):",
            "    Zero-latency governance for devs and co-collaborating agents.",
            "    Manages standing directives, invariant guardrails, agent authority grants, verification",
            "    risk gates (check-work), and empirical telemetry without context switching.",
            "",
            "  • External IDE / Editor ([o]):",
            "    Long-form markdown writing, ADR drafting, and code implementation belong in your favored",
            "    IDE (VS Code, Cursor, Zed, Neovim, etc.). Press [o] on any document to launch it instantly.",
            "",
            "  • HyperControl (Future Enterprise):",
            "    Macro-governance plane for CISO, security teams, and executive organizational oversight.",
            "",
            "4. UNIVERSAL TEXT SELECTION & CLIPBOARD COPYING",
            "-----------------------------------------------",
            "  HyperKB is a standalone, terminal-agnostic tool designed to work effortlessly across",
            "  all terminal emulators (macOS Terminal, iTerm2, Alacritty, Kitty, Windows Terminal, SSH, tmux, etc.).",
            "",
            "  • Method 1 (Native Drag Selection - Default):",
            "    By default, terminal mouse capture is OFF. You can highlight any text anywhere on",
            "    screen with your mouse and copy it (Cmd+C / Ctrl+Shift+C) without requiring Shift or Option keys.",
            "",
            "  • Method 2 (In-TUI Visual Drag & Auto-Copy):",
            "    Press [m] to enable Mouse Mode. Clicking navigates lists/tabs, and dragging your mouse",
            "    over any text displays an instant visual highlight. Releasing the mouse automatically",
            "    copies the selected snippet to your clipboard via native clipboard and universal OSC 52.",
            "",
            "  • Method 3 (Instant 1-Key Yank):",
            "    Press [y] to instantly copy the active document, directive markdown, grant token JSON,",
            "    session scorecard, or help guide directly into your clipboard.",
            "",
            "5. SCIENTIFIC TELEMETRY & CODING EFFECTIVENESS",
            "----------------------------------------------",
            "  Score = clamp(100 - P_loops - P_friction - P_thrash + B_hazard, 5, 100)",
            "  • Baseline: 100% clean baseline execution without defect.",
            "  • Review Loops (P_loops): -15% per review oscillation (penalizes rework thrash).",
            "  • Iteration Friction (P_friction): -15% deduction if initial execution failed tests or compilation.",
            "  • Tool Thrash (P_thrash): Deductions when inspection-to-edit ratio exceeds 8:1.",
            "  • Hazard Prevention (B_hazard): +10% bonus when active directives intercept regressions.",
            "",
            "6. CLI COMMANDS & MCP SERVER",
            "----------------------------",
            "  hyperkb check-work [--diff]       Audit git changes against active risks & directives",
            "  hyperkb audit [--kb/--directives] Verify document bloat, depth, and schema validity",
            "  hyperkb directive list/new        Manage team policies and standing guardrails",
            "  hyperkb session briefing          Inspect agent telemetry and generate briefings",
            "  hyperkb mcp                       Launch stdio MCP server for AI coding agents",
        ];
        lines.join("\n")
    }
}
