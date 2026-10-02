use crate::ui::app::{App, FocusedPane};
use crate::ui::theme::ThemeMode;
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
            ("Mouse Navigation Mode", if app.mouse_capture { "Enabled (Click Nav)".to_string() } else { "Disabled (Text Copy)".to_string() }),
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

                let cursor_str = if is_selected { "▶ " } else { "  " };
                let marker_str = if is_selected { "● " } else { "○ " };
                let marker_style = if is_selected {
                    Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.text_muted())
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(cursor_str, Style::default().fg(t.accent())),
                        Span::styled(marker_str, marker_style),
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
            .title(Span::styled(" Configurable Settings & Policy Knobs ", t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [↑/↓] ", t.key_badge()),
                Span::styled("Select • ", Style::default().fg(t.text_muted())),
                Span::styled("[Tab/→] ", t.key_badge()),
                Span::styled("Details • ", Style::default().fg(t.text_muted())),
                Span::styled("[Enter/Space] ", t.key_badge()),
                Span::styled("Action", Style::default().fg(t.text_muted())),
            ]));

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

        let (title, desc, impact) = match app.settings_selected_idx {
            0 => (
                "Directives Ceiling (Rule of N)",
                "Controls the maximum number of active directives injected into AI agent session briefings and pre-commit checks. Originally set to 5 (Rule of 5) to balance guardrail coverage against prompt context bloat.",
                "Lower values reduce LLM prompt token costs and decrease agent distraction. Higher values permit more specific policy rules to be active across broader codebase paths simultaneously."
            ),
            1 => (
                "Stale Document Horizon",
                "Threshold in days before an unverified knowledge base document (Decision, Risk, or Spec) is flagged during 'hyperkb audit' and briefings as requiring architect re-verification.",
                "Prevents knowledge base drift and phantom architectural guidelines that no longer reflect the production codebase."
            ),
            2 => (
                "Max Document Line Ceiling",
                "The maximum allowed line count for any individual KB markdown document before the anti-bloat linter flags it as oversized.",
                "Enforces concise, modular ADRs and specs. When documents grow beyond this ceiling, authors should split them into focused child decisions."
            ),
            3 => (
                "Max Folder Nesting Depth",
                "Maximum directory nesting depth permitted within the knowledge base root before the linter raises a taxonomy hierarchy violation.",
                "Eliminates deep, labyrinthine folder trees in favor of shallow, discoverable topic groupings (e.g. decisions/001.md)."
            ),
            4 => (
                "Active Visual Theme",
                "User interface color palette and styling for the interactive terminal application.",
                "Themes include Modern (Electric Slate), Nord (Arctic Frost), Tokyo Night (Storm & Lavender), Cyberpunk (Terminal Neon), and Light (Paper & Indigo)."
            ),
            5 => (
                "Mouse Navigation Mode",
                "Controls terminal mouse event capture. When enabled, mouse clicks switch tabs, select items, and expand tree folders. When disabled, standard click-and-drag terminal text selection and copying are enabled.",
                "Toggle at any time in any view using the [m] key without opening Settings."
            ),
            6 => (
                "Policy Taxonomy Domains (hyperkb.json)",
                "Defines the architectural and operational classification domains for repo invariants and directives. Custom taxonomy categories can be added directly to hyperkb.json or mandated through corporate governance.",
                "Directives in active taxonomy domains are enforced at git pre-commit gates and briefed to autonomous agent sessions."
            ),
            7 => (
                "AI Harness & LLM Registry (Discovery & Governance)",
                "Autonomous coding harnesses and LLM engines discovered locally in PATH or registered in hyperkb.json. Supports CLI subprocesses, MCP bridges, and HTTP API endpoints without vendor lock-in.",
                "Harness governance policies can whitelist approved agents, mandate sandboxing, and enforce accountability tracking."
            ),
            _ => ("Setting", "", ""),
        };

        let mut text: Vec<Line> = Vec::new();

        // 1. Setting Title
        text.push(Line::from(vec![
            Span::styled("Setting: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
            Span::styled(title, Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
        ]));
        text.push(Line::from(""));

        // 2. HERO CONTROL CARD (Prominent & Tangible)
        match app.settings_selected_idx {
            0..=3 => {
                let (val, min, max, unit, step) = match app.settings_selected_idx {
                    0 => (app.manifest.settings.max_briefing_directives as i64, 1i64, 20i64, "directives", 1i64),
                    1 => (app.manifest.settings.stale_days_threshold, 7i64, 365i64, "days", 15i64),
                    2 => (app.manifest.settings.audit_max_lines as i64, 50i64, 1000i64, "lines", 25i64),
                    _ => (app.manifest.settings.audit_max_depth as i64, 1i64, 8i64, "levels", 1i64),
                };

                let ratio = if max > min {
                    ((val - min) as f64 / (max - min) as f64).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                let track_len = 36usize;
                let fill_len = ((track_len as f64) * ratio).round() as usize;
                let filled_bar = "═".repeat(fill_len);
                let empty_bar = "─".repeat(track_len.saturating_sub(fill_len));
                let pct = (ratio * 100.0).round() as usize;

                text.push(Line::from(Span::styled("┌── Value Control ───────────────────────────────────────────┐", Style::default().fg(t.accent()))));
                text.push(Line::from(vec![
                    Span::styled("│  ", Style::default().fg(t.accent())),
                    Span::styled(" [ - ] ", t.key_badge()),
                    Span::styled(format!("   {:>4} {}   ", val, unit), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled(" [ + ] ", t.key_badge()),
                    Span::styled(format!("    (step: ±{})", step), Style::default().fg(t.text_muted())),
                ]));
                text.push(Line::from(vec![
                    Span::styled("│  ", Style::default().fg(t.accent())),
                    Span::styled("[", Style::default().fg(t.text_muted())),
                    Span::styled(filled_bar, Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled("●", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled(empty_bar, Style::default().fg(t.border())),
                    Span::styled(format!("]  {:>3}%", pct), Style::default().fg(t.accent())),
                ]));
                text.push(Line::from(vec![
                    Span::styled("│  ", Style::default().fg(t.accent())),
                    Span::styled(format!("Min: {} {}", min, unit), Style::default().fg(t.text_muted())),
                    Span::styled("                             ", Style::default()),
                    Span::styled(format!("Max: {} {}", max, unit), Style::default().fg(t.text_muted())),
                ]));
                text.push(Line::from(Span::styled("└────────────────────────────────────────────────────────────┘", Style::default().fg(t.accent()))));
                text.push(Line::from(vec![
                    Span::styled("  Use ", Style::default().fg(t.text_muted())),
                    Span::styled("[-]", t.key_badge()),
                    Span::styled(" / ", Style::default().fg(t.text_muted())),
                    Span::styled("[+]", t.key_badge()),
                    Span::styled(" to nudge value live • ", Style::default().fg(t.text_muted())),
                    Span::styled("[Enter]", t.key_badge()),
                    Span::styled(" to save", Style::default().fg(t.text_muted())),
                ]));
            }
            4 => {
                text.push(Line::from(Span::styled("┌── Theme Selection ─────────────────────────────────────────┐", Style::default().fg(t.accent()))));
                for mode in ThemeMode::all() {
                    let is_active = *mode == app.theme;
                    let radio = if is_active { "● " } else { "○ " };
                    let name_style = if is_active {
                        t.selected_row()
                    } else {
                        Style::default().fg(t.text_primary())
                    };
                    let active_badge = if is_active {
                        Span::styled("   ◄ ACTIVE", t.badge_accepted())
                    } else {
                        Span::raw("")
                    };

                    text.push(Line::from(vec![
                        Span::styled("│  ", Style::default().fg(t.accent())),
                        Span::styled(radio, if is_active { Style::default().fg(t.accent()) } else { Style::default().fg(t.text_muted()) }),
                        Span::styled(mode.as_str(), name_style),
                        active_badge,
                    ]));
                }
                text.push(Line::from(Span::styled("└────────────────────────────────────────────────────────────┘", Style::default().fg(t.accent()))));
                text.push(Line::from(vec![
                    Span::styled("  Use ", Style::default().fg(t.text_muted())),
                    Span::styled("[- / +]", t.key_badge()),
                    Span::styled(" or ", Style::default().fg(t.text_muted())),
                    Span::styled("[Enter / Space]", t.key_badge()),
                    Span::styled(" to cycle themes live", Style::default().fg(t.text_muted())),
                ]));
            }
            5 => {
                text.push(Line::from(Span::styled("┌── Mouse Navigation Switch ─────────────────────────────────┐", Style::default().fg(t.accent()))));
                if app.mouse_capture {
                    text.push(Line::from(vec![
                        Span::styled("│  ", Style::default().fg(t.accent())),
                        Span::styled(" [ ● ENABLED (Click to Navigate) ] ", t.badge_accepted()),
                        Span::styled("    ○ Disabled (Text Copy) ", Style::default().fg(t.text_muted())),
                    ]));
                } else {
                    text.push(Line::from(vec![
                        Span::styled("│  ", Style::default().fg(t.accent())),
                        Span::styled("   ○ Enabled (Click to Navigate) ", Style::default().fg(t.text_muted())),
                        Span::styled("    [ ● DISABLED (Native Text Copy) ] ", t.badge_proposed()),
                    ]));
                }
                text.push(Line::from(Span::styled("└────────────────────────────────────────────────────────────┘", Style::default().fg(t.accent()))));
                text.push(Line::from(vec![
                    Span::styled("  Press ", Style::default().fg(t.text_muted())),
                    Span::styled("[Space]", t.key_badge()),
                    Span::styled(" or ", Style::default().fg(t.text_muted())),
                    Span::styled("[Enter]", t.key_badge()),
                    Span::styled(" or ", Style::default().fg(t.text_muted())),
                    Span::styled("[m]", t.key_badge()),
                    Span::styled(" to toggle state", Style::default().fg(t.text_muted())),
                ]));
            }
            6 => {
                text.push(Line::from(Span::styled("┌── Configured Taxonomy Domains ─────────────────────────────┐", Style::default().fg(t.accent()))));
                for cat in &app.manifest.taxonomy.categories {
                    text.push(Line::from(vec![
                        Span::styled("│  • ", Style::default().fg(t.accent())),
                        Span::styled(format!("{}: ", cat.id), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                        Span::styled(cat.label.clone(), Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    ]));
                    text.push(Line::from(vec![
                        Span::styled("│    ", Style::default().fg(t.accent())),
                        Span::styled(cat.description.clone(), Style::default().fg(t.text_muted())),
                    ]));
                }
                text.push(Line::from(Span::styled("└────────────────────────────────────────────────────────────┘", Style::default().fg(t.accent()))));
                text.push(Line::from(vec![
                    Span::styled("  Add domain via CLI: ", Style::default().fg(t.text_muted())),
                    Span::styled("hyperkb taxonomy add <id> <name> <desc>", Style::default().fg(t.accent())),
                ]));
            }
            7 => {
                text.push(Line::from(Span::styled("┌── AI Harness Registry & Discovery ─────────────────────────┐", Style::default().fg(t.accent()))));
                if app.harnesses.is_empty() {
                    text.push(Line::from(vec![
                        Span::styled("│  ", Style::default().fg(t.accent())),
                        Span::styled("No external AI coding harnesses detected in PATH.", Style::default().fg(t.text_muted())),
                    ]));
                } else {
                    for (idx, h) in app.harnesses.iter().enumerate() {
                        let is_sel = idx == app.selected_harness_idx;
                        let status_style = match h.governance_status {
                            crate::domain::HarnessGovernanceStatus::Allowed => t.badge_accepted(),
                            crate::domain::HarnessGovernanceStatus::Discovered => t.badge_proposed(),
                            crate::domain::HarnessGovernanceStatus::Blocked => t.badge_risk(),
                            crate::domain::HarnessGovernanceStatus::Enforced => t.badge_accepted(),
                        };
                        let pointer = if is_sel { "▶ " } else { "  " };

                        text.push(Line::from(vec![
                            Span::styled("│ ", Style::default().fg(t.accent())),
                            Span::styled(pointer, Style::default().fg(t.accent())),
                            Span::styled(format!("[{}] ", h.governance_status.as_str()), status_style),
                            Span::styled(h.name.clone(), if is_sel { t.selected_row() } else { Style::default().fg(t.text_primary()) }),
                            Span::styled(format!(" ({})", h.protocol.protocol_label()), Style::default().fg(t.text_muted())),
                        ]));

                        if let Some(ref bp) = h.binary_path {
                            text.push(Line::from(vec![
                                Span::styled("│     Binary: ", Style::default().fg(t.accent())),
                                Span::styled(bp.clone(), Style::default().fg(t.text_muted())),
                            ]));
                        }
                    }
                }
                text.push(Line::from(Span::styled("└────────────────────────────────────────────────────────────┘", Style::default().fg(t.accent()))));
                text.push(Line::from(vec![
                    Span::styled("  Use ", Style::default().fg(t.text_muted())),
                    Span::styled("[↑ / ↓]", t.key_badge()),
                    Span::styled(" in Details to browse harnesses • ", Style::default().fg(t.text_muted())),
                    Span::styled("[Enter]", t.key_badge()),
                    Span::styled(" to save active", Style::default().fg(t.text_muted())),
                ]));
            }
            _ => {}
        }

        // 3. Compact Purpose & Why It Matters
        text.push(Line::from(""));
        text.push(Line::from(Span::styled("PURPOSE", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))));
        text.push(Line::from(Span::styled(desc, Style::default().fg(t.text_primary()))));
        text.push(Line::from(""));
        text.push(Line::from(Span::styled("WHY IT MATTERS", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))));
        text.push(Line::from(Span::styled(impact, Style::default().fg(t.text_muted()))));
        text.push(Line::from(""));

        // 4. Clean Sync Status
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
