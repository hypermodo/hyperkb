use crate::ui::app::{ActiveTab, App};
use crate::ui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct Header;

impl Header {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let tabs = [
            (ActiveTab::Work, "[1] Work & Risks"),
            (ActiveTab::Explore, "[2] Explore"),
            (ActiveTab::Reader, "[3] Reader"),
        ];

        let mut spans = vec![
            Span::styled(" HyperKB ", Theme::title()),
            Span::styled(format!(" [{}] ", app.collection_id), Style::default().fg(Theme::TEXT_MUTED)),
            Span::raw("    "),
        ];

        for (tab, label) in tabs {
            if app.active_tab == tab {
                spans.push(Span::styled(
                    format!(" {} ", label),
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled(
                    format!(" {} ", label),
                    Style::default().fg(Color::White),
                ));
            }
            spans.push(Span::raw("  "));
        }

        let paragraph = Paragraph::new(Line::from(spans)).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Theme::BORDER)),
        );

        frame.render_widget(paragraph, area);
    }
}

pub struct Footer;

impl Footer {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let text = if app.is_filtering {
            Line::from(vec![
                Span::styled(" Search: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(&app.filter_query),
                Span::styled("█", Style::default().fg(Color::Yellow)),
                Span::styled("  (Press Enter to confirm, Esc to cancel)", Style::default().fg(Theme::TEXT_MUTED)),
            ])
        } else {
            let keys = match app.active_tab {
                ActiveTab::Work => vec![
                    Span::styled("[1-3] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Tabs  "),
                    Span::styled("[Tab] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Switch Pane  "),
                    Span::styled("[↑↓/jk] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Navigate  "),
                    Span::styled("[q] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Quit"),
                ],
                ActiveTab::Explore => vec![
                    Span::styled("[c] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Category  "),
                    Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Read Doc  "),
                    Span::styled("[Tab] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Switch Pane  "),
                    Span::styled("[↑↓/jk] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Navigate  "),
                    Span::styled("[1-3] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Tabs  "),
                    Span::styled("[q] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Quit"),
                ],
                ActiveTab::Reader => vec![
                    Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Back to Explorer  "),
                    Span::styled("[↑↓/jk] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Line  "),
                    Span::styled("[PgDn/PgUp/Space] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Page  "),
                    Span::styled("[v] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Toggle Raw  "),
                    Span::styled("[q] ", Style::default().fg(Theme::ACCENT)),
                    Span::raw("Quit"),
                ],
                ActiveTab::Memory => vec![],
            };
            Line::from(keys)
        };

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Theme::BORDER)),
        );

        frame.render_widget(paragraph, area);
    }
}
