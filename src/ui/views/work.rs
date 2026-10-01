use crate::ui::app::{App, FocusedPane};
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

pub struct WorkView;

impl WorkView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        if app.active_risks.is_empty() {
            Self::render_empty_state(frame, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);

        Self::render_risk_list(frame, app, chunks[0]);
        Self::render_risk_detail(frame, app, chunks[1]);
    }

    fn render_empty_state(frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::BORDER))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" Work & Active Context ", Theme::title()));

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "● Working Tree Clean & Verified",
                Style::default()
                    .fg(Theme::STATUS_ACCEPTED)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "No active risks or known regressions match your current work tree.",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("[2]", Style::default().fg(Theme::ACCENT)),
                Span::raw(" to explore team decisions, or "),
                Span::styled("[/]", Style::default().fg(Theme::ACCENT)),
                Span::raw(" to search knowledge."),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    fn render_risk_list(frame: &mut Frame, app: &App, area: Rect) {
        let border_color = if app.focused_pane == FocusedPane::List {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER
        };

        let items: Vec<ListItem> = app
            .active_risks
            .iter()
            .enumerate()
            .map(|(idx, risk)| {
                let is_selected = idx == app.selected_risk_idx;
                let title = Span::styled(
                    &risk.document.title,
                    if is_selected {
                        Theme::selected_row()
                    } else {
                        Style::default().fg(Color::White)
                    },
                );

                let badge = Span::styled("▲ OPEN RISK ", Theme::badge_risk());
                let path_info = Span::styled(
                    format!("  {} paths affected", risk.matched_paths.len()),
                    Style::default().fg(Theme::TEXT_MUTED),
                );

                ListItem::new(vec![
                    Line::from(vec![badge, title]),
                    Line::from(path_info),
                    Line::from(""),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .padding(Padding::new(2, 2, 1, 1))
                    .title(Span::styled(" Active Risks ", Theme::title())),
            )
            .highlight_style(Theme::selected_row());

        frame.render_widget(list, area);
    }

    fn render_risk_detail(frame: &mut Frame, app: &App, area: Rect) {
        let border_color = if app.focused_pane == FocusedPane::Detail {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" Risk Context & Evidence ", Theme::title()));

        if let Some(risk) = app.selected_risk() {
            let mut text = vec![
                Line::from(vec![
                    Span::styled("Title:          ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                    Span::styled(&risk.document.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Source:         ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                    Span::styled(&risk.document.path, Style::default().fg(Theme::ACCENT)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Applicability:  ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        risk.applicability.as_str(),
                        Style::default().fg(Theme::STATUS_PROPOSED),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "────── Matched File Paths ─────────────────────────────────",
                    Style::default().fg(Theme::BORDER),
                )),
                Line::from(""),
            ];

            for path in &risk.matched_paths {
                text.push(Line::from(vec![
                    Span::styled("• ", Style::default().fg(Color::Cyan)),
                    Span::styled(path, Style::default().fg(Color::White)),
                ]));
                text.push(Line::from(""));
            }

            text.push(Line::from(Span::styled(
                "────── Document Rationale / Evidence ─────────────────────",
                Style::default().fg(Theme::BORDER),
            )));
            text.push(Line::from(""));
            for line in risk.document.content.lines().take(20) {
                text.push(Line::from(line.to_string()));
            }

            let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        } else {
            let empty_preview = vec![
                Line::from(""),
                Line::from(Span::styled("Select a risk to inspect details.", Style::default().fg(Theme::TEXT_MUTED))),
            ];
            let paragraph = Paragraph::new(empty_preview)
                .block(block);
            frame.render_widget(paragraph, area);
        }
    }
}
