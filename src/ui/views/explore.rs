use crate::domain::{DocumentKind, DocumentStatus};
use crate::ui::app::{App, ExploreTreeItem, FocusedPane};
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

pub struct ExploreView;

impl ExploreView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let list_width = (area.width * 38 / 100).clamp(36, 68);
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

    fn render_document_list(frame: &mut Frame, app: &App, area: Rect) {
        let border_color = if app.focused_pane == FocusedPane::List {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER
        };

        let items: Vec<ListItem> = app
            .documents
            .iter()
            .enumerate()
            .map(|(idx, doc)| {
                let is_selected = idx == app.selected_doc_idx;

                let (badge_text, badge_style) = match doc.status {
                    DocumentStatus::Accepted => ("● ACCEPTED ", Theme::badge_accepted()),
                    DocumentStatus::Proposed => ("○ PROPOSED ", Theme::badge_proposed()),
                    DocumentStatus::Open => ("▲ OPEN ", Theme::badge_risk()),
                    DocumentStatus::Acknowledged => ("✔ ACKNOWLEDGED ", Theme::badge_acknowledged()),
                    DocumentStatus::Resolved => ("✔ RESOLVED ", Theme::badge_resolved()),
                    DocumentStatus::Superseded => (
                        "✕ SUPERSEDED ",
                        Style::default().fg(Theme::STATUS_SUPERSEDED),
                    ),
                    DocumentStatus::Conflict => {
                        ("! CONFLICT ", Theme::badge_conflict())
                    }
                    DocumentStatus::Unknown => {
                        if doc.kind == DocumentKind::Risk {
                            ("▲ RISK ", Theme::badge_risk())
                        } else {
                            ("· DOC ", Style::default().fg(Theme::STATUS_UNKNOWN))
                        }
                    }
                };

                let title = Span::styled(
                    &doc.title,
                    if is_selected {
                        Theme::selected_row()
                    } else {
                        Style::default().fg(Color::White)
                    },
                );

                let path_span = Span::styled(
                    format!("  {}", &doc.path),
                    Style::default().fg(Theme::TEXT_MUTED),
                );

                ListItem::new(vec![
                    Line::from(vec![Span::styled(badge_text, badge_style), title]),
                    Line::from(path_span),
                    Line::from(""),
                ])
            })
            .collect();

        let title_text = format!(" Knowledge Documents ({}) ", app.documents.len());
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .padding(Padding::new(2, 2, 1, 1))
                    .title(Span::styled(title_text, Theme::title())),
            )
            .highlight_style(Theme::selected_row());

        frame.render_widget(list, area);
    }

    fn render_document_preview(frame: &mut Frame, app: &App, area: Rect) {
        let border_color = if app.focused_pane == FocusedPane::Detail {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER
        };

        let title_style = if app.focused_pane == FocusedPane::Detail {
            Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Theme::title()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" Document Preview [Tab to focus, Enter for Reader] ", title_style));

        if let Some(doc) = app.selected_document() {
            let mut text = vec![
                Line::from(vec![
                    Span::styled("Title:     ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                    Span::styled(&doc.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Path:      ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                    Span::styled(&doc.path, Style::default().fg(Theme::ACCENT)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Status:    ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("● {}", doc.status.as_str()), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw("       "),
                    Span::styled("Kind: ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                    Span::styled(doc.kind.as_str().to_uppercase(), Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)),
                ]),
            ];

            if let Ok(parsed) = crate::core::MetadataParser::parse(&doc.content) {
                if let Some(ref del) = parsed.meta.as_ref().and_then(|m| m.delegation.as_ref()) {
                    let grant_str = del.grant_id.to_string();
                    let short_grant = if grant_str.len() >= 8 { &grant_str[..8] } else { &grant_str };
                    text.push(Line::from(""));
                    text.push(Line::from(vec![
                        Span::styled("Governance: ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("Agent [{}] ", del.agent_id), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("via Grant #{} ", short_grant), Style::default().fg(Color::LightBlue)),
                        Span::styled(format!("(Authorizer: {})", del.granted_by), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    ]));
                }
            }

            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "────── Content Preview ──────────────────────────────────────",
                Style::default().fg(Theme::BORDER),
            )));
            text.push(Line::from(""));

            let preview_width = area.width.saturating_sub(6) as usize;
            let formatted_body = crate::ui::markdown::MarkdownFormatter::format_markdown(&doc.content, preview_width);
            text.extend(formatted_body);

            let paragraph = Paragraph::new(text)
                .block(block)
                .scroll((app.preview_scroll_offset as u16, 0))
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        } else {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled("No documents found in this view.", Style::default().fg(Theme::TEXT_MUTED))),
            ];
            let paragraph = Paragraph::new(empty_text)
                .block(block)
                .style(Style::default().fg(Theme::TEXT_MUTED));
            frame.render_widget(paragraph, area);
        }
    }

    fn render_tree_view(frame: &mut Frame, app: &App, area: Rect) {
        let border_color = if app.focused_pane == FocusedPane::List {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER
        };

        let tree = app.build_explore_tree();
        let items: Vec<ListItem> = tree
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let is_selected = idx == app.selected_tree_idx;
                match item {
                    ExploreTreeItem::Folder { name, doc_count, is_collapsed, .. } => {
                        let (icon, icon_style) = if *is_collapsed {
                            ("[+] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        } else {
                            ("[-] ", Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD))
                        };
                        let folder_style = if is_selected {
                            Theme::selected_row()
                        } else {
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                        };

                        let clean_name = name.trim_end_matches('/');
                        ListItem::new(vec![
                            Line::from(vec![
                                Span::raw(" "),
                                Span::styled(icon, icon_style),
                                Span::styled(format!("{}/", clean_name), folder_style),
                                Span::raw("  "),
                                Span::styled(format!("({} docs)", doc_count), Style::default().fg(Theme::TEXT_MUTED)),
                            ]),
                            Line::from(""),
                        ])
                    }
                    ExploreTreeItem::Doc { title, status, .. } => {
                        let doc_style = if is_selected {
                            Theme::selected_row()
                        } else {
                            Style::default().fg(Color::White)
                        };

                        let (badge_text, badge_style) = match status.as_str() {
                            "accepted" => ("● ", Theme::badge_accepted()),
                            "proposed" => ("○ ", Theme::badge_proposed()),
                            "open" => ("▲ ", Theme::badge_risk()),
                            "acknowledged" => ("✔ ", Theme::badge_acknowledged()),
                            "resolved" => ("✔ ", Theme::badge_resolved()),
                            "retired" | "superseded" => ("✕ ", Style::default().fg(Theme::STATUS_SUPERSEDED)),
                            _ => ("· ", Style::default().fg(Theme::STATUS_UNKNOWN)),
                        };

                        ListItem::new(vec![
                            Line::from(vec![
                                Span::raw("      "),
                                Span::styled(badge_text, badge_style),
                                Span::styled(title, doc_style),
                            ]),
                            Line::from(""),
                        ])
                    }
                }
            })
            .collect();

        let title_text = format!(" Tree View [t: List View, Space: Expand] ({}) ", app.documents.len());
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .padding(Padding::new(2, 2, 1, 1))
                    .title(Span::styled(title_text, Theme::title())),
            )
            .highlight_style(Theme::selected_row());

        frame.render_widget(list, area);
    }

    fn render_tree_preview(frame: &mut Frame, app: &App, area: Rect) {
        if let Some(item) = app.selected_tree_item() {
            match item {
                ExploreTreeItem::Doc { .. } => {
                    Self::render_document_preview(frame, app, area);
                }
                ExploreTreeItem::Folder { path, name, doc_count, is_collapsed } => {
                    let border_color = if app.focused_pane == FocusedPane::Detail {
                        Theme::BORDER_FOCUSED
                    } else {
                        Theme::BORDER
                    };

                    let title_style = if app.focused_pane == FocusedPane::Detail {
                        Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
                    } else {
                        Theme::title()
                    };

                    let clean_name = name.trim_end_matches('/');
                    let block = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                        .padding(Padding::new(2, 2, 1, 1))
                        .title(Span::styled(format!(" Directory: {}/ ", clean_name), title_style));

                    let matching_docs: Vec<&crate::domain::Document> = app.documents.iter().filter(|d| {
                        let f = std::path::Path::new(&d.path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                        let clean = if f.is_empty() { "general" } else { &f };
                        clean == path
                    }).collect();

                    let mut text = vec![
                        Line::from(vec![
                            Span::styled("Directory: ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("{}/", clean_name), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                            Span::raw("    "),
                            Span::styled(format!("({} documents)", doc_count), Style::default().fg(Color::Cyan)),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled("────── Documents in this Directory ──────────────────────────", Style::default().fg(Theme::BORDER))),
                        Line::from(""),
                    ];

                    for doc in matching_docs {
                        let badge = match doc.status.as_str() {
                            "accepted" => ("● ACCEPTED", Theme::badge_accepted()),
                            "proposed" => ("○ PROPOSED", Theme::badge_proposed()),
                            "open" => ("▲ OPEN", Theme::badge_risk()),
                            "acknowledged" => ("✔ ACKNOWLEDGED", Theme::badge_acknowledged()),
                            _ => ("· DOC", Style::default().fg(Theme::STATUS_UNKNOWN)),
                        };
                        text.push(Line::from(vec![
                            Span::styled(badge.0, badge.1),
                            Span::raw("  "),
                            Span::styled(&doc.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        ]));
                        text.push(Line::from(vec![
                            Span::raw("   "),
                            Span::styled(&doc.path, Style::default().fg(Theme::TEXT_MUTED)),
                        ]));
                        text.push(Line::from(""));
                    }

                    text.push(Line::from(Span::styled("────────────────────────────────────────────────────────────", Style::default().fg(Theme::BORDER))));
                    text.push(Line::from(vec![
                        Span::raw("Press "),
                        Span::styled("[Enter]", Style::default().fg(Theme::ACCENT)),
                        Span::raw(" or "),
                        Span::styled("[Space]", Style::default().fg(Theme::ACCENT)),
                        Span::raw(if is_collapsed { " to expand this folder" } else { " to collapse this folder" }),
                        Span::raw(", or "),
                        Span::styled("[t]", Style::default().fg(Theme::ACCENT)),
                        Span::raw(" to switch back to List view."),
                    ]));

                    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
                    frame.render_widget(p, area);
                }
            }
        } else {
            Self::render_document_preview(frame, app, area);
        }
    }
}
