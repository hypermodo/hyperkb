use crate::domain::{DocumentKind, DocumentStatus};
use crate::ui::app::{App, FocusedPane};
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub struct ExploreView;

impl ExploreView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let list_width = (area.width * 36 / 100).clamp(32, 46);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Min(40)])
            .split(area);

        Self::render_document_list(frame, app, chunks[0]);
        Self::render_document_preview(frame, app, chunks[1]);
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
                    DocumentStatus::Superseded => (
                        "✕ SUPERSEDED ",
                        Style::default().fg(Theme::STATUS_SUPERSEDED),
                    ),
                    DocumentStatus::Conflict => {
                        ("! CONFLICT ", Style::default().fg(Color::LightRed))
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

        let title_text = format!(" Knowledge Explorer ({} docs) ", app.documents.len());
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
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
            .title(Span::styled(" Document Preview [Tab to focus, Enter for Reader] ", title_style));

        if let Some(doc) = app.selected_document() {
            let mut text = vec![
                Line::from(vec![
                    Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(&doc.title, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Path: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(&doc.path, Style::default().fg(Theme::ACCENT)),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(doc.status.as_str(), Style::default().fg(Color::Yellow)),
                    Span::raw("   |   "),
                    Span::styled("Kind: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(doc.kind.as_str(), Style::default().fg(Color::LightBlue)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "────── Content Preview ──────",
                    Style::default().fg(Theme::BORDER),
                )),
                Line::from(""),
            ];

            let preview_width = area.width.saturating_sub(4) as usize;
            let formatted_body = crate::ui::markdown::MarkdownFormatter::format_markdown(&doc.content, preview_width);
            text.extend(formatted_body);

            let paragraph = Paragraph::new(text)
                .block(block)
                .scroll((app.preview_scroll_offset as u16, 0))
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        } else {
            let paragraph = Paragraph::new("No documents found in this view.")
                .block(block)
                .style(Style::default().fg(Theme::TEXT_MUTED));
            frame.render_widget(paragraph, area);
        }
    }
}
