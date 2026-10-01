use crate::ui::app::{App, FocusedPane};
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub struct SessionsView;

impl SessionsView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let list_width = (area.width * 38 / 100).clamp(36, 68);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Min(40)])
            .split(area);

        Self::render_sessions_list(frame, app, chunks[0]);
        Self::render_session_scorecard(frame, app, chunks[1]);
    }

    fn render_sessions_list(frame: &mut Frame, app: &App, area: Rect) {
        let border_color = if app.focused_pane == FocusedPane::List {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER
        };

        let items: Vec<ListItem> = app
            .sessions
            .iter()
            .enumerate()
            .map(|(idx, sess)| {
                let is_selected = idx == app.selected_session_idx;

                let (badge_text, badge_style) = match sess.status.as_str() {
                    "active" => ("● ACTIVE ", Theme::badge_accepted()),
                    "completed" => ("✔ DONE ", Theme::badge_resolved()),
                    "failed" => ("✕ FAILED ", Theme::badge_risk()),
                    _ => ("· SESS ", Style::default().fg(Theme::STATUS_UNKNOWN)),
                };

                let short_id = if sess.id.len() > 18 {
                    format!("{}...", &sess.id[..18])
                } else {
                    sess.id.clone()
                };

                let title = Span::styled(
                    short_id,
                    if is_selected {
                        Theme::selected_row()
                    } else {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    },
                );

                let agent_tag = Span::styled(
                    format!("[{}] ", sess.agent_id),
                    Style::default().fg(Color::Cyan),
                );

                let sub_info = Span::styled(
                    format!(
                        "   dur: {} | edits: {} | tools: {} | diff: {} lines",
                        sess.formatted_duration(), sess.total_edits, sess.total_tool_calls, sess.total_diff_lines
                    ),
                    Style::default().fg(Theme::TEXT_MUTED),
                );

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(badge_text, badge_style),
                        agent_tag,
                        title,
                    ]),
                    Line::from(sub_info),
                    Line::from(""),
                ])
            })
            .collect();

        let list_title = format!(" Agent Sessions ({}) ", app.sessions.len());
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(list_title, Theme::title()));

        if items.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No agent sessions recorded yet.",
                    Style::default().fg(Theme::TEXT_MUTED),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  Sessions are recorded automatically when AI agents"),
                ]),
                Line::from(vec![
                    Span::raw("  interact via the "),
                    Span::styled("HyperKB MCP Server", Style::default().fg(Theme::ACCENT)),
                    Span::raw("."),
                ]),
            ];
            let p = Paragraph::new(empty_text).block(list_block);
            frame.render_widget(p, area);
        } else {
            let list = List::new(items).block(list_block);
            frame.render_widget(list, area);
        }
    }

    fn render_session_scorecard(frame: &mut Frame, app: &App, area: Rect) {
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
            .title(Span::styled(" Session Scorecard & Behavioral Metrics ", title_style));

        if let Some(sess) = app.selected_session() {
            let edit_ratio = if sess.total_edits == 0 {
                sess.total_tool_calls as f64
            } else {
                sess.total_tool_calls as f64 / sess.total_edits as f64
            };

            // Estimate effectiveness baseline
            let mut score = 1.0f64;
            if sess.review_loops > 0 {
                score -= (sess.review_loops as f64 * 0.15).min(0.45);
            }
            if !sess.first_pass_clean {
                score -= 0.15;
            }
            if edit_ratio > 10.0 {
                score -= 0.20;
            }
            score = score.clamp(0.0, 1.0);
            let score_pct = (score * 100.0).round() as u32;

            let score_color = if score_pct >= 80 {
                Theme::STATUS_ACCEPTED
            } else if score_pct >= 50 {
                Theme::STATUS_PROPOSED
            } else {
                Theme::STATUS_RISK_OPEN
            };

            let text = vec![
                Line::from(""), // Top breathing room
                Line::from(vec![
                    Span::styled("  Session ID: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&sess.id, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("  Agent: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&sess.agent_id, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::raw("   |   "),
                    Span::styled("Status: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(
                        &sess.status,
                        if sess.status == "completed" {
                            Theme::STATUS_ACCEPTED
                        } else if sess.status == "active" {
                            Color::Yellow
                        } else {
                            Theme::STATUS_RISK_OPEN
                        },
                    ),
                    Span::raw("   |   "),
                    Span::styled("Duration: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(
                        sess.formatted_duration(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Started: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&sess.started_at, Style::default().fg(Theme::TEXT_MUTED)),
                    Span::raw("   |   "),
                    Span::styled("Ended: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(
                        sess.ended_at.as_deref().unwrap_or("In progress"),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  ────── Performance & Effectiveness ──────",
                    Style::default().fg(Theme::BORDER),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Coding Effectiveness Score: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{}% ", score_pct),
                        Style::default().fg(score_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        if score_pct >= 80 {
                            "(Optimal / Clean Execution)"
                        } else if score_pct >= 50 {
                            "(Moderate Friction / Oscillations Detected)"
                        } else {
                            "(High Thrashing / Poor Effectiveness)"
                        },
                        Style::default().fg(score_color),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Tool-to-Edit Ratio: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{:.1} calls/edit", edit_ratio), Style::default().fg(Color::White)),
                    Span::raw("   "),
                    Span::styled(
                        format!("({} tool calls across {} code edits)", sess.total_tool_calls, sess.total_edits),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Total Diff Volume: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} lines", sess.total_diff_lines), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("  Review Oscillations (Loops): ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{} loops", sess.review_loops),
                        if sess.review_loops > 0 {
                            Style::default().fg(Theme::STATUS_RISK_OPEN).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Theme::STATUS_ACCEPTED)
                        },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  First Pass Clean: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        if sess.first_pass_clean { "YES (Zero Review Cycles)" } else { "NO (Required Iteration)" },
                        if sess.first_pass_clean {
                            Style::default().fg(Theme::STATUS_ACCEPTED).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Yellow)
                        },
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  ────── Risk Interception & Governance ──────",
                    Style::default().fg(Theme::BORDER),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Risks Prevented: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{} prevented", sess.risks_prevented),
                        Style::default().fg(Theme::STATUS_ACCEPTED).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("   |   "),
                    Span::styled("  Risks Cited: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{} cited", sess.risks_cited),
                        if sess.risks_cited > 0 {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Theme::TEXT_MUTED)
                        },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Delegation Grant: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        sess.grant_id.as_deref().unwrap_or("None (Standard agent scope)"),
                        Style::default().fg(Color::LightBlue),
                    ),
                ]),
            ];

            let paragraph = Paragraph::new(text)
                .block(block)
                .scroll((app.session_preview_scroll as u16, 0))
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        } else {
            let paragraph = Paragraph::new("No session selected.")
                .block(block)
                .style(Style::default().fg(Theme::TEXT_MUTED));
            frame.render_widget(paragraph, area);
        }
    }
}
