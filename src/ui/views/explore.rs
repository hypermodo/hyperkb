use crate::domain::{DocumentKind, DocumentStatus};
use crate::ui::app::{App, ExploreTreeItem, FocusedPane};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame,
};

pub struct ExploreView;

impl ExploreView {
    pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
        let list_width = app.list_width(area.width);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Min(40)])
            .split(area);

        if app.explore_tree_mode {
            Self::render_tree_view(frame, app, chunks[0]);
            Self::render_tree_preview(frame, app, chunks[1]);
        } else {
            Self::render_document_list(frame, app, chunks[0]);
            Self::render_document_preview(frame, app, chunks[1]);
        }
    }

    fn render_document_list(frame: &mut Frame, app: &mut App, area: Rect) {
        let t = app.theme;
        let border_color = if app.focused_pane == FocusedPane::List {
            t.border_focused()
        } else {
            t.border()
        };

        let visible_docs = app.visible_explore_docs();
        let visible_count = visible_docs.len();
        let selected_doc_idx = app.selected_doc_idx;
        let selected_list_pos = app.documents_list_state.selected().unwrap_or(0);
        let items: Vec<ListItem<'static>> = visible_docs
            .iter()
            .map(|(doc_idx, doc)| {
                let is_selected = *doc_idx == selected_doc_idx;

                let (badge_text, badge_style) = match doc.status {
                    DocumentStatus::Accepted => ("● ACCEPTED ", t.badge_accepted()),
                    DocumentStatus::Proposed => ("○ PROPOSED ", t.badge_proposed()),
                    DocumentStatus::Pending => ("○ PENDING ", t.badge_proposed()),
                    DocumentStatus::InProgress => {
                        ("▶ IN PROGRESS ", t.badge_accepted())
                    }
                    DocumentStatus::Completed => ("✔ DONE ", t.badge_resolved()),
                    DocumentStatus::Blocked => ("✖ BLOCKED ", t.badge_conflict()),
                    DocumentStatus::Open => ("▲ OPEN ", t.badge_risk()),
                    DocumentStatus::Acknowledged => ("✔ ACKNOWLEDGED ", t.badge_acknowledged()),
                    DocumentStatus::Resolved => ("✔ RESOLVED ", t.badge_resolved()),
                    DocumentStatus::Superseded => (
                        "✕ SUPERSEDED ",
                        Style::default().fg(t.status_superseded()),
                    ),
                    DocumentStatus::Conflict => {
                        ("! CONFLICT ", t.badge_conflict())
                    }
                    DocumentStatus::Archived => {
                        ("🗄 ARCHIVED ", Style::default().fg(t.status_superseded()))
                    }
                    DocumentStatus::Unknown => {
                        if doc.kind == DocumentKind::Risk {
                            ("▲ RISK ", t.badge_risk())
                        } else {
                            ("· DOC ", Style::default().fg(t.status_unknown()))
                        }
                    }
                };

                let title = Span::styled(
                    doc.title.clone(),
                    if is_selected {
                        t.selected_row()
                    } else {
                        Style::default().fg(t.text_primary())
                    },
                );

                let path_span = Span::styled(
                    format!("  {}", &doc.path),
                    Style::default().fg(t.text_muted()),
                );

                ListItem::new(vec![
                    Line::from(vec![Span::styled(badge_text, badge_style), title]),
                    Line::from(path_span),
                ])
            })
            .collect();
        drop(visible_docs);

        let title_text = format!(" Knowledge Documents ({}) [t: Tree View] ", visible_count);
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
                    .padding(Padding::new(2, 2, 1, 1))
                    .title(Span::styled(title_text, t.title())),
            )
            .highlight_style(t.selected_row());

        frame.render_stateful_widget(list, area, &mut app.documents_list_state);

        if visible_count > 0 {
            let mut scrollbar_state = ScrollbarState::new(visible_count.saturating_sub(1))
                .position(selected_list_pos);
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn render_document_preview(frame: &mut Frame, app: &App, area: Rect) {
        let t = app.theme;
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
            .title(Span::styled(" Document Preview [Tab: focus, Enter: Reader, y: Copy] ", t.title()));

        if let Some(doc) = app.selected_document() {
            let mut text = vec![
                Line::from(vec![
                    Span::styled("Title:     ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(&doc.title, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Path:      ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(&doc.path, Style::default().fg(t.accent())),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Status:    ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("● {}", doc.status.as_str()), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::raw("       "),
                    Span::styled("Kind: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(doc.kind.as_str().to_uppercase(), Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                ]),
            ];

            if let Ok(parsed) = crate::core::MetadataParser::parse(&doc.content) {
                if let Some(ref del) = parsed.meta.as_ref().and_then(|m| m.delegation.as_ref()) {
                    let grant_str = del.grant_id.to_string();
                    let short_grant = if grant_str.len() >= 8 { &grant_str[..8] } else { &grant_str };
                    text.push(Line::from(""));
                    text.push(Line::from(vec![
                        Span::styled("Governance: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("Agent [{}] ", del.agent_id), Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("via Grant #{} ", short_grant), Style::default().fg(t.accent())),
                        Span::styled(format!("(Authorizer: {})", del.granted_by), Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                    ]));
                }
            }

            let rule_len = (area.width.saturating_sub(6) as usize).max(10);
            let content_header = if rule_len > 24 {
                format!("────── Content Preview {}", "─".repeat(rule_len.saturating_sub(25)))
            } else {
                "─".repeat(rule_len)
            };
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(content_header, Style::default().fg(t.border()))));
            text.push(Line::from(""));

            let preview_width = area.width.saturating_sub(6) as usize;
            let formatted_body = crate::ui::markdown::MarkdownFormatter::format_markdown_with_theme(&doc.content, preview_width, &t);
            text.extend(formatted_body);

            let total_lines = text.len();
            let visible_lines = area.height.saturating_sub(4) as usize;
            let max_scroll = total_lines.saturating_sub(visible_lines);
            let scroll = app.preview_scroll_offset.min(max_scroll);

            let paragraph = Paragraph::new(text)
                .block(block)
                .scroll((scroll as u16, 0))
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);

            if total_lines > visible_lines {
                let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll);
                let scrollbar = Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");
                frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
            }
        } else {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled("No documents found in this view.", Style::default().fg(t.text_muted()))),
            ];
            let paragraph = Paragraph::new(empty_text)
                .block(block)
                .style(Style::default().fg(t.text_muted()));
            frame.render_widget(paragraph, area);
        }
    }

    fn render_tree_view(frame: &mut Frame, app: &mut App, area: Rect) {
        let t = app.theme;
        let border_color = if app.focused_pane == FocusedPane::List {
            t.border_focused()
        } else {
            t.border()
        };

        let tree = app.build_explore_tree();
        let items: Vec<ListItem> = tree
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let is_selected = idx == app.selected_tree_idx;
                match item {
                    ExploreTreeItem::Folder { path, name, doc_count, is_collapsed } => {
                        let (icon, icon_style) = if *is_collapsed {
                            ("[+] ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD))
                        } else {
                            ("[-] ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))
                        };
                        let folder_style = if is_selected {
                            t.selected_row()
                        } else {
                            Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                        };

                        let depth = path.split('/').filter(|s| !s.is_empty()).count().saturating_sub(1);
                        let indent = "  ".repeat(depth);
                        let leaf_name = path.split('/').filter(|s| !s.is_empty()).last().unwrap_or(name);

                        ListItem::new(vec![
                            Line::from(vec![
                                Span::raw(format!(" {}", indent)),
                                Span::styled(icon, icon_style),
                                Span::styled(format!("{}/", leaf_name), folder_style),
                                Span::raw("  "),
                                Span::styled(format!("({} docs)", doc_count), Style::default().fg(t.text_muted())),
                            ]),
                        ])
                    }
                    ExploreTreeItem::Doc { title, status, path, .. } => {
                        let doc_style = if is_selected {
                            t.selected_row()
                        } else {
                            Style::default().fg(t.text_primary())
                        };

                        let (badge_text, badge_style) = match status.as_str() {
                            "accepted" => ("● ", t.badge_accepted()),
                            "proposed" => ("○ ", t.badge_proposed()),
                            "open" => ("▲ ", t.badge_risk()),
                            "acknowledged" => ("✔ ", t.badge_acknowledged()),
                            "resolved" => ("✔ ", t.badge_resolved()),
                            "retired" | "superseded" => ("✕ ", Style::default().fg(t.status_superseded())),
                            _ => ("· ", Style::default().fg(t.status_unknown())),
                        };

                        let depth = path.split('/').filter(|s| !s.is_empty()).count().saturating_sub(1);
                        let indent = "  ".repeat(depth + 1);

                        ListItem::new(vec![
                            Line::from(vec![
                                Span::raw(format!(" {}", indent)),
                                Span::styled(badge_text, badge_style),
                                Span::styled(title, doc_style),
                            ]),
                        ])
                    }
                }
            })
            .collect();

        let doc_count = app.visible_explore_docs().len();
        let title_text = format!(" Tree View [E: Expand All • X: Collapse All • Space: Toggle • t: List] ({}) ", doc_count);
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
                    .padding(Padding::new(2, 2, 1, 1))
                    .title(Span::styled(title_text, t.title())),
            )
            .highlight_style(t.selected_row());

        frame.render_stateful_widget(list, area, &mut app.tree_list_state);

        let total_tree = tree.len();
        if total_tree > 0 {
            let mut scrollbar_state = ScrollbarState::new(total_tree.saturating_sub(1))
                .position(app.selected_tree_idx);
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn render_tree_preview(frame: &mut Frame, app: &App, area: Rect) {
        let t = app.theme;
        if let Some(item) = app.selected_tree_item() {
            match item {
                ExploreTreeItem::Doc { .. } => {
                    Self::render_document_preview(frame, app, area);
                }
                ExploreTreeItem::Folder { path, name, doc_count, is_collapsed } => {
                    let border_color = if app.focused_pane == FocusedPane::Detail {
                        t.border_focused()
                    } else {
                        t.border()
                    };

                    let clean_name = name.trim_end_matches('/');
                    let block = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                        .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
                        .padding(Padding::new(2, 2, 1, 1))
                        .title(Span::styled(format!(" Directory: {}/ ", clean_name), t.title()));

                    let matching_docs: Vec<&crate::domain::Document> = app.documents.iter().filter(|d| {
                        let f = std::path::Path::new(&d.path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                        let clean = if f.is_empty() { "general" } else { &f };
                        clean == path
                    }).collect();

                    let rule_len = (area.width.saturating_sub(6) as usize).max(10);
                    let docs_header = if rule_len > 32 {
                        format!("────── Documents in this Directory {}", "─".repeat(rule_len.saturating_sub(33)))
                    } else {
                        "─".repeat(rule_len)
                    };

                    let mut text = vec![
                        Line::from(vec![
                            Span::styled("Directory: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("{}/", clean_name), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                            Span::raw("    "),
                            Span::styled(format!("({} documents)", doc_count), Style::default().fg(t.accent())),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(docs_header, Style::default().fg(t.border()))),
                        Line::from(""),
                    ];

                    for doc in matching_docs {
                        let badge = match doc.status.as_str() {
                            "accepted" => ("● ACCEPTED", t.badge_accepted()),
                            "proposed" => ("○ PROPOSED", t.badge_proposed()),
                            "open" => ("▲ OPEN", t.badge_risk()),
                            "acknowledged" => ("✔ ACKNOWLEDGED", t.badge_acknowledged()),
                            _ => ("· DOC", Style::default().fg(t.status_unknown())),
                        };
                        text.push(Line::from(vec![
                            Span::styled(badge.0, badge.1),
                            Span::raw("  "),
                            Span::styled(&doc.title, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                        ]));
                        text.push(Line::from(vec![
                            Span::raw("   "),
                            Span::styled(&doc.path, Style::default().fg(t.text_muted())),
                        ]));
                        text.push(Line::from(""));
                    }

                    let rule_len = (area.width.saturating_sub(6) as usize).max(10);
                    text.push(Line::from(Span::styled("─".repeat(rule_len), Style::default().fg(t.border()))));
                    text.push(Line::from(vec![
                        Span::styled("Press ", Style::default().fg(t.text_primary())),
                        Span::styled("[Enter]", t.key_badge()),
                        Span::styled(" or ", Style::default().fg(t.text_primary())),
                        Span::styled("[Space]", t.key_badge()),
                        Span::styled(if is_collapsed { " to expand this folder" } else { " to collapse this folder" }, Style::default().fg(t.text_primary())),
                        Span::styled(", or ", Style::default().fg(t.text_primary())),
                        Span::styled("[t]", t.key_badge()),
                        Span::styled(" to switch back to List view.", Style::default().fg(t.text_primary())),
                    ]));

                    let total_lines = text.len();
                    let visible_lines = area.height.saturating_sub(4) as usize;
                    let max_scroll = total_lines.saturating_sub(visible_lines);
                    let scroll = app.preview_scroll_offset.min(max_scroll);

                    let p = Paragraph::new(text)
                        .block(block)
                        .scroll((scroll as u16, 0))
                        .wrap(Wrap { trim: false });
                    frame.render_widget(p, area);

                    if total_lines > visible_lines {
                        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll);
                        let scrollbar = Scrollbar::default()
                            .orientation(ScrollbarOrientation::VerticalRight)
                            .begin_symbol(Some("▲"))
                            .end_symbol(Some("▼"))
                            .track_symbol(Some("│"))
                            .thumb_symbol("█");
                        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
                    }
                }
            }
        } else {
            Self::render_document_preview(frame, app, area);
        }
    }
}
