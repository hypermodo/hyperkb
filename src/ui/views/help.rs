use crate::ui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

pub struct HelpModal;

impl HelpModal {
    pub fn render(frame: &mut Frame, scroll_offset: usize, area: Rect) {
        // Center modal: 80% width (min 60, max 100), 85% height (min 20, max 40)
        let modal_width = (area.width * 82 / 100).clamp(60, 110);
        let modal_height = (area.height * 86 / 100).clamp(18, 42);

        let horiz_pad = (area.width.saturating_sub(modal_width)) / 2;
        let vert_pad = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(
            area.x + horiz_pad,
            area.y + vert_pad,
            modal_width,
            modal_height,
        );

        // 1. Clear background behind modal
        frame.render_widget(Clear, modal_area);

        // 2. Render Modal Block with generous padding
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(
                " [?] HyperKB System Documentation & Shortcuts [Esc or '?' to Close] ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));

        let text = vec![
            Line::from(vec![
                Span::styled("HyperKB", Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)),
                Span::raw(" — Autonomous Knowledge Base, Agent Governance & Architecture Telemetry"),
            ]),
            Line::from(""),
            Line::from(Span::styled("────────────────────────────────────────────────────────────────────────────", Style::default().fg(Theme::BORDER))),
            Line::from(""),
            Line::from(Span::styled("1. KEYBOARD NAVIGATION & SHORTCUTS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [1] - [5]      ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Switch tabs (Work, Explore, Directives, Sessions, Settings)"),
            ]),
            Line::from(vec![
                Span::styled("  [Tab]          ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Toggle focus between list selection and content detail view"),
            ]),
            Line::from(vec![
                Span::styled("  [j] / [k], ↑/↓ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Navigate records and items up and down"),
            ]),
            Line::from(vec![
                Span::styled("  [PgUp] / [PgDn]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Scroll detailed content preview and reader"),
            ]),
            Line::from(vec![
                Span::styled("  [Enter]        ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Open document in reader, drill down into folders, or confirm"),
            ]),
            Line::from(vec![
                Span::styled("  [Space]        ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Expand or collapse directory folder in Explore Tree View"),
            ]),
            Line::from(vec![
                Span::styled("  [t]            ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Toggle hierarchical Tree View vs flat List View in Explore tab"),
            ]),
            Line::from(vec![
                Span::styled("  [c]            ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Cycle category filters (Decisions, Risks, Specs, Plans / Taxonomies)"),
            ]),
            Line::from(vec![
                Span::styled("  [/]            ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Search knowledge base documents by title, keyword, or path"),
            ]),
            Line::from(vec![
                Span::styled("  [r]            ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Retire the selected directive (in Directives tab)"),
            ]),
            Line::from(vec![
                Span::styled("  [v]            ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Toggle formatted markdown preview vs RAW file view (in Reader tab)"),
            ]),
            Line::from(vec![
                Span::styled("  [T]            ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Cycle visual themes (Cyberpunk, Modern, Nord, Tokyo Night, Light)"),
            ]),
            Line::from(vec![
                Span::styled("  [m]            ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Toggle Mouse Mode: ON (Click Nav) / OFF (Text Select & Copy)"),
            ]),
            Line::from(vec![
                Span::styled("  [?] or [h]     ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Open or close this interactive documentation overlay"),
            ]),
            Line::from(vec![
                Span::styled("  [q] / Ctrl+C   ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Exit HyperKB safely"),
            ]),
            Line::from(""),
            Line::from(Span::styled("────────────────────────────────────────────────────────────────────────────", Style::default().fg(Theme::BORDER))),
            Line::from(""),
            Line::from(Span::styled("2. TUI TEXT SELECTION & COPY / PASTE", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from("  Terminal applications capture mouse events for clicking and scrolling,"),
            Line::from("  which can intercept standard terminal text drag-selection. HyperKB provides"),
            Line::from("  two zero-friction solutions to select and copy text:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Option A (Quick Toggle): ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("Press "),
                Span::styled("[m]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" to disable mouse capture. You can now immediately drag"),
            ]),
            Line::from("  to select any text on screen and copy with Cmd+C / Ctrl+Shift+C. Press [m] again to re-enable."),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Option B (Terminal Bypass): ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("Hold the "),
                Span::styled("Option (⌥)", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" key on macOS (or "),
                Span::styled("Shift", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" on Linux) while"),
            ]),
            Line::from("  clicking and dragging with your mouse. Your terminal will natively select the text"),
            Line::from("  without needing to toggle mouse mode off."),
            Line::from(""),
            Line::from(Span::styled("────────────────────────────────────────────────────────────────────────────", Style::default().fg(Theme::BORDER))),
            Line::from(""),
            Line::from(Span::styled("3. SCIENTIFIC TELEMETRY & CODING EFFECTIVENESS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from("  HyperKB rejects subjective vanity scores. Coding Effectiveness is grounded in a"),
            Line::from("  mathematically formal behavioral model:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Formula: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("Score = clamp(100 - P_loops - P_friction - P_thrash + B_hazard, 5, 100)", Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from("  • Base Score: 100% clean baseline execution."),
            Line::from("  • P_loops (Oscillations): -15% penalty per review loop (penalizes rework thrash)."),
            Line::from("  • P_friction (Iteration): -15% deduction if initial execution failed tests/compilation."),
            Line::from("  • P_thrash (Tool-to-Edit): Deductions applied when inspections exceed 8:1 ratio."),
            Line::from("  • B_hazard (Risk Prevention): +10% bonus for active invariant interception."),
            Line::from(""),
            Line::from(Span::styled("────────────────────────────────────────────────────────────────────────────", Style::default().fg(Theme::BORDER))),
            Line::from(""),
            Line::from(Span::styled("4. CLI COMMANDS & MCP SERVER", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  hyperkb check-work [--diff]  ", Style::default().fg(Theme::ACCENT)),
                Span::raw("Audit current git changes against risks & directives"),
            ]),
            Line::from(vec![
                Span::styled("  hyperkb audit [--kb/--directives] ", Style::default().fg(Theme::ACCENT)),
                Span::raw("Verify line bloat, depth, and schema validity"),
            ]),
            Line::from(vec![
                Span::styled("  hyperkb directive list/new   ", Style::default().fg(Theme::ACCENT)),
                Span::raw("Manage team policies and standing guardrails"),
            ]),
            Line::from(vec![
                Span::styled("  hyperkb session briefing/list", Style::default().fg(Theme::ACCENT)),
                Span::raw("Inspect agent telemetry and generate briefings"),
            ]),
            Line::from(vec![
                Span::styled("  hyperkb mcp                  ", Style::default().fg(Theme::ACCENT)),
                Span::raw("Run zero-config MCP server for AI coding agents"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Press [Esc] or [?] to close this help window.", Style::default().fg(Theme::TEXT_MUTED))),
        ];

        let p = Paragraph::new(text)
            .block(block)
            .scroll((scroll_offset as u16, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(p, modal_area);
    }
}
