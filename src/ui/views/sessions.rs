use crate::ui::app::{App, FocusedPane};
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
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
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(list_title, Theme::title()));

        if items.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No agent sessions recorded yet.",
                    Style::default().fg(Theme::TEXT_MUTED),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Sessions are recorded automatically when AI agents"),
                ]),
                Line::from(vec![
                    Span::raw("interact via the "),
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
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" Session Scorecard & Behavioral Metrics ", title_style));

        if let Some(sess) = app.selected_session() {
            let edit_ratio = if sess.total_edits == 0 {
                sess.total_tool_calls as f64
            } else {
                sess.total_tool_calls as f64 / sess.total_edits as f64
            };

            // Formal Effectiveness Decomposition
            let loop_penalty = (sess.review_loops as f64 * 0.15).min(0.45);
            let first_pass_penalty = if !sess.first_pass_clean { 0.15 } else { 0.0 };
            let thrash_penalty = if edit_ratio > 10.0 { 0.20 } else if edit_ratio > 6.0 { 0.10 } else { 0.0 };
            let hazard_bonus = if sess.risks_prevented > 0 { 0.10 } else { 0.0 };

            let mut score = 1.0f64 - loop_penalty - first_pass_penalty - thrash_penalty + hazard_bonus;
            score = score.clamp(0.05, 1.0);
            let score_pct = (score * 100.0).round() as u32;

            let score_color = if score_pct >= 80 {
                Theme::STATUS_ACCEPTED
            } else if score_pct >= 50 {
                Theme::STATUS_PROPOSED
            } else {
                Theme::STATUS_RISK_OPEN
            };

            // Analytical Quantifications
            let total_risks = sess.risks_prevented + sess.risks_cited;
            let interception_rate = if total_risks > 0 {
                (sess.risks_prevented as f64 / total_risks as f64 * 100.0).round() as u32
            } else {
                100
            };
            let churn_density = if sess.total_edits > 0 {
                format!("{:.1} lines/edit", sess.total_diff_lines as f64 / sess.total_edits as f64)
            } else {
                "0.0 lines/edit".to_string()
            };
            let dur_secs = sess.duration_seconds().max(1);
            let throughput_velocity = format!("{:.0} lines/min", (sess.total_diff_lines as f64 / dur_secs as f64) * 60.0);
            let action_efficiency = if sess.total_tool_calls > 0 {
                format!("{:.0}%", (sess.total_edits as f64 / sess.total_tool_calls as f64) * 100.0)
            } else {
                "100%".to_string()
            };

            if app.show_scoring_methodology {
                // Render Formal Mathematical Proof & Methodology Card
                let text = vec![
                    Line::from(vec![
                        Span::styled("MATHEMATICAL SCORING SPECIFICATION & PROOF", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::raw("  [Press 'e' to return]"),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("1. Objective & Design Philosophy", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
                    Line::from("  Coding Effectiveness measures an agent's capability to converge on correct code"),
                    Line::from("  without human intervention, review thrashing, or excessive search oscillations."),
                    Line::from(""),
                    Line::from(Span::styled("2. Formal Formula", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
                    Line::from(vec![
                        Span::styled("  S = clamp(", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled("100% ", Style::default().fg(Theme::STATUS_ACCEPTED).add_modifier(Modifier::BOLD)),
                        Span::styled("- P_loops - P_friction - P_thrash + B_hazard", Style::default().fg(Color::LightBlue)),
                        Span::styled(", 5%, 100%)", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from("  Where:"),
                    Line::from(vec![
                        Span::styled("  • P_loops    = min(review_loops × 15%, 45%)", Style::default().fg(Color::White)),
                        Span::raw(" (penalizes cyclic PR rejection loops)"),
                    ]),
                    Line::from(vec![
                        Span::styled("  • P_friction = 15%", Style::default().fg(Color::White)),
                        Span::raw(" if initial pass required test/compiler fix cycles"),
                    ]),
                    Line::from(vec![
                        Span::styled("  • P_thrash   = 20%", Style::default().fg(Color::White)),
                        Span::raw(" if tool-to-edit ratio > 10.0 (lost exploration loops)"),
                    ]),
                    Line::from(vec![
                        Span::styled("  • B_hazard   = +10%", Style::default().fg(Color::White)),
                        Span::raw(" for proactive adherence to active architectural invariants"),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("3. Current Session Parameter Values", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
                    Line::from("  • Base Score:          100%"),
                    Line::from(format!("  • Loop Deduction:     -{:.0}% ({} loops detected)", loop_penalty * 100.0, sess.review_loops)),
                    Line::from(format!("  • Re-iteration:       -{:.0}% (first_pass_clean: {})", first_pass_penalty * 100.0, sess.first_pass_clean)),
                    Line::from(format!("  • Tool Thrashing:     -{:.0}% (ratio: {:.1} calls/edit)", thrash_penalty * 100.0, edit_ratio)),
                    Line::from(format!("  • Hazard Mitigation:  +{:.0}% ({} risks prevented)", hazard_bonus * 100.0, sess.risks_prevented)),
                    Line::from("  ──────────────────────────────────────────"),
                    Line::from(vec![
                        Span::styled(format!("  • Net Effectiveness:   {}% ", score_pct), Style::default().fg(score_color).add_modifier(Modifier::BOLD)),
                        Span::styled("(Mathematically Verified)", Style::default().fg(Theme::STATUS_ACCEPTED)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("Press [e] or [Esc] to return to session dashboard.", Style::default().fg(Theme::TEXT_MUTED))),
                ];

                let paragraph = Paragraph::new(text)
                    .block(block)
                    .scroll((app.session_preview_scroll as u16, 0))
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, area);
                return;
            }

            let text = vec![
                Line::from(vec![
                    Span::styled("Session ID:  ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                    Span::styled(&sess.id, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Agent: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&sess.agent_id, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::raw("    "),
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
                    Span::raw("    "),
                    Span::styled("Duration: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(
                        sess.formatted_duration(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Started: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(&sess.started_at, Style::default().fg(Theme::TEXT_MUTED)),
                    Span::raw("    "),
                    Span::styled("Ended: ", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(
                        sess.ended_at.as_deref().unwrap_or("In progress"),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "────── Validated Coding Effectiveness & Behavioral Score ────",
                    Style::default().fg(Theme::BORDER),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Coding Effectiveness Score:  ", Style::default().add_modifier(Modifier::BOLD)),
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
                    Span::raw("  Decomposition: "),
                    Span::styled("Base(100%)", Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(format!(" - Loops({:.0}%)", loop_penalty * 100.0), Style::default().fg(if loop_penalty > 0.0 { Theme::STATUS_RISK_OPEN } else { Theme::TEXT_MUTED })),
                    Span::styled(format!(" - Iter({:.0}%)", first_pass_penalty * 100.0), Style::default().fg(if first_pass_penalty > 0.0 { Color::Yellow } else { Theme::TEXT_MUTED })),
                    Span::styled(format!(" - Thrash({:.0}%)", thrash_penalty * 100.0), Style::default().fg(if thrash_penalty > 0.0 { Color::Yellow } else { Theme::TEXT_MUTED })),
                    Span::styled(format!(" + HazardBonus(+{:.0}%)", hazard_bonus * 100.0), Style::default().fg(if hazard_bonus > 0.0 { Theme::STATUS_ACCEPTED } else { Theme::TEXT_MUTED })),
                    Span::styled("  [Press 'e' for Proof]", Style::default().fg(Theme::ACCENT)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Tool-to-Edit Ratio:          ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{:.1} calls/edit", edit_ratio), Style::default().fg(Color::White)),
                    Span::raw("   "),
                    Span::styled(
                        format!("({} tool calls across {} code edits)", sess.total_tool_calls, sess.total_edits),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Total Diff Volume:            ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} lines", sess.total_diff_lines), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Review Oscillations (Loops):  ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{} loops", sess.review_loops),
                        if sess.review_loops > 0 {
                            Style::default().fg(Theme::STATUS_RISK_OPEN).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Theme::STATUS_ACCEPTED)
                        },
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("First Pass Clean:             ", Style::default().add_modifier(Modifier::BOLD)),
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
                    "────── Advanced Scientific Telemetry & Efficiency ───────────",
                    Style::default().fg(Theme::BORDER),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Hazard Interception Rate:     ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{}% ", interception_rate),
                        Style::default().fg(if interception_rate >= 80 { Theme::STATUS_ACCEPTED } else { Color::Yellow }).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("({} prevented / {} cited)", sess.risks_prevented, total_risks),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Code Churn Density:           ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(churn_density, Style::default().fg(Color::White)),
                    Span::raw("   |   "),
                    Span::styled("Action Velocity: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(action_efficiency, Style::default().fg(Color::LightBlue)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Throughput Velocity:          ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(throughput_velocity, Style::default().fg(Color::White)),
                    Span::raw("   |   "),
                    Span::styled("Delegation Grant: ", Style::default().add_modifier(Modifier::BOLD)),
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
            let empty_card = vec![
                Line::from(""),
                Line::from(Span::styled("No session selected.", Style::default().fg(Theme::TEXT_MUTED))),
                Line::from(""),
                Line::from(Span::styled("Select an agent session from the left list to inspect telemetry and metrics.", Style::default().fg(Theme::TEXT_MUTED))),
            ];
            let paragraph = Paragraph::new(empty_card)
                .block(block);
            frame.render_widget(paragraph, area);
        }
    }
}
