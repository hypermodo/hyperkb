use crate::ui::app::App;
use crate::ui::markdown::MarkdownFormatter;
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct ReaderView;

impl ReaderView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        if let Some(ref doc) = app.current_document {
            if app.show_raw {
                Self::render_raw(frame, app, area, doc);
            } else {
                Self::render_formatted(frame, app, area, doc);
            }
        } else {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::BORDER))
                .title(Span::styled(" Reader ", Theme::title()));

            let p = Paragraph::new("No document selected. Press [2] to browse and Enter to open.")
                .block(block)
                .style(Style::default().fg(Theme::TEXT_MUTED));
            frame.render_widget(p, area);
        }
    }

    fn render_formatted(frame: &mut Frame, app: &App, area: Rect, doc: &crate::domain::Document) {
        // Vertical split: Metadata Card (5 rows) + Formatted Document Body (Remaining)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(8)])
            .split(area);

        // 1. Metadata Card Header
        let meta_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::BORDER))
            .title(Span::styled(format!(" {} ", doc.title), Theme::title()));

        let meta_lines = MarkdownFormatter::format_metadata_card(doc);
        let meta_widget = Paragraph::new(meta_lines).block(meta_block);
        frame.render_widget(meta_widget, chunks[0]);

        // 2. Formatted Markdown Body
        let body_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::BORDER_FOCUSED))
            .title(Span::styled(
                " Document Body [Press 'v' for Raw, Esc to Exit] ",
                Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD),
            ));

        let reader_width = chunks[1].width.saturating_sub(4) as usize;
        let formatted_lines = MarkdownFormatter::format_markdown(&doc.content, reader_width);
        let body_widget = Paragraph::new(formatted_lines)
            .block(body_block)
            .scroll((app.reader_scroll_offset as u16, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(body_widget, chunks[1]);
    }

    fn render_raw(frame: &mut Frame, app: &App, area: Rect, doc: &crate::domain::Document) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(
                " RAW Document View [Press 'v' to return to Formatted] ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));

        let lines: Vec<Line> = doc
            .content
            .lines()
            .map(|l| Line::from(format!("  {}", l)))
            .collect();

        let widget = Paragraph::new(lines)
            .block(block)
            .scroll((app.reader_scroll_offset as u16, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(widget, area);
    }
}
