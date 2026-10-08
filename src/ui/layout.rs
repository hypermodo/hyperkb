use crate::ui::app::{ActiveTab, App};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub struct Header;

impl Header {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let mut tabs = vec![
            (ActiveTab::Work, "[1] Work"),
            (ActiveTab::Explore, "[2] Knowledge"),
            (ActiveTab::Directives, "[3] Directives"),
            (ActiveTab::Sessions, "[4] Agents"),
            (ActiveTab::Settings, "[5] Settings"),
        ];

        if app.active_tab == ActiveTab::Reader {
            tabs.push((ActiveTab::Reader, "[Reader]"));
        }

        // Line 0: Main Navigation & Branding
        let mut tab_spans = vec![
            Span::styled(" HyperKB", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {} ", app.collection_id), Style::default().fg(t.text_muted())),
            Span::raw("    "),
        ];

        let tabs = [
            (ActiveTab::Work, "1", "Work"),
            (ActiveTab::Explore, "2", "Knowledge"),
            (ActiveTab::Directives, "3", "Directives"),
            (ActiveTab::Sessions, "4", "Agents"),
            (ActiveTab::Settings, "5", "Settings"),
        ];

        for (tab, num, name) in tabs {
            if app.active_tab == tab {
                tab_spans.push(Span::styled(
                    format!(" ● {} {} ", num, name),
                    Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
                ));
            } else {
                tab_spans.push(Span::styled(
                    format!("   {} {} ", num, name),
                    Style::default().fg(t.text_muted()),
                ));
            }
            tab_spans.push(Span::raw(" "));
        }

        if app.active_tab == ActiveTab::Reader {
            tab_spans.push(Span::styled(
                " ● Reader ",
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
            ));
        }

        // Line 1: Context Sub-Header (Quiet, clean text tabs)
        let sub_spans = match app.active_tab {
            ActiveTab::Work => {
                let proj_span = if app.work_tab_mode == crate::ui::app::WorkTabMode::Projects {
                    Span::styled("● Projects [p]", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("Projects [p]", Style::default().fg(t.text_muted()))
                };
                let risks_span = if app.work_tab_mode == crate::ui::app::WorkTabMode::Risks {
                    Span::styled("● Risks [w]", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("Risks [w]", Style::default().fg(t.text_muted()))
                };
                let logs_span = if app.work_tab_mode == crate::ui::app::WorkTabMode::Console {
                    Span::styled("● Logs [c]", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("Logs [c]", Style::default().fg(t.text_muted()))
                };
                let scope_label = if app.work_show_handoffs { "ALL [H]" } else { "CONTROL [H]" };
                vec![
                    Span::styled("  View:  ", Style::default().fg(t.text_muted())),
                    proj_span,
                    Span::styled("   •   ", Style::default().fg(t.border())),
                    risks_span,
                    Span::styled("   •   ", Style::default().fg(t.border())),
                    logs_span,
                    Span::styled("   |   Scope: ", Style::default().fg(t.text_muted())),
                    Span::styled(scope_label, Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("   |   {} projects", app.projects.len()), Style::default().fg(t.text_muted())),
                ]
            }
            ActiveTab::Explore => {
                let doc_label = if app.explore_tree_mode { "● Curated Tree [m]" } else { "● Curated Docs [m]" };
                let docs_span = if app.knowledge_view_mode == crate::ui::app::KnowledgeViewMode::Documents {
                    Span::styled(doc_label, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("Curated Docs [m]", Style::default().fg(t.text_muted()))
                };
                let living_label = format!("Living Knowledge ({}) [m]", app.knowledge_items.len());
                let living_span = if app.knowledge_view_mode == crate::ui::app::KnowledgeViewMode::Living {
                    Span::styled(format!("● {}", living_label), Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled(living_label, Style::default().fg(t.text_muted()))
                };
                let kind_label = match app.knowledge_kind_filter {
                    None => "ALL",
                    Some(crate::domain::KnowledgeKind::Warning) => "WARNINGS",
                    Some(crate::domain::KnowledgeKind::Pattern) => "PATTERNS",
                    Some(crate::domain::KnowledgeKind::Decision) => "DECISIONS",
                    Some(crate::domain::KnowledgeKind::Note) => "NOTES",
                    Some(crate::domain::KnowledgeKind::Preference) => "PREFERENCES",
                };
                vec![
                    Span::styled("  View:  ", Style::default().fg(t.text_muted())),
                    docs_span,
                    Span::styled("   •   ", Style::default().fg(t.border())),
                    living_span,
                    Span::styled("   |   Kind [f]: ", Style::default().fg(t.text_muted())),
                    Span::styled(kind_label, Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                ]
            }
            ActiveTab::Directives => {
                let active_dir_count = app.directives.iter().filter(|d| d.status == "active").count();
                vec![
                    Span::styled("  Taxonomy Category [c]: ", Style::default().fg(t.text_muted())),
                    Span::styled(app.directive_category.to_uppercase(), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("   |   Active: {} / {}", active_dir_count, app.directives.len()), Style::default().fg(t.text_muted())),
                ]
            }
            ActiveTab::Sessions => {
                match app.governance_tab_mode {
                    crate::ui::app::GovernanceTabMode::Sessions => {
                        let filter_label = app.session_harness_filter.as_deref().unwrap_or("ALL");
                        vec![
                            Span::styled("  View:  ", Style::default().fg(t.text_muted())),
                            Span::styled("● Runs [g]", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                            Span::styled("   •   ", Style::default().fg(t.border())),
                            Span::styled("Grants [g]", Style::default().fg(t.text_muted())),
                            Span::styled("   |   Harness [h]: ", Style::default().fg(t.text_muted())),
                            Span::styled(filter_label.to_uppercase(), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("   |   {} sessions", app.sessions.len()), Style::default().fg(t.text_muted())),
                        ]
                    }
                    crate::ui::app::GovernanceTabMode::Grants => {
                        let active_count = app.grants.iter().filter(|g| {
                            g.constraints.expires_at.map(|exp| exp > chrono::Utc::now()).unwrap_or(true)
                        }).count();
                        vec![
                            Span::styled("  View:  ", Style::default().fg(t.text_muted())),
                            Span::styled("Runs [g]", Style::default().fg(t.text_muted())),
                            Span::styled("   •   ", Style::default().fg(t.border())),
                            Span::styled("● Grants [g]", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("   |   {} active grants", active_count), Style::default().fg(t.text_muted())),
                        ]
                    }
                }
            }
            ActiveTab::Settings => {
                vec![
                    Span::styled("  Engine Configuration   |   ", Style::default().fg(t.text_muted())),
                    Span::styled(format!("{} harness(es) registered", app.harnesses.len()), Style::default().fg(t.text_primary())),
                ]
            }
            ActiveTab::Reader => {
                vec![
                    Span::styled("  Document Reader   |   ", Style::default().fg(t.text_muted())),
                    Span::styled("[Esc] Back to Knowledge", Style::default().fg(t.accent())),
                ]
            }
        };

        let paragraph = Paragraph::new(vec![
            Line::from(tab_spans),
            Line::from(sub_spans),
            Line::from(""),
        ]);

        frame.render_widget(paragraph, area);
    }

    pub fn handle_click(app: &mut App, db: &crate::storage::Database, col: u16, row: u16) -> bool {
        if row == 0 {
            let prefix_len = (1 + 7 + 2 + app.collection_id.len() + 1 + 4) as u16;
            let tabs = [
                (ActiveTab::Work, 10u16),
                (ActiveTab::Explore, 15u16),
                (ActiveTab::Directives, 16u16),
                (ActiveTab::Sessions, 12u16),
                (ActiveTab::Settings, 14u16),
            ];
            let mut cur_x = prefix_len;
            for (tab, tab_w) in tabs {
                if col >= cur_x && col < cur_x + tab_w {
                    app.switch_tab(tab);
                    return true;
                }
                cur_x += tab_w + 1;
            }
            if app.active_tab == ActiveTab::Reader {
                let tab_w = 12u16;
                if col >= cur_x && col < cur_x + tab_w {
                    app.switch_tab(ActiveTab::Reader);
                    return true;
                }
            }
        } else if row == 1 {
            match app.active_tab {
                ActiveTab::Work => {
                    if col >= 8 && col <= 24 {
                        app.work_tab_mode = crate::ui::app::WorkTabMode::Projects;
                        return true;
                    } else if col >= 25 && col <= 38 {
                        app.work_tab_mode = crate::ui::app::WorkTabMode::Risks;
                        return true;
                    } else if col >= 39 && col <= 54 {
                        app.work_tab_mode = crate::ui::app::WorkTabMode::Console;
                        return true;
                    } else if col >= 55 && col <= 80 {
                        app.toggle_work_handoffs(db);
                        return true;
                    }
                }
                ActiveTab::Directives => {
                    if col >= 20 && col <= 23 {
                        app.next_directive_category(db);
                        return true;
                    }
                    let mut cur_x = 24u16;
                    for cat in app.directive_categories() {
                        let pill_w = (cat.len() + 4) as u16;
                        if col >= cur_x && col < cur_x + pill_w {
                            app.set_directive_category(&cat, db);
                            return true;
                        }
                        cur_x += pill_w + 1;
                    }
                }
                ActiveTab::Explore => {
                    if col >= 8 && col <= 50 {
                        app.toggle_knowledge_view_mode(db);
                        return true;
                    }
                    if col >= 51 && col <= 90 {
                        app.cycle_knowledge_kind_filter(db);
                        return true;
                    }
                }
                ActiveTab::Sessions => {
                    if col >= 8 && col <= 35 {
                        app.toggle_governance_tab_mode();
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
}

pub struct Footer;

impl Footer {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;

        let text = if let Some(ref msg) = app.status_message {
            Line::from(vec![
                Span::styled(" ● ", t.badge_accepted()),
                Span::styled(msg, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                Span::styled("  (Press any key to dismiss)", Style::default().fg(t.text_muted())),
            ])
        } else if app.is_filtering {
            Line::from(vec![
                Span::styled(" Search: ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                Span::raw(&app.filter_query),
                Span::styled("█", Style::default().fg(t.status_proposed())),
                Span::styled("  (Press Enter to confirm, Esc to cancel)", Style::default().fg(t.text_muted())),
            ])
        } else {
            let keys = if app.repl_active {
                vec![
                    Span::styled("[Enter] ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                    Span::raw("Send   "),
                    Span::styled("[Shift+Enter] ", Style::default().fg(t.accent())),
                    Span::raw("Newline   "),
                    Span::styled("[Tab] ", Style::default().fg(t.accent())),
                    Span::raw("Complete   "),
                    Span::styled("[Esc] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::raw("Unfocus   "),
                    Span::styled("[?] ", Style::default().fg(t.accent())),
                    Span::raw("Help"),
                ]
            } else {
                match app.active_tab {
                    ActiveTab::Work => match app.work_tab_mode {
                        crate::ui::app::WorkTabMode::Projects => vec![
                            Span::styled("[/] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Dock   "),
                            Span::styled("[n] ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                            Span::raw("New Task   "),
                            Span::styled("[t] ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                            Span::raw("Transition   "),
                            Span::styled("[p] ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                            Span::raw("Priority   "),
                            Span::styled("[f] ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                            Span::raw("Filter   "),
                            Span::styled("[x] ", Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)),
                            Span::raw("Delete   "),
                            Span::styled("[v] ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                            Span::raw("Verify Exit   "),
                            Span::styled("[?] ", Style::default().fg(t.accent())),
                            Span::raw("Help"),
                        ],
                        crate::ui::app::WorkTabMode::Risks => vec![
                            Span::styled("[/] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Command Dock   "),
                            Span::styled("[p] ", Style::default().fg(t.accent())),
                            Span::raw("Projects   "),
                            Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                            Span::raw("Select Risk   "),
                            Span::styled("[Tab] ", Style::default().fg(t.accent())),
                            Span::raw("Pane   "),
                            Span::styled("[c] ", Style::default().fg(t.accent())),
                            Span::raw("Logs   "),
                            Span::styled("[o] ", Style::default().fg(t.accent())),
                            Span::raw("Open in IDE   "),
                            Span::styled("[?] ", Style::default().fg(t.accent())),
                            Span::raw("Help"),
                        ],
                        crate::ui::app::WorkTabMode::Console => vec![
                            Span::styled("[/] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Command Dock   "),
                            Span::styled("[p] ", Style::default().fg(t.accent())),
                            Span::raw("Projects   "),
                            Span::styled("[w] ", Style::default().fg(t.accent())),
                            Span::raw("Risks View   "),
                            Span::styled("[↑↓] ", Style::default().fg(t.accent())),
                            Span::raw("Scroll Output   "),
                            Span::styled("[o] ", Style::default().fg(t.accent())),
                            Span::raw("Open Target   "),
                            Span::styled("[?] ", Style::default().fg(t.accent())),
                            Span::raw("Help"),
                        ],
                    },
                    ActiveTab::Explore => if app.knowledge_view_mode == crate::ui::app::KnowledgeViewMode::Living {
                        vec![
                            Span::styled("[/] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Dock   "),
                            Span::styled("[m] ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                            Span::raw("Curated Docs   "),
                            Span::styled("[f] ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                            Span::raw("Kind Filter   "),
                            Span::styled("[x] ", Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)),
                            Span::raw("Delete   "),
                            Span::styled("[?] ", Style::default().fg(t.accent())),
                            Span::raw("Help"),
                        ]
                    } else if app.explore_tree_mode {
                        vec![
                            Span::styled("[/] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Command Dock   "),
                            Span::styled("[E] ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                            Span::raw("Expand All   "),
                            Span::styled("[X] ", Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)),
                            Span::raw("Collapse All   "),
                            Span::styled("[Space] ", Style::default().fg(t.accent())),
                            Span::raw("Toggle   "),
                            Span::styled("[d] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Durable   "),
                            Span::styled("[t] ", Style::default().fg(t.accent())),
                            Span::raw("List   "),
                            Span::styled("[Enter] ", Style::default().fg(t.accent())),
                            Span::raw("Read   "),
                            Span::styled("[o] ", Style::default().fg(t.accent())),
                            Span::raw("IDE   "),
                            Span::styled("[?] ", Style::default().fg(t.accent())),
                            Span::raw("Help"),
                        ]
                    } else {
                        vec![
                            Span::styled("[/] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Command Dock   "),
                            Span::styled("[t] ", Style::default().fg(t.accent())),
                            Span::raw("Tree   "),
                            Span::styled("[d] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Durable   "),
                            Span::styled("[c] ", Style::default().fg(t.accent())),
                            Span::raw("Category   "),
                            Span::styled("[Enter] ", Style::default().fg(t.accent())),
                            Span::raw("Read   "),
                            Span::styled("[o] ", Style::default().fg(t.accent())),
                            Span::raw("IDE   "),
                            Span::styled("[?] ", Style::default().fg(t.accent())),
                            Span::raw("Help"),
                        ]
                    },
                    ActiveTab::Directives => vec![
                        Span::styled("[/] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                        Span::raw("Command Dock   "),
                        Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                        Span::raw("Select   "),
                        Span::styled("[r] ", Style::default().fg(t.accent())),
                        Span::raw("Toggle Active   "),
                        Span::styled("[c] ", Style::default().fg(t.accent())),
                        Span::raw("Taxonomy   "),
                        Span::styled("[n] ", Style::default().fg(t.accent())),
                        Span::raw("New   "),
                        Span::styled("[o] ", Style::default().fg(t.accent())),
                        Span::raw("Open in IDE   "),
                        Span::styled("[?] ", Style::default().fg(t.accent())),
                        Span::raw("Help"),
                    ],
                    ActiveTab::Sessions => match app.governance_tab_mode {
                        crate::ui::app::GovernanceTabMode::Sessions => vec![
                            Span::styled("[/] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Command Dock   "),
                            Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                            Span::raw("Select   "),
                            Span::styled("[h] ", Style::default().fg(t.accent())),
                            Span::raw("Harness Filter   "),
                            Span::styled("[g] ", Style::default().fg(t.accent())),
                            Span::raw("Grants   "),
                            Span::styled("[y] ", Style::default().fg(t.accent())),
                            Span::raw("Copy Scorecard   "),
                            Span::styled("[?] ", Style::default().fg(t.accent())),
                            Span::raw("Help"),
                        ],
                        crate::ui::app::GovernanceTabMode::Grants => vec![
                            Span::styled("[/] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("Command Dock   "),
                            Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                            Span::raw("Select   "),
                            Span::styled("[g] ", Style::default().fg(t.accent())),
                            Span::raw("Runs   "),
                            Span::styled("[n] ", Style::default().fg(t.accent())),
                            Span::raw("Issue Grant   "),
                            Span::styled("[r] ", Style::default().fg(t.accent())),
                            Span::raw("Revoke   "),
                            Span::styled("[y] ", Style::default().fg(t.accent())),
                            Span::raw("Copy Token   "),
                            Span::styled("[?] ", Style::default().fg(t.accent())),
                            Span::raw("Help"),
                        ],
                    },
                    ActiveTab::Settings => vec![
                        Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                        Span::raw("Navigate   "),
                        Span::styled("[Tab/←→] ", Style::default().fg(t.accent())),
                        Span::raw("Switch Pane   "),
                        Span::styled("[Enter/Space] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                        Span::raw("Action/Toggle   "),
                        Span::styled("[+-] ", Style::default().fg(t.accent())),
                        Span::raw("Nudge   "),
                        Span::styled("[?] ", Style::default().fg(t.accent())),
                        Span::raw("Help"),
                    ],
                    ActiveTab::Reader => vec![
                        Span::styled("[Esc] ", Style::default().fg(t.accent())),
                        Span::raw("Back to Knowledge   "),
                        Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                        Span::raw("Scroll   "),
                        Span::styled("[PgDn/Space] ", Style::default().fg(t.accent())),
                        Span::raw("Page   "),
                        Span::styled("[v] ", Style::default().fg(t.accent())),
                        Span::raw("Toggle Raw   "),
                        Span::styled("[y] ", Style::default().fg(t.accent())),
                        Span::raw("Copy   "),
                        Span::styled("[?] ", Style::default().fg(t.accent())),
                        Span::raw("Help"),
                    ],
                }
            };
            Line::from(keys)
        };

        let mut lines = Vec::new();
        if area.height >= 2 {
            lines.push(Line::from(""));
        }
        lines.push(text);

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }
}
