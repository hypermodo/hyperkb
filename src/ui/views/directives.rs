use crate::ui::app::{App, FocusedPane};
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub struct DirectivesView;

impl DirectivesView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let list_width = (area.width * 38 / 100).clamp(36, 68);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Min(40)])
            .split(area);

        Self::render_directives_list(frame, app, chunks[0]);
        Self::render_directive_preview(frame, app, chunks[1]);
    }

    fn render_directives_list(frame: &mut Frame, app: &App, area: Rect) {
        let border_color = if app.focused_pane == FocusedPane::List {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER
        };

        let items: Vec<ListItem> = app
            .directives
            .iter()
            .enumerate()
            .map(|(idx, dir)| {
                let is_selected = idx == app.selected_directive_idx;

                let (badge_text, badge_style) = match dir.status.as_str() {
                    "active" => ("● ACTIVE ", Theme::badge_accepted()),
                    "dormant" => ("○ DORMANT ", Theme::badge_proposed()),
                    "retired" => ("✕ RETIRED ", Style::default().fg(Theme::STATUS_SUPERSEDED)),
                    "superseded" => ("✕ SUPERSEDED ", Style::default().fg(Theme::STATUS_SUPERSEDED)),
                    _ => ("· DIR ", Style::default().fg(Theme::STATUS_UNKNOWN)),
                };

                let title = Span::styled(
                    &dir.title,
                    if is_selected {
                        Theme::selected_row()
                    } else {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    },
                );

                let cat_badge = Span::styled(
                    format!("[{}] ", dir.category.to_uppercase()),
                    Style::default().fg(Color::Cyan),
                );

                let scope_text = if dir.is_global() {
                    "global".to_string()
                } else {
                    dir.scope.join(", ")
                };

                let sub_info = Span::styled(
                    format!("   scope: {}  |  enforce: {}", scope_text, dir.enforcement),
                    Style::default().fg(Theme::TEXT_MUTED),
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

        let list_title = format!(" Directives ({}) ", app.directives.len());
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(list_title, Theme::title()));

        if items.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No directives found for this category.",
                    Style::default().fg(Theme::TEXT_MUTED),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  Use "),
                    Span::styled("hyperkb directive new", Style::default().fg(Theme::ACCENT)),
                    Span::raw(" to draft one."),
                ]),
            ];
            let p = Paragraph::new(empty_text).block(list_block);
            frame.render_widget(p, area);
        } else {
            let list = List::new(items).block(list_block);
            frame.render_widget(list, area);
        }
    }

    fn render_directive_preview(frame: &mut Frame, app: &App, area: Rect) {
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
            .title(Span::styled(" Directive Card [Tab: Switch Pane, Enter: Reader, r: Retire] ", title_style));

        if let Some(dir) = app.selected_directive() {
            let mut text = vec![
                Line::from(""), // Top breathing room
                Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(&dir.title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(format!("[{}]", dir.id), Style::default().fg(Theme::TEXT_MUTED)),
                ]),
                Line::from(vec![
                    Span::styled("  Category: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(format!("[{}]", dir.category.to_uppercase()), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw("   "),
                    Span::styled("Status: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(
                        format!("[{}]", dir.status.to_uppercase()),
                        if dir.status == "active" {
                            Style::default().fg(Theme::STATUS_ACCEPTED).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Yellow)
                        },
                    ),
                    Span::raw("   "),
                    Span::styled("Enforcement: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&dir.enforcement, Style::default().fg(Color::LightBlue)),
                ]),
                Line::from(vec![
                    Span::styled("  Author: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&dir.author, Style::default().fg(Color::White)),
                    Span::raw("   "),
                    Span::styled("Scope: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(
                        if dir.is_global() { "global (*)".to_string() } else { dir.scope.join(", ") },
                        Style::default().fg(Color::LightYellow),
                    ),
                    Span::raw("   "),
                    Span::styled("Created: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&dir.created_at, Style::default().fg(Theme::TEXT_MUTED)),
                ]),
            ];

            if let Some(ref sup) = dir.supersedes {
                text.push(Line::from(vec![
                    Span::styled("  Supersedes: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(sup, Style::default().fg(Theme::STATUS_SUPERSEDED)),
                ]));
            }

            if dir.status == "retired" {
                text.push(Line::from(""));
                text.push(Line::from(Span::styled(
                    "  ⚠️  NOTICE: This directive is RETIRED and no longer active in briefings or pre-commit checks.",
                    Style::default().fg(Theme::STATUS_RISK_OPEN).add_modifier(Modifier::BOLD),
                )));
            }

            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "  ──────────────────────────────────────────────────────────",
                Style::default().fg(Theme::BORDER),
            )));
            text.push(Line::from(""));

            let preview_width = area.width.saturating_sub(4) as usize;
            let formatted_body = crate::ui::markdown::MarkdownFormatter::format_markdown(&dir.content, preview_width);
            text.extend(formatted_body);

            let paragraph = Paragraph::new(text)
                .block(block)
                .scroll((app.directive_preview_scroll as u16, 0))
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        } else {
            let paragraph = Paragraph::new("No directive selected.")
                .block(block)
                .style(Style::default().fg(Theme::TEXT_MUTED));
            frame.render_widget(paragraph, area);
        }
    }
}
