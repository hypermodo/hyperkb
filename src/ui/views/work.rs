use crate::ui::app::{App, FocusedPane, WorkTabMode};
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
        match app.work_tab_mode {
            WorkTabMode::Risks => Self::render_risks_view(frame, app, area),
            WorkTabMode::Console => Self::render_console_view(frame, app, area),
        }
    }

    fn render_risks_view(frame: &mut Frame, app: &App, area: Rect) {
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
                Span::styled("[c]", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled(" for the Diagnostic Console & REPL, or ", Style::default().fg(t.text_primary())),
                Span::styled("[Space]", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled(" for the Action Palette.", Style::default().fg(t.text_primary())),
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
                    .title(Span::styled(" Active Risks [w] ", t.title())),
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

    fn render_console_view(frame: &mut Frame, app: &App, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(6), Constraint::Length(3)])
            .split(area);

        Self::render_diagnostic_stream(frame, app, chunks[0]);
        Self::render_command_repl(frame, app, chunks[1]);
    }

    fn render_diagnostic_stream(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let border_color = if !app.repl_active {
            t.border_focused()
        } else {
            t.border()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(
                " Diagnostic Output & Audit Stream [o: Open Flagged | ↑↓: History | c: Clear] ",
                t.title(),
            ));

        if app.diagnostic_stream.is_empty() {
            let empty = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "● Diagnostic stream is empty.",
                    Style::default().fg(t.text_muted()),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Type ", Style::default().fg(t.text_primary())),
                    Span::styled("audit", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(" or ", Style::default().fg(t.text_primary())),
                    Span::styled("check", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(" in prompt below, or press ", Style::default().fg(t.text_primary())),
                    Span::styled("[Space]", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(" for Action Palette.", Style::default().fg(t.text_primary())),
                ]),
            ];
            frame.render_widget(Paragraph::new(empty).block(block), area);
            return;
        }

        let entry_idx = app.selected_diagnostic_idx.min(app.diagnostic_stream.len().saturating_sub(1));
        let entry = &app.diagnostic_stream[entry_idx];

        let mut text = Vec::new();

        // 1. Header Card with Pill, Timestamp, Command, and Status Badge
        let status_badge = if entry.success {
            Span::styled(" ● PASS ", t.badge_accepted())
        } else {
            Span::styled(" ! ISSUES DETECTED ", t.badge_risk())
        };

        text.push(Line::from(vec![
            Span::styled(
                format!("[Entry {} of {}] ", entry_idx + 1, app.diagnostic_stream.len()),
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} ", entry.timestamp.format("%H:%M:%S UTC")),
                Style::default().fg(t.text_muted()),
            ),
            Span::styled(
                format!("▶ {} ", entry.command),
                Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD),
            ),
            status_badge,
        ]));

        text.push(Line::from(vec![
            Span::styled(
                format!("Title: {}", entry.title),
                Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({})", entry.summary),
                Style::default().fg(t.text_muted()),
            ),
        ]));

        text.push(Line::from(Span::styled(
            "─────────────────────────────────────────────────────────────────────────────",
            Style::default().fg(t.border()),
        )));

        // 2. Report Diagnostic Lines
        for line in &entry.lines {
            let styled_span = if line.trim_start().starts_with('✓') {
                Span::styled(line.clone(), Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD))
            } else if line.trim_start().starts_with('!') {
                Span::styled(line.clone(), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD))
            } else if line.trim_start().starts_with('✗') {
                Span::styled(line.clone(), Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD))
            } else if line.trim_start().starts_with('•') {
                Span::styled(line.clone(), Style::default().fg(t.text_primary()))
            } else {
                Span::styled(line.clone(), Style::default().fg(t.text_muted()))
            };
            text.push(Line::from(styled_span));
        }

        // 3. Actionable Flagged Files Section
        if !entry.file_targets.is_empty() {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "────── Actionable Flagged Files [Press o to Open in Editor] ─────────────",
                Style::default().fg(t.accent()),
            )));

            for (f_idx, target) in entry.file_targets.iter().enumerate() {
                let is_sel = f_idx == entry.selected_file_idx;
                if is_sel {
                    text.push(Line::from(vec![
                        Span::styled(" ▶ ", Style::default().fg(t.accent())),
                        Span::styled(target.clone(), t.selected_row()),
                        Span::styled("  [Selected: press 'o' to open in IDE]", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    text.push(Line::from(vec![
                        Span::styled("   • ", Style::default().fg(t.text_muted())),
                        Span::styled(target.clone(), Style::default().fg(t.text_primary())),
                    ]));
                }
            }
        }

        let p = Paragraph::new(text)
            .block(block)
            .scroll((app.diagnostic_scroll as u16, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(p, area);
    }

    fn render_command_repl(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let border_color = if app.repl_active {
            t.accent()
        } else {
            t.border()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if app.repl_active {
                Style::default().fg(border_color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(border_color)
            })
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(1, 1, 0, 0))
            .title(Span::styled(
                if app.repl_active {
                    " Cockpit REPL [Active: type command & press Enter] "
                } else {
                    " Cockpit REPL [Press ':' to focus | Space: Palette] "
                },
                if app.repl_active { t.title() } else { Style::default().fg(t.text_muted()) },
            ));

        let prompt_line = if app.repl_input.is_empty() {
            if app.repl_active {
                vec![
                    Span::styled(" > ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled("▌", Style::default().fg(t.accent())),
                    Span::styled(
                        " (type: audit, check, reindex, directives, grants, harnesses, clear, help...)",
                        Style::default().fg(t.text_muted()),
                    ),
                ]
            } else {
                vec![
                    Span::styled(" > ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        "Press ':' or click to type commands (audit, check, reindex, harnesses, help)...",
                        Style::default().fg(t.text_muted()),
                    ),
                ]
            }
        } else {
            vec![
                Span::styled(" > ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled(&app.repl_input, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                Span::styled("▌", Style::default().fg(t.accent())),
            ]
        };

        let p = Paragraph::new(vec![Line::from(prompt_line)]).block(block);
        frame.render_widget(p, area);
    }
}
