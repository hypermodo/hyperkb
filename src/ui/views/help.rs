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
                Span::styled("  [ 1 ] - [ 5 ]      ", t.key_badge()),
                Span::styled("Switch tabs (Work, Explore, Directives, Sessions, Settings)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ Tab ]            ", t.key_badge()),
                Span::styled("Toggle focus between List selection and Detail preview", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ j ] / [ k ]      ", t.key_badge()),
                Span::styled("Navigate records and tree items up / down (or Arrow keys)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ PgUp ] / [ PgDn ]", t.key_badge()),
                Span::styled(" Scroll detailed content preview, reader, and documentation", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ Enter ]          ", t.key_badge()),
                Span::styled("Open document in Reader, drill down into folders, or confirm", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ Esc ]            ", t.key_badge()),
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
                Span::styled("  [ y ]              ", t.key_badge()),
                Span::styled("Yank / Copy active doc, directive, risk, or scorecard to clipboard", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ m ]              ", t.key_badge()),
                Span::styled("Toggle Mouse Mode: ON (Click Navigation) / OFF (Text Selection)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ t ]              ", t.key_badge()),
                Span::styled("Toggle Tree View vs flat List View in Explore tab", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ Space ]          ", t.key_badge()),
                Span::styled("Expand or collapse directory folder in Explore Tree View", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ c ]              ", t.key_badge()),
                Span::styled("Cycle category filters (Decisions, Risks, Specs, Plans)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ / ]              ", t.key_badge()),
                Span::styled("Search knowledge base documents by title, keyword, or path", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ v ]              ", t.key_badge()),
                Span::styled("Toggle formatted Markdown preview vs RAW file view (Reader tab)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ r ]              ", t.key_badge()),
                Span::styled("Retire selected directive (in Directives tab)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ T ]              ", t.key_badge()),
                Span::styled("Cycle visual theme (Cyberpunk, Modern, Nord, Tokyo Night, Light)", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  [ q ] / Ctrl+C     ", t.key_badge()),
                Span::styled("Exit HyperKB safely", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("3. UNIVERSAL TEXT SELECTION & CLIPBOARD COPYING", t.section_header())),
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
            ]),
            Line::from(Span::styled(
                "    Pressing 'y' instantly copies the entire active document, directive, risk, or session",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(Span::styled(
                "    scorecard directly into your system clipboard without requiring any manual dragging.",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "────────────────────────────────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )),
            Line::from(""),
            Line::from(Span::styled("4. SCIENTIFIC TELEMETRY & CODING EFFECTIVENESS", t.section_header())),
            Line::from(""),
            Line::from(Span::styled(
                "  HyperKB rejects arbitrary vanity metrics. Coding Effectiveness is calculated using an",
                Style::default().fg(t.text_primary()),
            )),
            Line::from(Span::styled(
                "  empirically grounded behavioral model:",
                Style::default().fg(t.text_primary()),
            )),
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
            Line::from(Span::styled("5. CLI COMMANDS & MCP SERVER", t.section_header())),
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
                Span::styled("• Close: ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] or [?] ", t.key_badge()),
            ]));

        let p = Paragraph::new(text)
            .block(block)
            .scroll((app.help_scroll as u16, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(p, modal_area);
    }
}
