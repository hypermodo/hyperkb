use crate::domain::schema::TaskState;
use crate::ui::app::{App, FocusedPane, WorkTabMode};
use crate::ui::markdown::MarkdownFormatter;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
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

        let visible_indices = app.visible_project_indices();
        let total_projects = app.projects.len();
        let hidden_count = total_projects.saturating_sub(visible_indices.len());

        let mut items: Vec<ListItem> = visible_indices
            .iter()
            .map(|&idx| {
                let proj = &app.projects[idx];
                let is_selected = idx == app.selected_project_idx;

                let health_badge = if proj.tasks_blocked > 0 {
                    Span::styled(format!(" ✖ BLOCKED: {} ", proj.tasks_blocked), t.badge_risk())
                } else if proj.status == "reopened" {
                    Span::styled(" ▲ REOPENED ", t.badge_proposed().add_modifier(Modifier::BOLD))
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

        if app.projects_filter_active_only && hidden_count > 0 {
            items.push(ListItem::new(vec![
                Line::from(Span::styled(
                    format!("  ▶ {} CLOSED PROJECTS HIDDEN [a: Actions / f: Show All]", hidden_count),
                    Style::default().fg(t.text_muted()).add_modifier(Modifier::ITALIC),
                )),
                Line::from(""),
            ]));
        }

        let list_title = if app.projects_filter_active_only {
            format!(" Projects ({}/{} Active) [←/→: Pane] ", visible_indices.len(), total_projects)
        } else {
            format!(" Projects ({}) [←/→: Pane] ", total_projects)
        };

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

        if !visible_indices.is_empty() {
            let current_pos = visible_indices.iter().position(|&i| i == app.selected_project_idx).unwrap_or(0);
            let mut scrollbar_state = ScrollbarState::new(visible_indices.len().saturating_sub(1))
                .position(current_pos);
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

        let cockpit_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Min(8),
                Constraint::Length(5),
            ])
            .split(area);

        let overview_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 0, 0))
            .title(Span::styled(format!(" Project: {} ", proj.name), t.title()));

        let lifecycle_badge = if proj.status == "reopened" {
            Span::styled(" [▲ REOPENED] ", t.badge_proposed().add_modifier(Modifier::BOLD))
        } else if proj.tasks_blocked > 0 || proj.health == "blocked" {
            Span::styled(" [✖ BLOCKED] ", t.badge_risk())
        } else if proj.is_closed() {
            Span::styled(" [✔ CLOSED] ", t.badge_resolved())
        } else {
            Span::styled(" [✔ HEALTHY] ", t.badge_accepted())
        };

        let exit_badge = if let Some(ref ec) = proj.exit_criteria {
            if proj.exit_verified {
                Span::styled(format!("Exit: [✔ VERIFIED: {}]", ec), Style::default().fg(t.status_accepted()))
            } else {
                Span::styled(format!("Exit: [○ PENDING: {}]", ec), Style::default().fg(t.status_proposed()))
            }
        } else {
            Span::styled("Exit: [None declared]", Style::default().fg(t.text_muted()))
        };

        let line1 = Line::from(vec![
            Span::styled("Path: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
            Span::styled(format!("projects/{}    ", proj.name), Style::default().fg(t.accent())),
            Span::styled("Lifecycle: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
            lifecycle_badge,
            Span::styled("    ", Style::default()),
            exit_badge,
        ]);

        let goal_text = app.project_goal_summary.as_deref().unwrap_or("No goal specified");
        let fence_text = app.project_fence_summary.as_deref().unwrap_or("No boundaries declared");

        let line2 = Line::from(vec![
            Span::styled("Goal:  ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
            Span::styled(goal_text, Style::default().fg(t.text_primary())),
        ]);

        let line3 = Line::from(vec![
            Span::styled("Fence: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
            Span::styled(fence_text, Style::default().fg(t.status_superseded())),
        ]);

        let overview_p = Paragraph::new(vec![line1, line2, line3])
            .block(overview_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(overview_p, cockpit_chunks[0]);

        let handoff_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 0, 0))
            .title(Span::styled(" 📌 LATEST BATON HANDOFF ", t.title()));

        let handoff_lines = if let Some((ref id, ref state, ref next_step, ref blocker)) = app.project_latest_handoff {
            vec![
                Line::from(vec![
                    Span::styled("• Handoff: ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(id, Style::default().fg(t.accent())),
                    Span::styled("    • Blocker: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(blocker, if blocker.contains("None") { Style::default().fg(t.status_accepted()) } else { Style::default().fg(t.status_superseded()) }),
                ]),
                Line::from(vec![
                    Span::styled("• State: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(state, Style::default().fg(t.text_primary())),
                ]),
                Line::from(vec![
                    Span::styled("• Next Step: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(next_step, Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled("• No agent handoffs recorded yet. (Recorded automatically when session ends)", Style::default().fg(t.text_muted())),
                ]),
                Line::from(vec![
                    Span::styled("• Next Step: Queue tactical tasks with [n] or prompt AI agent via [/].", Style::default().fg(t.text_muted())),
                ]),
            ]
        };

        let handoff_p = Paragraph::new(handoff_lines).block(handoff_block).wrap(Wrap { trim: true });
        frame.render_widget(handoff_p, cockpit_chunks[1]);

        let task_title = format!(
            " ⚡ WORK QUEUE ({}) [Space: Transition | Enter: Details | n: New | a: Actions] ",
            app.native_tasks.len()
        );

        let task_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(1, 1, 0, 0))
            .title(Span::styled(task_title, t.title()));

        if !app.native_tasks.is_empty() {
            let task_items: Vec<ListItem> = app
                .native_tasks
                .iter()
                .enumerate()
                .map(|(idx, task)| {
                    let is_sel = idx == app.selected_native_task_idx && app.focused_pane == FocusedPane::Detail;
                    let status_badge = match task.status {
                        TaskState::InProgress => Span::styled(" ▶ ACTIVE ", t.badge_accepted()),
                        TaskState::Blocked => Span::styled(" ✖ BLOCKED ", t.badge_risk()),
                        TaskState::Pending => Span::styled(" ○ PENDING ", t.badge_proposed()),
                        TaskState::Completed => Span::styled(" ✔ DONE ", t.badge_resolved()),
                    };
                    let pri_badge = if task.priority >= 90 {
                        Span::styled(format!(" P: {:>3} ", task.priority), t.badge_risk().add_modifier(Modifier::BOLD))
                    } else if task.priority >= 70 {
                        Span::styled(format!(" P: {:>3} ", task.priority), Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled(format!(" P: {:>3} ", task.priority), Style::default().fg(t.text_muted()))
                    };
                    let title = Span::styled(
                        format!(" {}", task.title),
                        if is_sel {
                            t.selected_row()
                        } else {
                            Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                        },
                    );
                    let short_id = if task.id.len() > 8 { &task.id[..8] } else { &task.id };
                    let desc_snippet = if task.description.len() > 80 { &task.description[..80] } else { &task.description };
                    let subline = Span::styled(
                        format!("    ID: {} • Updated: {} • {}", short_id, &task.updated_at[..10.min(task.updated_at.len())], desc_snippet),
                        Style::default().fg(t.text_muted()),
                    );
                    ListItem::new(vec![
                        Line::from(vec![status_badge, Span::styled(" ", Style::default()), pri_badge, title]),
                        Line::from(subline),
                    ])
                })
                .collect();

            let task_list = List::new(task_items)
                .block(task_block)
                .highlight_style(t.selected_row());
            frame.render_stateful_widget(task_list, cockpit_chunks[2], &mut app.native_tasks_list_state);

            let total_tasks = app.native_tasks.len();
            if total_tasks > 0 {
                let mut scrollbar_state = ScrollbarState::new(total_tasks.saturating_sub(1))
                    .position(app.selected_native_task_idx);
                let scrollbar = Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");
                frame.render_stateful_widget(scrollbar, cockpit_chunks[2], &mut scrollbar_state);
            }
        } else {
            let empty_p = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("⚡ No active native tasks in queue.", Style::default().fg(t.text_muted()))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(t.text_muted())),
                    Span::styled("[n] New Task", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(" or let your AI agent queue work via MCP.", Style::default().fg(t.text_muted())),
                ]),
            ])
            .block(task_block);
            frame.render_widget(empty_p, cockpit_chunks[2]);
        }

        let fence_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 0, 0))
            .title(Span::styled(" 🛡️ CHARTER & OUT-OF-CHARTER FENCE ", t.title()));

        let ec_str = proj.exit_criteria.as_deref().unwrap_or("cargo test");
        let ec_status = if proj.exit_verified { "[✔ PASSED]" } else { "[⏳ PENDING]" };

        let fence_lines = vec![
            Line::from(vec![
                Span::styled("• Goal: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                Span::styled(goal_text, Style::default().fg(t.text_primary())),
            ]),
            Line::from(vec![
                Span::styled("• ⛔ OUT OF CHARTER: ", Style::default().fg(t.status_superseded()).add_modifier(Modifier::BOLD)),
                Span::styled(fence_text, Style::default().fg(t.status_superseded())),
            ]),
            Line::from(vec![
                Span::styled("• Exit Gate: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                Span::styled(ec_str, Style::default().fg(t.accent())),
                Span::styled(format!(" {} [v: Verify Gate]", ec_status), Style::default().fg(t.status_accepted())),
            ]),
        ];

        let fence_p = Paragraph::new(fence_lines).block(fence_block).wrap(Wrap { trim: true });
        frame.render_widget(fence_p, cockpit_chunks[3]);

        if app.show_actions_popover {
            Self::render_actions_popover(frame, app, area);
        }
    }

    fn render_actions_popover(frame: &mut Frame, app: &App, area: Rect) {
        let t = app.theme;
        let popover_w = 42.min(area.width.saturating_sub(4));
        let popover_h = 10.min(area.height.saturating_sub(2));
        let x = area.x + (area.width.saturating_sub(popover_w)) / 2;
        let y = area.y + (area.height.saturating_sub(popover_h)) / 2;
        let popover_area = Rect::new(x, y, popover_w, popover_h);

        frame.render_widget(Clear, popover_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent()))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(1, 1, 0, 0))
            .title(Span::styled(" Task Actions (a) ", t.title()));

        let actions = [
            ("[1] Transition State... (Space)", 0),
            ("[2] Bump Priority", 1),
            ("[3] Run Exit Gate Check", 2),
            ("[4] Open in Editor (o)", 3),
            ("[5] Toggle Filter Active/All", 4),
            ("[6] Delete Task", 5),
        ];

        let items: Vec<ListItem> = actions
            .iter()
            .map(|(label, idx)| {
                let is_sel = *idx == app.actions_popover_idx;
                let style = if is_sel {
                    t.selected_row()
                } else {
                    Style::default().fg(t.text_primary())
                };
                ListItem::new(Line::from(Span::styled(format!(" {} ", label), style)))
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, popover_area);
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
