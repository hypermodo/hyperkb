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
        let input_lines = app.repl_input.lines().count().max(1);
        let prompt_h = (input_lines as u16 + 8).clamp(10, 16);
        let filtered_slash = app.filtered_slash_commands();

        if app.repl_active && !filtered_slash.is_empty() {
            let slash_h = (filtered_slash.len() as u16 + 2).min(7);
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(6),
                    Constraint::Length(slash_h),
                    Constraint::Length(prompt_h),
                ])
                .split(area);

            Self::render_diagnostic_stream(frame, app, chunks[0]);
            Self::render_slash_menu(frame, app, &filtered_slash, chunks[1]);
            Self::render_command_input(frame, app, chunks[2]);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(6), Constraint::Length(prompt_h)])
                .split(area);

            Self::render_diagnostic_stream(frame, app, chunks[0]);
            Self::render_command_input(frame, app, chunks[1]);
        }
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
                " Terminal Output & Audit Stream [o: Open Flagged | ↑↓: History/Scroll | c: Clear] ",
                t.title(),
            ));

        if app.diagnostic_stream.is_empty() {
            let empty = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "● Output stream is empty.",
                    Style::default().fg(t.text_muted()),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Type ", Style::default().fg(t.text_primary())),
                    Span::styled("/", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled(" for Command Palette, or enter ", Style::default().fg(t.text_primary())),
                    Span::styled("audit", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(" / ", Style::default().fg(t.text_primary())),
                    Span::styled("check", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(" in the prompt below.", Style::default().fg(t.text_primary())),
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

    fn render_slash_menu(
        frame: &mut Frame,
        app: &App,
        filtered: &[&'static crate::ui::app::SlashCommand],
        area: Rect,
    ) {
        let t = &app.theme;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(1, 1, 0, 0))
            .title(Span::styled(" Command Palette ", t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [↑↓] ", t.key_badge()),
                Span::styled("Navigate • ", Style::default().fg(t.text_muted())),
                Span::styled("[Tab] ", t.key_badge()),
                Span::styled("Autocomplete • ", Style::default().fg(t.text_muted())),
                Span::styled("[Enter] ", t.key_badge()),
                Span::styled("Run • ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] ", t.key_badge()),
                Span::styled("Dismiss ", Style::default().fg(t.text_muted())),
            ]));

        let sel_idx = app.slash_menu_selected_idx.min(filtered.len().saturating_sub(1));
        let mut lines = Vec::new();

        for (idx, cmd) in filtered.iter().enumerate() {
            let is_sel = idx == sel_idx;
            if is_sel {
                lines.push(Line::from(vec![
                    Span::styled(" ▶ /", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{:<12}", cmd.name), t.selected_row()),
                    Span::styled(format!("  {}", cmd.description), Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("   /", Style::default().fg(t.text_muted())),
                    Span::styled(format!("{:<12}", cmd.name), Style::default().fg(t.status_proposed())),
                    Span::styled(format!("  {}", cmd.description), Style::default().fg(t.text_muted())),
                ]));
            }
        }

        let p = Paragraph::new(lines).block(block);
        frame.render_widget(p, area);
    }

    fn render_command_input(frame: &mut Frame, app: &App, area: Rect) {
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
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(
                " Command Input ",
                if app.repl_active { t.title() } else { Style::default().fg(t.text_muted()) },
            ))
            .title_bottom(Line::from(vec![
                Span::styled(" [Enter] ", if app.repl_active { t.key_badge() } else { Style::default().fg(t.text_muted()) }),
                Span::styled("Run  ", Style::default().fg(t.text_muted())),
                Span::styled("[Shift+Enter / Option+Enter] ", if app.repl_active { t.key_badge() } else { Style::default().fg(t.text_muted()) }),
                Span::styled("Newline  ", Style::default().fg(t.text_muted())),
                Span::styled("[/] ", if app.repl_active { Span::styled("/", t.badge_accepted()).style } else { Style::default().fg(t.text_muted()) }),
                Span::styled("Palette  ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] ", if app.repl_active { t.key_badge() } else { Style::default().fg(t.text_muted()) }),
                Span::styled("Unfocus ", Style::default().fg(t.text_muted())),
            ]));

        let mut lines = Vec::new();

        if app.repl_input.is_empty() {
            if app.repl_active {
                lines.push(Line::from(vec![
                    Span::styled("> ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled("▌", Style::default().fg(t.accent())),
                    Span::styled(
                        " Type a command or '/' for palette (e.g. /audit, /check, /reindex, /help)...",
                        Style::default().fg(t.text_muted()),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("> ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        "Click or press '/' to enter commands (audit, check, reindex, help)...",
                        Style::default().fg(t.text_muted()),
                    ),
                ]));
            }
        } else {
            let input_lines: Vec<&str> = app.repl_input.split('\n').collect();
            let show_line_numbers = input_lines.len() > 1;
            for (idx, line_str) in input_lines.iter().enumerate() {
                let is_last = idx == input_lines.len() - 1;
                let prefix = if show_line_numbers {
                    format!("{:>2} │ ", idx + 1)
                } else {
                    "> ".to_string()
                };
                let mut spans = vec![
                    Span::styled(
                        prefix,
                        if show_line_numbers {
                            Style::default().fg(t.text_muted())
                        } else {
                            Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
                        },
                    ),
                    Span::styled(*line_str, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                ];
                if is_last && app.repl_active {
                    spans.push(Span::styled("▌", Style::default().fg(t.accent())));
                }
                lines.push(Line::from(spans));
            }
        }

        let p = Paragraph::new(lines).block(block);
        frame.render_widget(p, area);
    }
}
