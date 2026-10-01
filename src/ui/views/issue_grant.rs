use crate::ui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

pub struct IssueGrantModal;

impl IssueGrantModal {
    pub fn modal_area(area: Rect) -> Rect {
        let width = (area.width * 78 / 100).clamp(60, 92);
        let height = (area.height * 75 / 100).clamp(20, 28);

        let horiz_pad = area.width.saturating_sub(width) / 2;
        let vert_pad = area.height.saturating_sub(height) / 2;

        Rect::new(
            area.x + horiz_pad,
            area.y + vert_pad,
            width,
            height,
        )
    }

    pub const PRESETS: &'static [(&'static str, &'static str, &'static str)] = &[
        ("Frontend & UI Specialist", "src/ui/**, docs/specs/**", "ProposeDecision, AutoRepair (max diff: 250)"),
        ("Documentation & Governance", "docs/**", "ProposeDecision, AcceptDecision (max diff: 400)"),
        ("Full Workspace Autonomy", "*", "All actions & scopes (max diff: 500)"),
        ("Conservative Reviewer", "src/**", "ProposeDecision only (read-only write diff: 200)"),
    ];

    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let modal_area = Self::modal_area(area);

        // 1. Clear background
        frame.render_widget(Clear, modal_area);

        // 2. Outer container
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" [n] Issue Agent Authority Grant (Zero-Trust Delegation) ", t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [Tab] ", t.key_badge()),
                Span::styled("Next • ", Style::default().fg(t.text_muted())),
                Span::styled("[Space] ", t.key_badge()),
                Span::styled("Cycle Preset/TTL • ", Style::default().fg(t.text_muted())),
                Span::styled("[Enter] ", t.key_badge()),
                Span::styled("Issue Grant • ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] ", t.key_badge()),
                Span::styled("Cancel", Style::default().fg(t.text_muted())),
            ]));

        let inner_area = outer_block.inner(modal_area);
        frame.render_widget(outer_block, modal_area);

        // 3. Layout: Subtitle + Form Rows + Preset Specs preview
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Subtitle
                Constraint::Length(3), // Field 0: Grantee Alias
                Constraint::Length(3), // Field 1: Preset Selector
                Constraint::Length(3), // Field 2: TTL Hours
                Constraint::Min(4),    // Granted Capability Breakdown
            ])
            .split(inner_area);

        // Subtitle
        let subtitle = vec![
            Line::from(vec![
                Span::styled(
                    "Issue a cryptographically signed authority grant for an autonomous agent or sub-agent.",
                    Style::default().fg(t.text_primary()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Harness- and LLM-agnostic: Stored as atomic JSON in .hyperkb/grants/<uuid>.json",
                    Style::default().fg(t.text_muted()),
                ),
            ]),
        ];
        frame.render_widget(Paragraph::new(subtitle), chunks[0]);

        // Field 0: Grantee Alias
        let is_f0 = app.new_grant_field == 0;
        let f0_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_f0 {
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.border())
            })
            .title(Span::styled(
                if is_f0 { " Grantee Agent Alias / Model ID (Tab to advance) ▶ " } else { " Grantee Agent Alias / Model ID " },
                if is_f0 { t.title() } else { Style::default().fg(t.text_muted()) },
            ));
        let grantee_content = vec![Line::from(vec![
            Span::styled(
                if app.new_grant_grantee.is_empty() && !is_f0 {
                    "e.g. frontend-subagent, claude-3-7-sonnet, cursor-agent".to_string()
                } else {
                    app.new_grant_grantee.clone()
                },
                if app.new_grant_grantee.is_empty() {
                    Style::default().fg(t.text_muted())
                } else {
                    Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                },
            ),
            if is_f0 {
                Span::styled("▌", Style::default().fg(t.accent()))
            } else {
                Span::raw("")
            },
        ])];
        frame.render_widget(Paragraph::new(grantee_content).block(f0_block), chunks[1]);

        // Field 1: Preset Selector
        let is_f1 = app.new_grant_field == 1;
        let preset_idx = app.new_grant_preset_idx % Self::PRESETS.len();
        let (preset_name, _, _) = Self::PRESETS[preset_idx];
        let f1_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_f1 {
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.border())
            })
            .title(Span::styled(
                if is_f1 { " Security & Scope Preset ([Space] to cycle) ▶ " } else { " Security & Scope Preset " },
                if is_f1 { t.title() } else { Style::default().fg(t.text_muted()) },
            ));
        let preset_content = vec![Line::from(vec![
            Span::styled(format!(" [{}/{}] ", preset_idx + 1, Self::PRESETS.len()), Style::default().fg(t.accent())),
            Span::styled(
                preset_name,
                Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD),
            ),
        ])];
        frame.render_widget(Paragraph::new(preset_content).block(f1_block), chunks[2]);

        // Field 2: TTL Hours
        let is_f2 = app.new_grant_field == 2;
        let f2_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_f2 {
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.border())
            })
            .title(Span::styled(
                if is_f2 { " Grant Lifetime / Expiry TTL ([Space] to cycle) ▶ " } else { " Grant Lifetime / Expiry TTL " },
                if is_f2 { t.title() } else { Style::default().fg(t.text_muted()) },
            ));
        let ttl_text = match app.new_grant_ttl_hours {
            0 => "No Expiry (Permanent session until revoked)".to_string(),
            1 => "1 Hour".to_string(),
            4 => "4 Hours (Recommended for task run)".to_string(),
            8 => "8 Hours (Standard working day)".to_string(),
            24 => "24 Hours (Full day sprint)".to_string(),
            h => format!("{} Hours", h),
        };
        let ttl_content = vec![Line::from(vec![
            Span::styled(" ⏱  ", Style::default().fg(t.accent())),
            Span::styled(
                ttl_text,
                Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD),
            ),
        ])];
        frame.render_widget(Paragraph::new(ttl_content).block(f2_block), chunks[3]);

        // Breakdown Box
        let (_, preset_scopes, preset_actions) = Self::PRESETS[preset_idx];
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border()))
            .title(Span::styled(" Active Preset Configuration Details ", Style::default().fg(t.text_muted())));

        let details = vec![
            Line::from(vec![
                Span::styled("  Allowed Scopes:       ", Style::default().fg(t.text_muted())),
                Span::styled(preset_scopes, Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Delegated Actions:    ", Style::default().fg(t.text_muted())),
                Span::styled(preset_actions, Style::default().fg(t.status_accepted())),
            ]),
            Line::from(vec![
                Span::styled("  Yank on Issue:        ", Style::default().fg(t.text_muted())),
                Span::styled("Auto-copies UUID token to system clipboard for instant agent prompt injection.", Style::default().fg(t.text_primary())),
            ]),
        ];
        frame.render_widget(Paragraph::new(details).block(detail_block), chunks[4]);
    }
}
