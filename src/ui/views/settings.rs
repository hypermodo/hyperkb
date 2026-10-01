use crate::ui::app::{App, FocusedPane};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

pub struct SettingsView;

impl SettingsView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let list_width = (area.width * 40 / 100).clamp(38, 65);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Min(40)])
            .split(area);

        Self::render_settings_list(frame, app, chunks[0]);
        Self::render_setting_detail(frame, app, chunks[1]);
    }

    fn render_settings_list(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let border_color = if app.focused_pane == FocusedPane::List {
            t.border_focused()
        } else {
            t.border()
        };

        let active_harness_str = if let Some(h) = app.harnesses.get(app.selected_harness_idx) {
            format!("{} [{}]", h.name, h.governance_status.as_str())
        } else {
            format!("{} detected / registered", app.harnesses.len())
        };

        let settings = [
            ("Directives Ceiling (Rule of N)", format!("{} directives", app.manifest.settings.max_briefing_directives)),
            ("Stale Document Horizon", format!("{} days", app.manifest.settings.stale_days_threshold)),
            ("Max Document Line Ceiling", format!("{} lines", app.manifest.settings.audit_max_lines)),
            ("Max Folder Nesting Depth", format!("{} levels", app.manifest.settings.audit_max_depth)),
            ("Active Visual Theme", app.theme.as_str().to_string()),
            ("Mouse Navigation Mode", if app.mouse_capture { "Enabled (Click Nav)".to_string() } else { "Disabled (Text Selection)".to_string() }),
            ("Policy Taxonomy Categories", format!("{} domains defined", app.manifest.taxonomy.categories.len())),
            ("AI Harness & LLM Registry", active_harness_str),
        ];

        let items: Vec<ListItem> = settings
            .iter()
            .enumerate()
            .map(|(idx, (name, val))| {
                let is_selected = idx == app.settings_selected_idx;
                let title_style = if is_selected {
                    t.selected_row()
                } else {
                    Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("  {}  ", if is_selected { "●" } else { "○" }), if is_selected { Style::default().fg(t.accent()) } else { Style::default().fg(t.text_muted()) }),
                        Span::styled(name.to_string(), title_style),
                    ]),
                    Line::from(vec![
                        Span::styled("      Current: ", Style::default().fg(t.text_muted())),
                        Span::styled(val.clone(), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                ])
            })
            .collect();

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" Configurable Settings & Policy Knobs ", t.title()));

        let list = List::new(items).block(list_block);
        frame.render_widget(list, area);
    }

    fn render_setting_detail(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let border_color = if app.focused_pane == FocusedPane::Detail {
            t.border_focused()
        } else {
            t.border()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" Setting Details & Live Adjustment ", t.title()));

        let (title, val_str, desc, impact) = match app.settings_selected_idx {
            0 => (
                "Directives Ceiling (Rule of N)",
                format!("{} directives", app.manifest.settings.max_briefing_directives),
                "Controls the maximum number of active directives injected into AI agent session briefings and pre-commit checks. Originally set to 5 (Rule of 5) to balance guardrail coverage against prompt context bloat.",
                "Lower values reduce LLM prompt token costs and decrease agent distraction. Higher values permit more specific policy rules to be active across broader codebase paths simultaneously."
            ),
            1 => (
                "Stale Document Horizon",
                format!("{} days", app.manifest.settings.stale_days_threshold),
                "Threshold in days before an unverified knowledge base document (Decision, Risk, or Spec) is flagged during 'hyperkb audit' and briefings as requiring architect re-verification.",
                "Prevents knowledge base drift and phantom architectural guidelines that no longer reflect the production codebase."
            ),
            2 => (
                "Max Document Line Ceiling",
                format!("{} lines", app.manifest.settings.audit_max_lines),
                "The maximum allowed line count for any individual KB markdown document before the anti-bloat linter flags it as oversized.",
                "Enforces concise, modular ADRs and specs. When documents grow beyond this ceiling, authors should split them into focused child decisions."
            ),
            3 => (
                "Max Folder Nesting Depth",
                format!("{} directory levels", app.manifest.settings.audit_max_depth),
                "Maximum directory nesting depth permitted within the knowledge base root before the linter raises a taxonomy hierarchy violation.",
                "Eliminates deep, labyrinthine folder trees in favor of shallow, discoverable topic groupings (e.g. decisions/001.md)."
            ),
            4 => (
                "Active Visual Theme",
                app.theme.as_str().to_string(),
                "User interface color palette and styling for the interactive terminal application.",
                "Themes include Cyberpunk (Terminal Neon), Modern (Electric Slate), Nord (Arctic Frost), Tokyo Night (Storm & Lavender), and Light (Paper & Indigo)."
            ),
            5 => (
                "Mouse Navigation Mode",
                if app.mouse_capture { "Enabled (Click to Navigate)".to_string() } else { "Disabled (Native Text Selection & Copy)".to_string() },
                "Controls terminal mouse event capture. When enabled, mouse clicks switch tabs, select items, and expand tree folders. When disabled, standard click-and-drag terminal text selection and copying are enabled.",
                "Press [m] at any time in any view to quickly toggle this without entering Settings."
            ),
            6 => (
                "Policy Taxonomy Domains (hyperkb.json)",
                format!("{} domains configured", app.manifest.taxonomy.categories.len()),
                "Defines the architectural and operational classification domains for repo invariants and directives. Custom taxonomy categories can be added directly to hyperkb.json or mandated through corporate HyperControl policies.",
                "Directives in active taxonomy domains are enforced at git pre-commit gates and briefed to autonomous agent sessions."
            ),
            7 => (
                "AI Harness & LLM Registry (Discovery & Governance)",
                if let Some(h) = app.harnesses.get(app.selected_harness_idx) {
                    format!("{} ({})", h.name, h.protocol.protocol_label())
                } else {
                    "None".to_string()
                },
                "Autonomous coding harnesses and LLM engines discovered locally in PATH or registered in hyperkb.json. Supports CLI subprocesses, MCP bridges, and HTTP API endpoints without hardcoded vendor lock-in.",
                "Future HyperControl CISO policies can mandate whitelists, block unauthorized shadow AI, and enforce signed tokens."
            ),
            _ => ("Setting", "".to_string(), "", ""),
        };

        let mut text = vec![
            Line::from(vec![
                Span::styled("Setting: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                Span::styled(title, Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Current Value: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                Span::styled(val_str, Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled("────── Adjustment Controls ──────────────────────────────────", Style::default().fg(t.border()))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Use ", Style::default().fg(t.text_primary())),
                Span::styled("[←] / [→]", t.key_badge()),
                Span::styled(" or ", Style::default().fg(t.text_primary())),
                Span::styled("[h] / [l]", t.key_badge()),
                Span::styled(" or ", Style::default().fg(t.text_primary())),
                Span::styled("[-] / [+]", t.key_badge()),
                Span::styled(" to adjust value live.", Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("  Press ", Style::default().fg(t.text_primary())),
                Span::styled("[Enter]", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled(" to persist changes to ", Style::default().fg(t.text_primary())),
                Span::styled("hyperkb.json", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                Span::styled(".", Style::default().fg(t.text_primary())),
            ]),
            Line::from(""),
            Line::from(Span::styled("────── Description & Rationale ──────────────────────────────", Style::default().fg(t.border()))),
            Line::from(""),
            Line::from(Span::styled(desc, Style::default().fg(t.text_primary()))),
            Line::from(""),
        ];

        // Specific rich item listings for Taxonomy and Harnesses
        if app.settings_selected_idx == 6 {
            text.push(Line::from(Span::styled("────── Configured Taxonomy Domains ──────────────────────────", Style::default().fg(t.border()))));
            text.push(Line::from(""));
            for cat in &app.manifest.taxonomy.categories {
                text.push(Line::from(vec![
                    Span::styled("  • ", Style::default().fg(t.accent())),
                    Span::styled(format!("{}: ", cat.id), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled(cat.label.clone(), Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                ]));
                text.push(Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(cat.description.clone(), Style::default().fg(t.text_muted())),
                ]));
                text.push(Line::from(""));
            }
        } else if app.settings_selected_idx == 7 {
            text.push(Line::from(Span::styled("────── Local AI Harness Registry & Discovery ────────────────", Style::default().fg(t.border()))));
            text.push(Line::from(""));
            for (idx, h) in app.harnesses.iter().enumerate() {
                let is_sel = idx == app.selected_harness_idx;
                let status_style = match h.governance_status {
                    crate::domain::HarnessGovernanceStatus::Allowed => t.badge_accepted(),
                    crate::domain::HarnessGovernanceStatus::Discovered => t.badge_proposed(),
                    crate::domain::HarnessGovernanceStatus::Blocked => t.badge_risk(),
                    crate::domain::HarnessGovernanceStatus::Enforced => t.badge_accepted(),
                };

                let marker = if is_sel { "▶ " } else { "  " };
                text.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(t.accent())),
                    Span::styled(format!("[{}] ", h.governance_status.as_str()), status_style),
                    Span::styled(format!("{} ", h.name), if is_sel { t.selected_row() } else { Style::default().fg(t.text_primary()) }),
                    Span::styled(format!("({})", h.protocol.protocol_label()), Style::default().fg(t.text_muted())),
                ]));

                if let Some(ref bp) = h.binary_path {
                    text.push(Line::from(vec![
                        Span::styled("    Binary: ", Style::default().fg(t.text_muted())),
                        Span::styled(bp.clone(), Style::default().fg(t.accent())),
                    ]));
                }
                if !h.detected_models.is_empty() {
                    text.push(Line::from(vec![
                        Span::styled("    Models: ", Style::default().fg(t.text_muted())),
                        Span::styled(h.detected_models.join(", "), Style::default().fg(t.text_primary())),
                    ]));
                }
                if let Some(ref reason) = h.governance_reason {
                    text.push(Line::from(vec![
                        Span::styled("    Policy: ", Style::default().fg(t.text_muted())),
                        Span::styled(reason.clone(), Style::default().fg(t.text_muted())),
                    ]));
                }
                text.push(Line::from(""));
            }
        }

        text.push(Line::from(Span::styled("────── Engineering Impact ───────────────────────────────────", Style::default().fg(t.border()))));
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(impact, Style::default().fg(t.text_muted()))));
        text.push(Line::from(""));
        text.push(Line::from(Span::styled("────────────────────────────────────────────────────────────", Style::default().fg(t.border()))));
        text.push(Line::from(vec![
            Span::styled(
                if app.settings_dirty {
                    "● Unsaved changes: press [Enter] to write to hyperkb.json"
                } else {
                    "✔ All settings synced with hyperkb.json"
                },
                if app.settings_dirty {
                    Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.status_accepted())
                },
            ),
        ]));

        let p = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        frame.render_widget(p, area);
    }
}
