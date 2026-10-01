use crate::ui::app::{ActiveTab, App};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct Header;

impl Header {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let mut tabs = vec![
            (ActiveTab::Work, "[1] Work & Risks"),
            (ActiveTab::Explore, "[2] Explore"),
            (ActiveTab::Directives, "[3] Directives"),
            (ActiveTab::Sessions, "[4] Sessions"),
            (ActiveTab::Settings, "[5] Settings"),
        ];

        if app.active_tab == ActiveTab::Reader {
            tabs.push((ActiveTab::Reader, "[Reader]"));
        }

        // Line 0: Main Navigation & Branding
        let mut tab_spans = vec![
            Span::styled(" HyperKB ", t.title()),
            Span::styled(format!(" [{}] ", app.collection_id), Style::default().fg(t.text_muted())),
            Span::raw("    "),
        ];

        for (tab, label) in tabs {
            if app.active_tab == tab {
                tab_spans.push(Span::styled(
                    format!(" {} ", label),
                    t.active_tab(),
                ));
            } else {
                tab_spans.push(Span::styled(
                    format!(" {} ", label),
                    Style::default().fg(t.text_primary()),
                ));
            }
            tab_spans.push(Span::raw("  "));
        }

        // Line 1: Context Sub-Header (Full-width taxonomy/category filter or status bar)
        let sub_spans = match app.active_tab {
            ActiveTab::Directives => {
                let mut spans = vec![
                    Span::styled("  Policy Taxonomy:  ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(" [c] ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                ];
                for cat in App::DIRECTIVE_CATEGORIES {
                    if *cat == app.directive_category {
                        spans.push(Span::styled(
                            format!(" [{}] ", cat.to_uppercase()),
                            Style::default()
                                .fg(t.status_proposed())
                                .bg(t.bg_panel())
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        spans.push(Span::styled(
                            format!("  {}  ", cat),
                            Style::default().fg(t.text_muted()),
                        ));
                    }
                    spans.push(Span::raw(" "));
                }
                spans
            }
            ActiveTab::Explore => {
                let mode_pill = if app.explore_tree_mode {
                    Span::styled(" [t: TREE VIEW] ", Style::default().fg(t.bg()).bg(t.accent()).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled(" [t: LIST VIEW] ", Style::default().fg(t.accent()).bg(t.bg_panel()).add_modifier(Modifier::BOLD))
                };
                let mut spans = vec![
                    Span::styled("  View: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    mode_pill,
                    Span::styled("  |  Category [c]: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                ];
                for cat in App::CATEGORIES {
                    if *cat == app.selected_category {
                        spans.push(Span::styled(
                            format!(" [{}] ", cat.to_uppercase()),
                            Style::default()
                                .fg(t.status_proposed())
                                .bg(t.bg_panel())
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        spans.push(Span::styled(
                            format!("  {}  ", cat),
                            Style::default().fg(t.text_muted()),
                        ));
                    }
                    spans.push(Span::raw(" "));
                }
                spans
            }
            ActiveTab::Sessions => {
                let active_count = app.sessions.iter().filter(|s| s.status == "active").count();
                vec![
                    Span::styled("  Sessions Telemetry:  ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("Total: {}  |  Active: {}  |  Scoring Proof: [e]  ", app.sessions.len(), active_count), Style::default().fg(t.text_primary())),
                ]
            }
            ActiveTab::Work => {
                vec![
                    Span::styled("  Work & Verification:  ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("Branch: master  |  Active Directives: {}  |  Pre-Commit Gate: Enabled  ", app.directives.iter().filter(|d| d.status == "active").count()), Style::default().fg(t.text_primary())),
                ]
            }
            ActiveTab::Settings => {
                vec![
                    Span::styled("  Configuration & Policy Knobs:  ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled("Select knob [↑↓/jk]  |  Adjust [←→/hl/+-]  |  Commit [Enter]  |  Cycle Theme [T]  |  Mouse [m]  ", Style::default().fg(t.text_primary())),
                ]
            }
            ActiveTab::Reader => {
                vec![
                    Span::styled("  Document Reader:  ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled("[Esc / Enter] Back to List  |  [j/k / Space] Scroll  |  [v] Toggle Raw  |  [y] Copy Content  ", Style::default().fg(t.accent())),
                ]
            }
        };

        let paragraph = Paragraph::new(vec![
            Line::from(""),
            Line::from(tab_spans),
            Line::from(""),
            Line::from(sub_spans),
            Line::from(""),
        ]);

        frame.render_widget(paragraph, area);
    }

    pub fn handle_click(app: &mut App, db: &crate::storage::Database, col: u16, row: u16) -> bool {
        if row == 1 {
            let prefix_len = (9 + 2 + app.collection_id.len() + 2 + 4) as u16;
            let tabs = [
                (ActiveTab::Work, "[1] Work & Risks"),
                (ActiveTab::Explore, "[2] Explore"),
                (ActiveTab::Directives, "[3] Directives"),
                (ActiveTab::Sessions, "[4] Sessions"),
                (ActiveTab::Settings, "[5] Settings"),
            ];
            let mut cur_x = prefix_len;
            for (tab, label) in tabs {
                let tab_w = (label.len() + 2) as u16;
                if col >= cur_x && col < cur_x + tab_w {
                    app.switch_tab(tab);
                    return true;
                }
                cur_x += tab_w + 2;
            }
            if app.active_tab == ActiveTab::Reader {
                let tab_w = "[Reader]".len() as u16 + 2;
                if col >= cur_x && col < cur_x + tab_w {
                    app.switch_tab(ActiveTab::Reader);
                    return true;
                }
            }
        } else if row == 3 {
            match app.active_tab {
                ActiveTab::Directives => {
                    // "  Policy Taxonomy:  [c] " is 24 chars
                    if col >= 20 && col <= 23 {
                        app.next_directive_category(db);
                        return true;
                    }
                    let mut cur_x = 24u16;
                    for cat in App::DIRECTIVE_CATEGORIES {
                        let pill_w = (cat.len() + 4) as u16;
                        if col >= cur_x && col < cur_x + pill_w {
                            app.set_directive_category(cat, db);
                            return true;
                        }
                        cur_x += pill_w + 1;
                    }
                }
                ActiveTab::Explore => {
                    // "  View: " (8 chars) -> [t: TREE VIEW] (16 chars, col 8..23)
                    if col >= 8 && col <= 23 {
                        app.toggle_explore_tree_mode();
                        return true;
                    }
                    // "  |  Category [c]: " (19 chars: 24..42). [c] is around col 38..40
                    if col >= 37 && col <= 41 {
                        app.next_category(db);
                        return true;
                    }
                    let mut cur_x = 43u16;
                    for cat in App::CATEGORIES {
                        let pill_w = (cat.len() + 4) as u16;
                        if col >= cur_x && col < cur_x + pill_w {
                            app.set_category(cat, db);
                            return true;
                        }
                        cur_x += pill_w + 1;
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
            let mouse_label = if app.mouse_capture { "Mouse: ON [m]" } else { "Mouse: OFF (copy) [m]" };
            let mouse_fg = if app.mouse_capture { t.accent() } else { t.status_proposed() };
            let keys = match app.active_tab {
                ActiveTab::Work => vec![
                    Span::styled("[1-5] ", Style::default().fg(t.accent())),
                    Span::raw("Tabs  "),
                    Span::styled("[Tab] ", Style::default().fg(t.accent())),
                    Span::raw("Pane  "),
                    Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                    Span::raw("Nav  "),
                    Span::styled("[y] ", Style::default().fg(t.accent())),
                    Span::raw("Copy  "),
                    Span::styled("[T] ", Style::default().fg(t.accent())),
                    Span::raw("Theme  "),
                    Span::styled(format!("[{}] ", mouse_label), Style::default().fg(mouse_fg)),
                    Span::styled("[?] ", Style::default().fg(t.accent())),
                    Span::raw("Help  "),
                    Span::styled("[q] ", Style::default().fg(t.accent())),
                    Span::raw("Quit"),
                ],
                ActiveTab::Explore => vec![
                    Span::styled("[t] ", Style::default().fg(t.accent())),
                    Span::raw("Tree/List  "),
                    Span::styled("[c] ", Style::default().fg(t.accent())),
                    Span::raw("Category  "),
                    Span::styled("[Enter] ", Style::default().fg(t.accent())),
                    Span::raw("Read  "),
                    Span::styled("[y] ", Style::default().fg(t.accent())),
                    Span::raw("Copy  "),
                    Span::styled("[Tab] ", Style::default().fg(t.accent())),
                    Span::raw("Pane  "),
                    Span::styled("[T] ", Style::default().fg(t.accent())),
                    Span::raw("Theme  "),
                    Span::styled(format!("[{}] ", mouse_label), Style::default().fg(mouse_fg)),
                    Span::styled("[?] ", Style::default().fg(t.accent())),
                    Span::raw("Help  "),
                    Span::styled("[q] ", Style::default().fg(t.accent())),
                    Span::raw("Quit"),
                ],
                ActiveTab::Directives => vec![
                    Span::styled("[c] ", Style::default().fg(t.accent())),
                    Span::raw("Taxonomy  "),
                    Span::styled("[r] ", Style::default().fg(t.accent())),
                    Span::raw("Retire  "),
                    Span::styled("[Enter] ", Style::default().fg(t.accent())),
                    Span::raw("Read  "),
                    Span::styled("[y] ", Style::default().fg(t.accent())),
                    Span::raw("Copy  "),
                    Span::styled("[T] ", Style::default().fg(t.accent())),
                    Span::raw("Theme  "),
                    Span::styled(format!("[{}] ", mouse_label), Style::default().fg(mouse_fg)),
                    Span::styled("[?] ", Style::default().fg(t.accent())),
                    Span::raw("Help  "),
                    Span::styled("[q] ", Style::default().fg(t.accent())),
                    Span::raw("Quit"),
                ],
                ActiveTab::Sessions => vec![
                    Span::styled("[e] ", Style::default().fg(t.accent())),
                    Span::raw("Scoring Proof  "),
                    Span::styled("[y] ", Style::default().fg(t.accent())),
                    Span::raw("Copy  "),
                    Span::styled("[Tab] ", Style::default().fg(t.accent())),
                    Span::raw("Pane  "),
                    Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                    Span::raw("Nav  "),
                    Span::styled("[T] ", Style::default().fg(t.accent())),
                    Span::raw("Theme  "),
                    Span::styled(format!("[{}] ", mouse_label), Style::default().fg(mouse_fg)),
                    Span::styled("[?] ", Style::default().fg(t.accent())),
                    Span::raw("Help  "),
                    Span::styled("[q] ", Style::default().fg(t.accent())),
                    Span::raw("Quit"),
                ],
                ActiveTab::Settings => vec![
                    Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                    Span::raw("Select  "),
                    Span::styled("[←→/hl/+-] ", Style::default().fg(t.accent())),
                    Span::raw("Adjust  "),
                    Span::styled("[Enter] ", Style::default().fg(t.accent())),
                    Span::raw("Save  "),
                    Span::styled("[T] ", Style::default().fg(t.accent())),
                    Span::raw("Theme  "),
                    Span::styled(format!("[{}] ", mouse_label), Style::default().fg(mouse_fg)),
                    Span::styled("[?] ", Style::default().fg(t.accent())),
                    Span::raw("Help  "),
                    Span::styled("[q] ", Style::default().fg(t.accent())),
                    Span::raw("Quit"),
                ],
                ActiveTab::Reader => vec![
                    Span::styled("[Esc] ", Style::default().fg(t.accent())),
                    Span::raw("Back  "),
                    Span::styled("[↑↓/jk] ", Style::default().fg(t.accent())),
                    Span::raw("Line  "),
                    Span::styled("[PgDn/PgUp/Space] ", Style::default().fg(t.accent())),
                    Span::raw("Page  "),
                    Span::styled("[v] ", Style::default().fg(t.accent())),
                    Span::raw("Raw  "),
                    Span::styled("[y] ", Style::default().fg(t.accent())),
                    Span::raw("Copy  "),
                    Span::styled("[T] ", Style::default().fg(t.accent())),
                    Span::raw("Theme  "),
                    Span::styled(format!("[{}] ", mouse_label), Style::default().fg(mouse_fg)),
                    Span::styled("[?] ", Style::default().fg(t.accent())),
                    Span::raw("Help  "),
                    Span::styled("[q] ", Style::default().fg(t.accent())),
                    Span::raw("Quit"),
                ],
            };
            Line::from(keys)
        };

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(t.border())),
        );

        frame.render_widget(paragraph, area);
    }
}
