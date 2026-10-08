use crate::domain::{DocumentKind, DocumentStatus};
use crate::ui::app::{App, FocusedPane, WorkTabMode};
use crate::ui::markdown::MarkdownFormatter;
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

pub struct WorkView;

impl WorkView {
    pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
        match app.work_tab_mode {
            WorkTabMode::Projects => Self::render_projects_view(frame, app, area),
            WorkTabMode::Risks => Self::render_risks_view(frame, app, area),
            WorkTabMode::Console => Self::render_diagnostic_stream(frame, app, area),
        }
    }

    fn render_projects_view(frame: &mut Frame, app: &mut App, area: Rect) {
        if app.projects.is_empty() {
            Self::render_empty_projects_state(frame, app, area);
            return;
        }

        let list_width = app.list_width(area.width);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Min(40)])
            .split(area);

        Self::render_projects_list(frame, app, chunks[0]);
        Self::render_project_cockpit(frame, app, chunks[1]);
    }

    fn render_empty_projects_state(frame: &mut Frame, app: &App, area: Rect) {
        let t = app.theme;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border()))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" Projects Cockpit ", t.title()));

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "● No Subprojects Detected",
                Style::default()
                    .fg(t.status_proposed())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "HyperKB indexes and aggregates tasks, decisions, and statuses for directories under 'projects/<name>/'.",
                Style::default().fg(t.text_muted()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Run ", Style::default().fg(t.text_primary())),
                Span::styled("hyperkb index", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled(" or switch view: ", Style::default().fg(t.text_primary())),
                Span::styled("[w] Risks", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled(" • ", Style::default().fg(t.border())),
                Span::styled("[c] Logs", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    fn render_projects_list(frame: &mut Frame, app: &mut App, area: Rect) {
        let t = app.theme;
        let border_color = if app.focused_pane == FocusedPane::List {
            t.border_focused()
        } else {
            t.border()
        };

        let items: Vec<ListItem> = app
            .projects
            .iter()
            .enumerate()
            .map(|(idx, proj)| {
                let is_selected = idx == app.selected_project_idx;

                let health_badge = if proj.tasks_blocked > 0 {
                    Span::styled(format!(" ✖ BLOCKED: {} ", proj.tasks_blocked), t.badge_risk())
                } else if proj.open_risks > 0 {
                    Span::styled(format!(" ▲ RISKS: {} ", proj.open_risks), t.badge_proposed())
                } else if proj.is_healthy() {
                    Span::styled(" ✔ HEALTHY ", t.badge_accepted())
                } else {
                    Span::styled(" ○ ACTIVE ", t.badge_accepted())
                };

                let title = Span::styled(
                    format!(" {}", proj.name),
                    if is_selected {
                        t.selected_row()
                    } else {
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                    },
                );

                let status_pill = if proj.has_status_doc {
                    Span::styled(" • status.md ✓", Style::default().fg(t.status_accepted()))
                } else {
                    Span::styled(" • no status.md", Style::default().fg(t.text_muted()))
                };

                let metrics = Span::styled(
                    format!(
                        "  {} docs ({} active, {} pending)",
                        proj.total_documents, proj.tasks_in_progress, proj.tasks_pending
                    ),
                    Style::default().fg(t.text_muted()),
                );

                let mut header_spans = vec![health_badge];
                if proj.churn_warning {
                    header_spans.push(Span::styled(" [▲ CHURN] ", t.badge_risk()));
                }
                header_spans.push(title);

                ListItem::new(vec![
                    Line::from(header_spans),
                    Line::from(vec![metrics, status_pill]),
                    Line::from(""),
                ])
            })
            .collect();

        let list_title = format!(" Projects Cockpit ({}) [Tab: Pane] ", app.projects.len());
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
                    .padding(Padding::new(1, 1, 1, 1))
                    .title(Span::styled(list_title, t.title())),
            )
            .highlight_style(t.selected_row());

        frame.render_stateful_widget(list, area, &mut app.projects_list_state);

        let total_projects = app.projects.len();
        if total_projects > 0 {
            let mut scrollbar_state = ScrollbarState::new(total_projects.saturating_sub(1))
                .position(app.selected_project_idx);
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn render_project_cockpit(frame: &mut Frame, app: &mut App, area: Rect) {
        let t = app.theme;
        let border_color = if app.focused_pane == FocusedPane::Detail {
            t.border_focused()
        } else {
            t.border()
        };

        let proj = match app.projects.get(app.selected_project_idx) {
            Some(p) => p.clone(),
            None => {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
                    .padding(Padding::new(2, 2, 1, 1))
                    .title(Span::styled(" Project Cockpit ", t.title()));
                let paragraph = Paragraph::new(vec![Line::from("Select a project from the left pane.")])
                    .block(block);
                frame.render_widget(paragraph, area);
                return;
            }
        };

        // Compact overview card on short screens (< 28 lines), expanded on tall screens
        let overview_h = if area.height < 28 { 5 } else { 8 };
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(overview_h), Constraint::Min(8)])
            .split(area);

        // --- 1. Overview Card ---
        let overview_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 0, 0))
            .title(Span::styled(format!(" Project: {} ", proj.name), t.title()));

        let status_doc_desc = if proj.has_status_doc {
            Span::styled("✓ Tracked (status.md)", Style::default().fg(t.status_accepted()))
        } else {
            Span::styled("✗ Missing status.md", Style::default().fg(t.status_proposed()))
        };

        let health_span = match proj.health.as_str() {
            "healthy" => Span::styled(" [✔ HEALTHY] ", t.badge_accepted()),
            "blocked" => Span::styled(" [✖ BLOCKED] ", t.badge_risk()),
            "degraded" => Span::styled(" [▲ DEGRADED] ", t.badge_proposed()),
            _ => Span::styled(format!(" [{}] ", proj.health.to_uppercase()), t.badge_proposed()),
        };

        let mut line1_spans = vec![
            Span::styled("Path: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
            Span::styled(format!("projects/{}    ", proj.name), Style::default().fg(t.accent())),
            Span::styled("Doc: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
            status_doc_desc,
            Span::styled("    Health: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
            health_span,
        ];
        if proj.churn_warning {
            line1_spans.push(Span::styled("  [▲ CHURN WARNING] ", t.badge_risk()));
        }

        let mut overview_text = vec![Line::from(line1_spans)];

        if overview_h >= 8 {
            let mut line2_spans = vec![
                Span::styled("Critical Path: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
            ];
            if let Some(ref lock) = proj.active_task {
                line2_spans.push(Span::styled(format!("🔒 {}    ", lock), Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)));
            } else if proj.tasks_in_progress > 0 {
                line2_spans.push(Span::styled("▶ Auto (in-progress task)    ", Style::default().fg(t.status_accepted())));
            } else {
                line2_spans.push(Span::styled("○ None (unlocked)    ", Style::default().fg(t.text_muted())));
            }

            line2_spans.push(Span::styled("Exit Criteria: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)));
            if let Some(ref ec) = proj.exit_criteria {
                if proj.exit_verified {
                    line2_spans.push(Span::styled(format!("🎯 [✔ VERIFIED] {}", ec), Style::default().fg(t.status_accepted())));
                } else {
                    line2_spans.push(Span::styled(format!("🎯 [○ PENDING] {}", ec), Style::default().fg(t.status_proposed())));
                }
            } else {
                line2_spans.push(Span::styled("○ None declared", Style::default().fg(t.text_muted())));
            }

            overview_text.push(Line::from(line2_spans));
            overview_text.push(Line::from(""));
        }

        if main_chunks[0].width < 110 {
            overview_text.push(Line::from(vec![
                Span::styled(format!("[ Total: {} ]  ", proj.total_documents), Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[ In-Progress: {} ]  ", proj.tasks_in_progress), Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[ Blocked: {} ]  ", proj.tasks_blocked), if proj.tasks_blocked > 0 { Style::default().fg(t.status_superseded()).add_modifier(Modifier::BOLD) } else { Style::default().fg(t.text_muted()) }),
                Span::styled(format!("[ Pending: {} ]", proj.tasks_pending), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
            ]));
            overview_text.push(Line::from(vec![
                Span::styled(format!("[ Done: {} ]  ", proj.tasks_completed), Style::default().fg(t.text_muted())),
                Span::styled(format!("[ Risks: {} ]  ", proj.open_risks), if proj.open_risks > 0 { Style::default().fg(t.status_superseded()) } else { Style::default().fg(t.text_muted()) }),
                Span::styled(format!("[ Decs: {} ]", proj.decisions_count), Style::default().fg(t.accent())),
            ]));
        } else {
            overview_text.push(Line::from(vec![
                Span::styled(format!("[ Total: {} ]  ", proj.total_documents), Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[ In-Progress: {} ]  ", proj.tasks_in_progress), Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[ Blocked: {} ]  ", proj.tasks_blocked), if proj.tasks_blocked > 0 { Style::default().fg(t.status_superseded()).add_modifier(Modifier::BOLD) } else { Style::default().fg(t.text_muted()) }),
                Span::styled(format!("[ Pending: {} ]  ", proj.tasks_pending), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[ Done: {} ]  ", proj.tasks_completed), Style::default().fg(t.text_muted())),
                Span::styled(format!("[ Risks: {} ]  ", proj.open_risks), if proj.open_risks > 0 { Style::default().fg(t.status_superseded()) } else { Style::default().fg(t.text_muted()) }),
                Span::styled(format!("[ Decs: {} ]", proj.decisions_count), Style::default().fg(t.accent())),
            ]));
        }

        let overview_p = Paragraph::new(overview_text).block(overview_block).wrap(Wrap { trim: true });
        frame.render_widget(overview_p, main_chunks[0]);

        // --- 2. Task Funnel & Document Inspector ---
        let funnel_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(main_chunks[1]);

        // Task List
        let task_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(1, 1, 0, 0))
            .title(Span::styled(
                format!(" Tasks & Documents ({}) [Enter: Read | t: Transition | o: IDE] ", app.project_tasks.len()),
                t.title(),
            ));

        if app.project_tasks.is_empty() {
            let empty_p = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("No documents or tasks indexed in projects/{}/", proj.name),
                    Style::default().fg(t.text_muted()),
                )),
            ])
            .block(task_block);
            frame.render_widget(empty_p, funnel_chunks[0]);
        } else {
            let task_items: Vec<ListItem> = app
                .project_tasks
                .iter()
                .enumerate()
                .map(|(idx, doc)| {
                    let is_sel = idx == app.selected_project_task_idx;

                    let badge = match doc.kind {
                        DocumentKind::Task => match doc.status {
                            DocumentStatus::InProgress => Span::styled(" ▶ IN PROGRESS ", t.badge_accepted()),
                            DocumentStatus::Blocked => Span::styled(" ✖ BLOCKED ", t.badge_risk()),
                            DocumentStatus::Pending => Span::styled(" ○ PENDING ", t.badge_proposed()),
                            DocumentStatus::Completed => Span::styled(" ✔ DONE ", t.badge_accepted()),
                            _ => Span::styled(" ⚡ TASK ", t.badge_accepted()),
                        },
                        DocumentKind::Decision => Span::styled(" ⚖ DECISION ", t.badge_accepted()),
                        DocumentKind::Risk => Span::styled(" ▲ HAZARD ", t.badge_risk()),
                        DocumentKind::Spec => Span::styled(" 📄 SPEC/STATUS ", t.badge_proposed()),
                        DocumentKind::Plan => Span::styled(" 📋 PLAN ", t.badge_proposed()),
                        DocumentKind::Audit => Span::styled(" 🔍 AUDIT ", t.badge_accepted()),
                        _ => Span::styled(" 📄 DOC ", t.badge_proposed()),
                    };

                    let title = Span::styled(
                        format!(" {}", doc.title),
                        if is_sel {
                            t.selected_row()
                        } else {
                            Style::default().fg(t.text_primary())
                        },
                    );

                    let path_sub = Span::styled(
                        format!("    {}", doc.path),
                        Style::default().fg(t.text_muted()),
                    );

                    ListItem::new(vec![
                        Line::from(vec![badge, title]),
                        Line::from(path_sub),
                    ])
                })
                .collect();

            let task_list = List::new(task_items)
                .block(task_block)
                .highlight_style(t.selected_row());
            frame.render_stateful_widget(task_list, funnel_chunks[0], &mut app.project_tasks_list_state);

            let total_tasks = app.project_tasks.len();
            if total_tasks > 0 {
                let mut scrollbar_state = ScrollbarState::new(total_tasks.saturating_sub(1))
                    .position(app.selected_project_task_idx);
                let scrollbar = Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");
                frame.render_stateful_widget(scrollbar, funnel_chunks[0], &mut scrollbar_state);
            }
        }

        // Preview snippet below
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border()))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 0, 0))
            .title(Span::styled(" Document Snippet / Summary [Scroll: PgUp/PgDn/Wheel] ", t.title()));

        if let Some(doc) = app.project_tasks.get(app.selected_project_task_idx) {
            let preview_width = funnel_chunks[1].width.saturating_sub(6) as usize;
            let formatted_lines = MarkdownFormatter::format_markdown_with_theme(&doc.content, preview_width, &t);
            let total_lines = formatted_lines.len();
            let visible_lines = funnel_chunks[1].height.saturating_sub(4) as usize;
            let max_scroll = total_lines.saturating_sub(visible_lines);
            let scroll_offset = app.cockpit_preview_scroll.min(max_scroll);

            let mut preview_lines = vec![
                Line::from(vec![
                    Span::styled("File:  ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(&doc.path, Style::default().fg(t.accent())),
                    Span::styled(format!("  ({}/{} lines)", (scroll_offset + 1).min(total_lines), total_lines), Style::default().fg(t.text_muted())),
                ]),
                Line::from(""),
            ];

            for line in formatted_lines.iter().skip(scroll_offset).take(visible_lines) {
                preview_lines.push(line.clone());
            }

            let preview_p = Paragraph::new(preview_lines).block(preview_block).wrap(Wrap { trim: false });
            frame.render_widget(preview_p, funnel_chunks[1]);

            if total_lines > visible_lines {
                let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll_offset);
                let scrollbar = Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");
                frame.render_stateful_widget(scrollbar, funnel_chunks[1], &mut scrollbar_state);
            }
        } else {
            let empty_preview = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("Select a document or task above to preview content.", Style::default().fg(t.text_muted()))),
            ])
            .block(preview_block);
            frame.render_widget(empty_preview, funnel_chunks[1]);
        }
    }

    fn render_risks_view(frame: &mut Frame, app: &mut App, area: Rect) {
        if app.active_risks.is_empty() {
            Self::render_empty_state(frame, app, area);
            return;
        }

        let list_width = app.list_width(area.width);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Min(40)])
            .split(area);

        Self::render_risk_list(frame, app, chunks[0]);
        Self::render_risk_detail(frame, app, chunks[1]);
    }

    fn render_empty_state(frame: &mut Frame, app: &App, area: Rect) {
        let t = app.theme;
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
                Span::styled(" for System Logs, or ", Style::default().fg(t.text_primary())),
                Span::styled("[/]", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                Span::styled(" to ask AI or run commands in the Command Dock below.", Style::default().fg(t.text_primary())),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    fn render_risk_list(frame: &mut Frame, app: &mut App, area: Rect) {
        let t = app.theme;
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

        frame.render_stateful_widget(list, area, &mut app.risks_list_state);

        let total_risks = app.active_risks.len();
        if total_risks > 0 {
            let mut scrollbar_state = ScrollbarState::new(total_risks.saturating_sub(1))
                .position(app.selected_risk_idx);
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn render_risk_detail(frame: &mut Frame, app: &App, area: Rect) {
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
            .title(Span::styled(" Risk Context & Evidence [Scroll: PgUp/PgDn/Wheel] ", t.title()));

        if let Some(risk) = app.selected_risk() {
            let rule_len = (area.width.saturating_sub(6) as usize).max(10);
            let paths_header = if rule_len > 24 {
                format!("────── Matched File Paths {}", "─".repeat(rule_len.saturating_sub(25)))
            } else {
                "─".repeat(rule_len)
            };
            let doc_header = if rule_len > 34 {
                format!("────── Document Rationale / Evidence {}", "─".repeat(rule_len.saturating_sub(35)))
            } else {
                "─".repeat(rule_len)
            };

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
                Line::from(Span::styled(paths_header, Style::default().fg(t.border()))),
                Line::from(""),
            ];

            for path in &risk.matched_paths {
                text.push(Line::from(vec![
                    Span::styled("• ", Style::default().fg(t.accent())),
                    Span::styled(path, Style::default().fg(t.text_primary())),
                ]));
                text.push(Line::from(""));
            }

            text.push(Line::from(Span::styled(doc_header, Style::default().fg(t.border()))));
            text.push(Line::from(""));

            let preview_width = area.width.saturating_sub(6) as usize;
            let formatted_body = MarkdownFormatter::format_markdown_with_theme(&risk.document.content, preview_width, &t);
            text.extend(formatted_body);

            let total_lines = text.len();
            let visible_lines = area.height.saturating_sub(4) as usize;
            let max_scroll = total_lines.saturating_sub(visible_lines);
            let scroll = app.risk_detail_scroll.min(max_scroll);

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
                Line::from(Span::styled("Select a risk to inspect details.", Style::default().fg(t.text_muted()))),
            ];
            let paragraph = Paragraph::new(empty_preview)
                .block(block);
            frame.render_widget(paragraph, area);
        }
    }

    fn render_diagnostic_stream(frame: &mut Frame, app: &App, area: Rect) {
        let t = app.theme;
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

        let rule_len = (area.width.saturating_sub(6) as usize).max(10);
        text.push(Line::from(Span::styled(
            "─".repeat(rule_len),
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
            let flagged_header = if rule_len > 44 {
                format!("────── Actionable Flagged Files [Press o to Open in Editor] {}", "─".repeat(rule_len.saturating_sub(45)))
            } else {
                "── Actionable Flagged Files [o: Open] ──".to_string()
            };
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                flagged_header,
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

        let total_lines = text.len();
        let visible_lines = area.height.saturating_sub(4) as usize;
        let max_scroll = total_lines.saturating_sub(visible_lines);
        let scroll = app.diagnostic_scroll.min(max_scroll);

        let p = Paragraph::new(text)
            .block(block)
            .scroll((scroll as u16, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(p, area);

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
    }
}
