use crate::ui::app::App;
use crate::ui::markdown::MarkdownFormatter;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

pub struct ReaderView;

impl ReaderView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        if let Some(ref doc) = app.current_document {
            if app.show_raw {
                Self::render_raw(frame, app, area, doc);
            } else {
                Self::render_formatted(frame, app, area, doc);
            }
        } else {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border()))
                .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
                .padding(Padding::new(2, 2, 1, 1))
                .title(Span::styled(" Reader ", t.title()));

            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled("No document selected.", Style::default().fg(t.text_muted()))),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Press "),
                    Span::styled("[2]", Style::default().fg(t.accent())),
                    Span::raw(" to browse documents and "),
                    Span::styled("[Enter]", Style::default().fg(t.accent())),
                    Span::raw(" to open in reader."),
                ]),
            ];
            let p = Paragraph::new(empty_text).block(block);
            frame.render_widget(p, area);
        }
    }

    fn render_formatted(frame: &mut Frame, app: &App, area: Rect, doc: &crate::domain::Document) {
        let t = &app.theme;
        let meta_lines = MarkdownFormatter::format_metadata_card_with_theme(doc, t);
        let meta_height = (meta_lines.len() as u16 + 4).clamp(6, 12);

        // Vertical split: Adaptive Metadata Card + Formatted Document Body (Remaining)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(meta_height), Constraint::Min(8)])
            .split(area);

        // 1. Metadata Card Header
        let meta_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border()))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(format!(" {} ", doc.title), t.title()));

        let meta_widget = Paragraph::new(meta_lines).block(meta_block);
        frame.render_widget(meta_widget, chunks[0]);

        // 2. Formatted Markdown Body
        let body_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border_focused()))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(
                " Document Body [Press 'v' for Raw, Esc to Exit, 'y' to Copy] ",
                t.title(),
            ));

        let reader_width = chunks[1].width.saturating_sub(6) as usize;
        let formatted_lines = MarkdownFormatter::format_markdown_with_theme(&doc.content, reader_width, t);
        let body_widget = Paragraph::new(formatted_lines)
            .block(body_block)
            .scroll((app.reader_scroll_offset as u16, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(body_widget, chunks[1]);
    }

    fn render_raw(frame: &mut Frame, app: &App, area: Rect, doc: &crate::domain::Document) {
        let t = &app.theme;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.status_proposed()))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(
                " RAW Document View [Press 'v' to return to Formatted, 'y' to Copy] ",
                Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD),
            ));

        let lines: Vec<Line> = doc
            .content
            .lines()
            .map(|l| Line::from(Span::styled(l.to_string(), Style::default().fg(t.text_primary()))))
            .collect();

        let widget = Paragraph::new(lines)
            .block(block)
            .scroll((app.reader_scroll_offset as u16, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(widget, area);
    }
}
