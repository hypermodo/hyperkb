use crate::ui::app::{App, FocusedPane};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

pub struct WorkView;

impl WorkView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        if app.active_risks.is_empty() {
            Self::render_empty_state(frame, app, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);

        Self::render_risk_list(frame, app, chunks[0]);
        Self::render_risk_detail(frame, app, chunks[1]);
    }

    fn render_empty_state(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border()))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" Work & Active Context ", t.title()));

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "● Working Tree Clean & Verified",
                Style::default()
                    .fg(t.status_accepted())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "No active risks or known regressions match your current work tree.",
                Style::default().fg(t.text_muted()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(t.text_primary())),
                Span::styled("[2]", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled(" to explore team decisions, or ", Style::default().fg(t.text_primary())),
                Span::styled("[/]", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled(" to search knowledge.", Style::default().fg(t.text_primary())),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    fn render_risk_list(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let border_color = if app.focused_pane == FocusedPane::List {
            t.border_focused()
        } else {
            t.border()
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
                        t.selected_row()
                    } else {
                        Style::default().fg(t.text_primary())
                    },
                );

                let badge = Span::styled("▲ OPEN RISK ", t.badge_risk());
                let path_info = Span::styled(
                    format!("  {} paths affected", risk.matched_paths.len()),
                    Style::default().fg(t.text_muted()),
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
                    .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
                    .padding(Padding::new(2, 2, 1, 1))
                    .title(Span::styled(" Active Risks ", t.title())),
            )
            .highlight_style(t.selected_row());

        frame.render_widget(list, area);
    }

    fn render_risk_detail(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
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
            .title(Span::styled(" Risk Context & Evidence ", t.title()));

        if let Some(risk) = app.selected_risk() {
            let mut text = vec![
                Line::from(vec![
                    Span::styled("Title:          ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(&risk.document.title, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Source:         ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(&risk.document.path, Style::default().fg(t.accent())),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Applicability:  ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        risk.applicability.as_str(),
                        Style::default().fg(t.status_proposed()),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "────── Matched File Paths ─────────────────────────────────",
                    Style::default().fg(t.border()),
                )),
                Line::from(""),
            ];

            for path in &risk.matched_paths {
                text.push(Line::from(vec![
                    Span::styled("• ", Style::default().fg(t.accent())),
                    Span::styled(path, Style::default().fg(t.text_primary())),
                ]));
                text.push(Line::from(""));
            }

            text.push(Line::from(Span::styled(
                "────── Document Rationale / Evidence ─────────────────────",
                Style::default().fg(t.border()),
            )));
            text.push(Line::from(""));
            for line in risk.document.content.lines().take(20) {
                text.push(Line::from(Span::styled(line.to_string(), Style::default().fg(t.text_primary()))));
            }

            let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        } else {
            let empty_preview = vec![
                Line::from(""),
                Line::from(Span::styled("Select a risk to inspect details.", Style::default().fg(t.text_muted()))),
            ];
            let paragraph = Paragraph::new(empty_preview)
                .block(block);
            frame.render_widget(paragraph, area);
        }
    }
}
