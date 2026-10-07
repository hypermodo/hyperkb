use crate::ui::app::{App, FocusedPane};
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

pub struct DirectivesView;

impl DirectivesView {
    pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
        let list_width = (area.width * 38 / 100).clamp(36, 68);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Min(40)])
            .split(area);

        Self::render_directives_list(frame, app, chunks[0]);
        Self::render_directive_preview(frame, app, chunks[1]);
    }

    fn render_directives_list(frame: &mut Frame, app: &mut App, area: Rect) {
        let t = app.theme;
        let border_color = if app.focused_pane == FocusedPane::List {
            t.border_focused()
        } else {
            t.border()
        };

        let items: Vec<ListItem> = app
            .directives
            .iter()
            .enumerate()
            .map(|(idx, dir)| {
                let is_selected = idx == app.selected_directive_idx;

                let (badge_text, badge_style) = match dir.status.as_str() {
                    "active" => ("● ACTIVE ", t.badge_accepted()),
                    "dormant" => ("○ DORMANT ", t.badge_proposed()),
                    "retired" => ("✕ RETIRED ", Style::default().fg(t.status_superseded())),
                    "superseded" => ("✕ SUPERSEDED ", Style::default().fg(t.status_superseded())),
                    _ => ("· DIR ", Style::default().fg(t.status_unknown())),
                };

                let title = Span::styled(
                    &dir.title,
                    if is_selected {
                        t.selected_row()
                    } else {
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                    },
                );

                let cat_badge = Span::styled(
                    format!("[{}] ", dir.category.to_uppercase()),
                    Style::default().fg(t.accent()),
                );

                let scope_text = if dir.is_global() {
                    "global".to_string()
                } else {
                    dir.scope.join(", ")
                };

                let sub_info = Span::styled(
                    format!("   scope: {}  |  enforce: {}", scope_text, dir.enforcement),
                    Style::default().fg(t.text_muted()),
                );

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(badge_text, badge_style),
                        cat_badge,
                        title,
                    ]),
                    Line::from(sub_info),
                    Line::from(""),
                ])
            })
            .collect();

        let list_title = format!(" Directives ({}) [Cat: {}] ", app.directives.len(), app.directive_category.to_uppercase());
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(list_title, t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [n] ", t.key_badge()),
                Span::styled("New • ", Style::default().fg(t.text_muted())),
                Span::styled("[r] ", t.key_badge()),
                Span::styled("Toggle Active/Retire • ", Style::default().fg(t.text_muted())),
                Span::styled("[c] ", t.key_badge()),
                Span::styled("Cat", Style::default().fg(t.text_muted())),
            ]));

        if items.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No directives found for this category.",
                    Style::default().fg(t.text_muted()),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(t.text_primary())),
                    Span::styled("[n]", t.key_badge()),
                    Span::styled(" to draft a new directive instantly.", Style::default().fg(t.text_primary())),
                ]),
            ];
            let p = Paragraph::new(empty_text).block(list_block);
            frame.render_widget(p, area);
        } else {
            let list = List::new(items).block(list_block);
            frame.render_stateful_widget(list, area, &mut app.directives_list_state);

            let total_dirs = app.directives.len();
            if total_dirs > 0 {
                let mut scrollbar_state = ScrollbarState::new(total_dirs.saturating_sub(1))
                    .position(app.selected_directive_idx);
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

    fn render_directive_preview(frame: &mut Frame, app: &App, area: Rect) {
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
            .title(Span::styled(" Directive Card [TUI: Governance | IDE: Prose ([o])] ", t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [o] ", t.key_badge()),
                Span::styled("Open in Editor • ", Style::default().fg(t.text_muted())),
                Span::styled("[y] ", t.key_badge()),
                Span::styled("Copy Markdown • ", Style::default().fg(t.text_muted())),
                Span::styled("[r] ", t.key_badge()),
                Span::styled("Toggle Active/Retired", Style::default().fg(t.text_muted())),
            ]));

        if let Some(dir) = app.selected_directive() {
            let mut text = vec![
                Line::from(vec![
                    Span::styled(&dir.title, Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::raw("   "),
                    Span::styled(format!("[{}]", dir.id), Style::default().fg(t.text_muted())),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Category: ", Style::default().fg(t.text_muted())),
                    Span::styled(format!("[{}]", dir.category.to_uppercase()), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::raw("    "),
                    Span::styled("Status: ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        format!("[{}]", dir.status.to_uppercase()),
                        if dir.status == "active" {
                            Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(t.status_proposed())
                        },
                    ),
                    Span::raw("    "),
                    Span::styled("Enforcement: ", Style::default().fg(t.text_muted())),
                    Span::styled(&dir.enforcement, Style::default().fg(t.accent())),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Author: ", Style::default().fg(t.text_muted())),
                    Span::styled(&dir.author, Style::default().fg(t.text_primary())),
                    Span::raw("    "),
                    Span::styled("Scope: ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        if dir.is_global() { "global (*)".to_string() } else { dir.scope.join(", ") },
                        Style::default().fg(t.status_proposed()),
                    ),
                    Span::raw("    "),
                    Span::styled("Created: ", Style::default().fg(t.text_muted())),
                    Span::styled(&dir.created_at, Style::default().fg(t.text_muted())),
                ]),
            ];

            if let Some(ref sup) = dir.supersedes {
                text.push(Line::from(""));
                text.push(Line::from(vec![
                    Span::styled("Supersedes: ", Style::default().fg(t.text_muted())),
                    Span::styled(sup, Style::default().fg(t.status_superseded())),
                ]));
            }

            if dir.status == "retired" {
                text.push(Line::from(""));
                text.push(Line::from(Span::styled(
                    "[NOTICE] This directive is RETIRED and no longer active in briefings or pre-commit checks.",
                    Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD),
                )));
            }

            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "────────────────────────────────────────────────────────────",
                Style::default().fg(t.border()),
            )));
            text.push(Line::from(""));

            let preview_width = area.width.saturating_sub(6) as usize;
            let formatted_body = crate::ui::markdown::MarkdownFormatter::format_markdown_with_theme(&dir.content, preview_width, &t);
            text.extend(formatted_body);

            let total_lines = text.len();
            let visible_lines = area.height.saturating_sub(4) as usize;
            let max_scroll = total_lines.saturating_sub(visible_lines);
            let scroll = app.directive_preview_scroll.min(max_scroll);

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
            let empty_preview = vec![
                Line::from(""),
                Line::from(Span::styled("No directive selected.", Style::default().fg(t.text_muted()))),
                Line::from(""),
                Line::from(Span::styled("Select a directive from the left list to view its policy rules and context.", Style::default().fg(t.text_muted()))),
            ];
            let paragraph = Paragraph::new(empty_preview)
                .block(block);
            frame.render_widget(paragraph, area);
        }
    }
}
