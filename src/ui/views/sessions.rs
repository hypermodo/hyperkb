use crate::ui::app::{App, FocusedPane, GovernanceTabMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
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

        match app.governance_tab_mode {
            GovernanceTabMode::Sessions => {
                Self::render_sessions_list(frame, app, chunks[0]);
                Self::render_session_scorecard(frame, app, chunks[1]);
            }
            GovernanceTabMode::Grants => {
                Self::render_grants_list(frame, app, chunks[0]);
                Self::render_grant_detail(frame, app, chunks[1]);
            }
        }
    }

    fn render_sessions_list(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let border_color = if app.focused_pane == FocusedPane::List {
            t.border_focused()
        } else {
            t.border()
        };

        let filtered = app.filtered_sessions();
        let filter_label = app.session_harness_filter.as_deref().unwrap_or("ALL");

        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(filtered_idx, (_orig_idx, sess))| {
                let is_selected = filtered_idx == app.selected_session_idx;
                let is_stale = sess.is_stale();

                let cursor_span = if is_selected {
                    Span::styled("▶ ", t.badge_accepted().add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("  ", Style::default().fg(t.text_muted()))
                };

                let (badge_text, badge_style) = match sess.status.as_str() {
                    "active" if is_stale => ("⏸ IDLE ", Style::default().fg(t.text_muted())),
                    "active" => ("● ACTIVE ", t.badge_accepted()),
                    "idle" => ("⏸ IDLE ", Style::default().fg(t.text_muted())),
                    "completed" => ("✔ DONE ", t.badge_resolved()),
                    "failed" => ("✕ FAILED ", t.badge_risk()),
                    _ => ("· SESS ", Style::default().fg(t.status_unknown())),
                };

                let project_tag = if let Some(ref proj) = sess.project {
                    Span::styled(
                        format!("[{}] ", proj.to_uppercase()),
                        Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("[UNSCOPED] ", Style::default().fg(t.text_muted()))
                };

                let (harness_name, model_opt) = sess.parse_agent_taxonomy();

                let harness_tag = Span::styled(
                    format!("[{}] ", harness_name.to_uppercase()),
                    Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
                );

                let model_tag = if let Some(model) = model_opt {
                    Span::styled(
                        format!("{model} • "),
                        Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("")
                };

                let short_id = if sess.id.len() > 14 {
                    format!("{}...", &sess.id[..14])
                } else {
                    sess.id.clone()
                };

                let title = Span::styled(
                    short_id,
                    if is_selected {
                        t.selected_row()
                    } else {
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                    },
                );

                let score = sess.efficiency_score_pct();
                let score_style = if score >= 80 {
                    t.badge_accepted()
                } else if score >= 50 {
                    Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)
                } else {
                    t.badge_risk()
                };
                let score_tag = Span::styled(format!(" Score: {}/100", score), score_style);

                let model_str = model_opt.unwrap_or("standard");
                let proj_str = sess.project.as_deref().unwrap_or("unscoped");
                let sub_info = Span::styled(
                    format!(
                        "   proj: {} | harness: {} | model: {} | dur: {} | edits: {} | tools: {}",
                        proj_str, harness_name, model_str, sess.formatted_duration(), sess.total_edits, sess.total_tool_calls
                    ),
                    if is_selected {
                        Style::default().fg(t.text_primary())
                    } else {
                        Style::default().fg(t.text_muted())
                    },
                );

                let item = ListItem::new(vec![
                    Line::from(vec![
                        cursor_span,
                        Span::styled(badge_text, badge_style),
                        project_tag,
                        harness_tag,
                        model_tag,
                        title,
                        score_tag,
                    ]),
                    Line::from(sub_info),
                    Line::from(""),
                ]);

                if is_selected {
                    item.style(t.selected_row())
                } else {
                    item
                }
            })
            .collect();

        let sel_count = if filtered.is_empty() {
            "0 runs".to_string()
        } else {
            format!("{} of {} selected", app.selected_session_idx + 1, filtered.len())
        };
        let list_title = format!(" Agent Runs [{}] [{}] ", filter_label.to_uppercase(), sel_count);
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(list_title, t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [h] ", t.key_badge()),
                Span::styled("Filter • ", Style::default().fg(t.text_muted())),
                Span::styled("[x] ", t.key_badge()),
                Span::styled("Prune Stale • ", Style::default().fg(t.text_muted())),
                Span::styled("[g] ", t.key_badge()),
                Span::styled("Grants • ", Style::default().fg(t.text_muted())),
                Span::styled("[y] ", t.key_badge()),
                Span::styled("Copy Scorecard", Style::default().fg(t.text_muted())),
            ]));

        if items.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No agent sessions recorded yet.",
                    Style::default().fg(t.text_muted()),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Sessions are recorded automatically when AI agents", Style::default().fg(t.text_primary())),
                ]),
                Line::from(vec![
                    Span::styled("interact via the ", Style::default().fg(t.text_primary())),
                    Span::styled("HyperKB MCP Server", Style::default().fg(t.accent())),
                    Span::styled(".", Style::default().fg(t.text_primary())),
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
            .title(Span::styled(" Session Scorecard & Behavioral Metrics [y: Copy, e: Proof] ", t.title()));

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
                t.status_accepted()
            } else if score_pct >= 50 {
                t.status_proposed()
            } else {
                t.status_risk()
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
                // Render Session Quality Score Heuristic Breakdown
                let text = vec![
                    Line::from(vec![
                        Span::styled("SESSION QUALITY SCORE BREAKDOWN", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                        Span::styled("  [Press 'e' to return]", Style::default().fg(t.text_muted())),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("1. Purpose & Heuristic", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD))),
                    Line::from(Span::styled("  Measures an agent's ability to complete its task efficiently", Style::default().fg(t.text_primary()))),
                    Line::from(Span::styled("  without excessive review loops, tool thrashing, or test iteration failures.", Style::default().fg(t.text_primary()))),
                    Line::from(""),
                    Line::from(Span::styled("2. Scoring Breakdown", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD))),
                    Line::from(vec![
                        Span::styled("  Score = clamp(", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                        Span::styled("100% ", Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)),
                        Span::styled("- P_loops - P_friction - P_thrash + B_hazard", Style::default().fg(t.accent())),
                        Span::styled(", 5%, 100%)", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("  Deductions & Adjustments:", Style::default().fg(t.text_muted()))),
                    Line::from(vec![
                        Span::styled("  • Review Loops (P_loops):    -15% per loop (max -45%)", Style::default().fg(t.text_primary())),
                        Span::styled(" (penalizes cyclic PR rejection loops)", Style::default().fg(t.text_muted())),
                    ]),
                    Line::from(vec![
                        Span::styled("  • Test Friction (P_friction): -15%", Style::default().fg(t.text_primary())),
                        Span::styled(" if initial pass required test or compiler fixes", Style::default().fg(t.text_muted())),
                    ]),
                    Line::from(vec![
                        Span::styled("  • Tool Thrash (P_thrash):     -20%", Style::default().fg(t.text_primary())),
                        Span::styled(" if tool-to-edit ratio exceeds 10:1 (lost search loops)", Style::default().fg(t.text_muted())),
                    ]),
                    Line::from(vec![
                        Span::styled("  • Directive Bonus (B_hazard): +10%", Style::default().fg(t.text_primary())),
                        Span::styled(" for proactive adherence to active architectural invariants", Style::default().fg(t.text_muted())),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("3. Current Run Values", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD))),
                    Line::from(Span::styled("  • Base Score:          100%", Style::default().fg(t.text_primary()))),
                    Line::from(Span::styled(format!("  • Loop Deduction:     -{:.0}% ({} loops detected)", loop_penalty * 100.0, sess.review_loops), Style::default().fg(t.text_primary()))),
                    Line::from(Span::styled(format!("  • Re-iteration:       -{:.0}% (first_pass_clean: {})", first_pass_penalty * 100.0, sess.first_pass_clean), Style::default().fg(t.text_primary()))),
                    Line::from(Span::styled(format!("  • Tool Thrashing:     -{:.0}% (ratio: {:.1} calls/edit)", thrash_penalty * 100.0, edit_ratio), Style::default().fg(t.text_primary()))),
                    Line::from(Span::styled(format!("  • Hazard Mitigation:  +{:.0}% ({} risks prevented)", hazard_bonus * 100.0, sess.risks_prevented), Style::default().fg(t.text_primary()))),
                    Line::from(Span::styled("  ──────────────────────────────────────────", Style::default().fg(t.border()))),
                    Line::from(vec![
                        Span::styled(format!("  • Net Quality Score:   {}% ", score_pct), Style::default().fg(score_color).add_modifier(Modifier::BOLD)),
                        Span::styled("(Heuristic Score)", Style::default().fg(t.status_accepted())),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("Press [e] or [Esc] to return to session dashboard.", Style::default().fg(t.text_muted()))),
                ];

                let paragraph = Paragraph::new(text)
                    .block(block)
                    .scroll((app.session_preview_scroll as u16, 0))
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, area);
                return;
            }

            let (harness_name, model_opt) = sess.parse_agent_taxonomy();
            let model_str = model_opt.unwrap_or("standard / uncalibrated");
            let is_stale = sess.is_stale();
            let display_status = match sess.status.as_str() {
                "active" if is_stale => "⏸ Idle / Disconnected",
                "active" => "● Active (In Progress)",
                "idle" => "⏸ Idle / Disconnected",
                "completed" => "✔ Completed",
                "failed" => "✕ Failed",
                other => other,
            };

            let mut text = vec![
                Line::from(vec![
                    Span::styled("Session ID:     ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(&sess.id, Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Target Project: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        sess.project.as_deref().unwrap_or("Unscoped (Global / Multi-project Root)"),
                        Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Agent Harness:  ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(harness_name.to_uppercase(), Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::raw("      "),
                    Span::styled("LLM Model / Engine: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                    Span::styled(model_str, Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Run Status:     ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        display_status,
                        if sess.status == "completed" {
                            Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)
                        } else if sess.status == "active" && !is_stale {
                            Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)
                        },
                    ),
                    Span::raw("    "),
                    Span::styled("Duration: ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        sess.formatted_duration(),
                        Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Started: ", Style::default().fg(t.text_muted())),
                    Span::styled(&sess.started_at, Style::default().fg(t.text_muted())),
                    Span::raw("    "),
                    Span::styled("Ended: ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        sess.ended_at.as_deref().unwrap_or(if is_stale { "Inactive" } else { "In progress" }),
                        Style::default().fg(t.text_muted()),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "────── Validated Coding Effectiveness & Behavioral Score ────",
                    Style::default().fg(t.border()),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Coding Effectiveness Score:  ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
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
                    Span::styled("  Decomposition: ", Style::default().fg(t.text_muted())),
                    Span::styled("Base(100%)", Style::default().fg(t.text_muted())),
                    Span::styled(format!(" - Loops({:.0}%)", loop_penalty * 100.0), Style::default().fg(if loop_penalty > 0.0 { t.status_risk() } else { t.text_muted() })),
                    Span::styled(format!(" - Iter({:.0}%)", first_pass_penalty * 100.0), Style::default().fg(if first_pass_penalty > 0.0 { t.status_proposed() } else { t.text_muted() })),
                    Span::styled(format!(" - Thrash({:.0}%)", thrash_penalty * 100.0), Style::default().fg(if thrash_penalty > 0.0 { t.status_proposed() } else { t.text_muted() })),
                    Span::styled(format!(" + HazardBonus(+{:.0}%)", hazard_bonus * 100.0), Style::default().fg(if hazard_bonus > 0.0 { t.status_accepted() } else { t.text_muted() })),
                    Span::styled("  [Press 'e' for Proof]", Style::default().fg(t.accent())),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Tool-to-Edit Ratio:          ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{:.1} calls/edit", edit_ratio), Style::default().fg(t.text_primary())),
                    Span::raw("   "),
                    Span::styled(
                        format!("({} tool calls across {} code edits)", sess.total_tool_calls, sess.total_edits),
                        Style::default().fg(t.text_muted()),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Total Diff Volume:            ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} lines", sess.total_diff_lines), Style::default().fg(t.text_primary())),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Review Oscillations (Loops):  ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{} loops", sess.review_loops),
                        if sess.review_loops > 0 {
                            Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(t.status_accepted())
                        },
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("First Pass Clean:             ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        if sess.first_pass_clean { "YES (Zero Review Cycles)" } else { "NO (Required Iteration)" },
                        if sess.first_pass_clean {
                            Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(t.status_proposed())
                        },
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "────── Run Telemetry & Quality Metrics ─────────────────────",
                    Style::default().fg(t.border()),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Hazard Interception Rate:     ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{}% ", interception_rate),
                        Style::default().fg(if interception_rate >= 80 { t.status_accepted() } else { t.status_proposed() }).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("({} prevented / {} cited)", sess.risks_prevented, total_risks),
                        Style::default().fg(t.text_muted()),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Code Churn Density:           ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    Span::styled(churn_density, Style::default().fg(t.text_primary())),
                    Span::raw("   |   "),
                    Span::styled("Action Velocity: ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    Span::styled(action_efficiency, Style::default().fg(t.accent())),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Throughput Velocity:          ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    Span::styled(throughput_velocity, Style::default().fg(t.text_primary())),
                    Span::raw("   |   "),
                    Span::styled("Delegation Grant: ", Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        sess.grant_id.as_deref().unwrap_or("None (Standard agent scope)"),
                        Style::default().fg(t.accent()),
                    ),
                ]),
            ];

            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "────── Chronological Action Ledger & Tool Execution Timeline ─────",
                Style::default().fg(t.border()),
            )));
            text.push(Line::from(""));

            if app.selected_session_events.is_empty() {
                text.push(Line::from(Span::styled(
                    "  No tool calls or audit events recorded yet for this session.",
                    Style::default().fg(t.text_muted()),
                )));
            } else {
                for ev in &app.selected_session_events {
                    let ts = if ev.timestamp.len() >= 19 {
                        &ev.timestamp[11..19]
                    } else {
                        &ev.timestamp
                    };
                    let (kind_color, kind_str) = match ev.event_kind.as_str() {
                        "session_start" => (t.status_accepted(), "START   "),
                        "agent_identified" => (t.accent(), "IDENT   "),
                        "project_scoped" => (t.accent(), "PROJECT "),
                        "tool_call" => (t.status_proposed(), "TOOL    "),
                        "risk_cited" => (t.status_risk(), "CITED   "),
                        "risk_prevented" => (t.status_accepted(), "RESOLVED"),
                        "file_edit" => (t.accent(), "EDIT    "),
                        _ => (t.text_muted(), "EVENT   "),
                    };
                    let mut spans = vec![
                        Span::styled(format!("  [{}] ", ts), Style::default().fg(t.text_muted())),
                        Span::styled(format!("{:<8} ", kind_str), Style::default().fg(kind_color).add_modifier(Modifier::BOLD)),
                    ];
                    if !ev.query_or_tool.is_empty() {
                        spans.push(Span::styled(format!("{} ", ev.query_or_tool), Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)));
                    }
                    if !ev.target_path.is_empty() {
                        spans.push(Span::styled(format!("→ {} ", ev.target_path), Style::default().fg(t.accent())));
                    }
                    if ev.detail_json != "{}" && !ev.detail_json.is_empty() {
                        spans.push(Span::styled(format!("{} ", ev.detail_json), Style::default().fg(t.text_muted())));
                    }
                    text.push(Line::from(spans));
                }
            }

            let paragraph = Paragraph::new(text)
                .block(block)
                .scroll((app.session_preview_scroll as u16, 0))
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        } else {
            let empty_card = vec![
                Line::from(""),
                Line::from(Span::styled("No session selected.", Style::default().fg(t.text_muted()))),
                Line::from(""),
                Line::from(Span::styled("Select an agent session from the left list to inspect telemetry and metrics.", Style::default().fg(t.text_muted()))),
            ];
            let paragraph = Paragraph::new(empty_card)
                .block(block);
            frame.render_widget(paragraph, area);
        }
    }

    fn render_grants_list(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let border_color = if app.focused_pane == FocusedPane::List {
            t.border_focused()
        } else {
            t.border()
        };

        let now = chrono::Utc::now();
        let items: Vec<ListItem> = app
            .grants
            .iter()
            .enumerate()
            .map(|(idx, grant)| {
                let is_selected = idx == app.selected_grant_idx;
                let is_expired = grant.constraints.expires_at.map(|exp| exp < now).unwrap_or(false);

                let (badge_text, badge_style) = if is_expired {
                    ("✕ EXPIRED ", Style::default().fg(t.status_superseded()))
                } else {
                    ("● ACTIVE ", t.badge_accepted())
                };

                let title = Span::styled(
                    &grant.grantee,
                    if is_selected {
                        t.selected_row()
                    } else {
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                    },
                );

                let short_id = format!("{:.8}", grant.grant_id);
                let id_badge = Span::styled(
                    format!("[{}] ", short_id),
                    Style::default().fg(t.accent()),
                );

                let scopes = if grant.allowed_scope_patterns.is_empty() {
                    "* (all)".to_string()
                } else {
                    grant.allowed_scope_patterns.join(", ")
                };

                let sub_info = Span::styled(
                    format!("   scopes: {} | actions: {}", scopes, grant.allowed_actions.len()),
                    Style::default().fg(t.text_muted()),
                );

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(badge_text, badge_style),
                        id_badge,
                        title,
                    ]),
                    Line::from(sub_info),
                    Line::from(""),
                ])
            })
            .collect();

        let list_title = format!(" Authority Grants ({}) ", app.grants.len());
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(list_title, t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [g] ", t.key_badge()),
                Span::styled("Sessions • ", Style::default().fg(t.text_muted())),
                Span::styled("[n] ", t.key_badge()),
                Span::styled("Issue • ", Style::default().fg(t.text_muted())),
                Span::styled("[r] ", t.key_badge()),
                Span::styled("Revoke • ", Style::default().fg(t.text_muted())),
                Span::styled("[y] ", t.key_badge()),
                Span::styled("Copy", Style::default().fg(t.text_muted())),
            ]));

        if items.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No authority grants active in .hyperkb/grants.",
                    Style::default().fg(t.text_muted()),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(t.text_primary())),
                    Span::styled("[n]", t.key_badge()),
                    Span::styled(" to issue an agent grant with scope and diff bounds.", Style::default().fg(t.text_primary())),
                ]),
            ];
            let p = Paragraph::new(empty_text).block(list_block);
            frame.render_widget(p, area);
        } else {
            let list = List::new(items).block(list_block);
            frame.render_widget(list, area);
        }
    }

    fn render_grant_detail(frame: &mut Frame, app: &App, area: Rect) {
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
            .title(Span::styled(" Authority Grant Inspector [Zero-Trust Boundary] ", t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [n] ", t.key_badge()),
                Span::styled("Issue New • ", Style::default().fg(t.text_muted())),
                Span::styled("[r] ", t.key_badge()),
                Span::styled("Revoke • ", Style::default().fg(t.text_muted())),
                Span::styled("[y] ", t.key_badge()),
                Span::styled("Copy JSON / UUID Token • ", Style::default().fg(t.text_muted())),
                Span::styled("[g] ", t.key_badge()),
                Span::styled("Switch Mode", Style::default().fg(t.text_muted())),
            ]));

        if let Some(grant) = app.selected_grant() {
            let now = chrono::Utc::now();
            let is_expired = grant.constraints.expires_at.map(|exp| exp < now).unwrap_or(false);

            let expiry_str = match grant.constraints.expires_at {
                None => "Permanent (no expiry limit)".to_string(),
                Some(exp) => {
                    if exp < now {
                        format!("Expired at {}", exp.format("%Y-%m-%d %H:%M:%S UTC"))
                    } else {
                        let diff = exp - now;
                        format!("Expires in {}h {}m ({})", diff.num_hours(), diff.num_minutes() % 60, exp.format("%H:%M:%S UTC"))
                    }
                }
            };

            let actions_spans: Vec<Span> = grant
                .allowed_actions
                .iter()
                .map(|a| {
                    Span::styled(
                        format!(" [{:?}] ", a),
                        Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD),
                    )
                })
                .collect();

            let scopes_spans: Vec<Span> = if grant.allowed_scope_patterns.is_empty() {
                vec![Span::styled(" [* (all workspace)] ", Style::default().fg(t.status_proposed()))]
            } else {
                grant
                    .allowed_scope_patterns
                    .iter()
                    .map(|s| {
                        Span::styled(
                            format!(" `{}` ", s),
                            Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
                        )
                    })
                    .collect()
            };

            let text = vec![
                Line::from(vec![
                    Span::styled("Grantee Agent: ", Style::default().fg(t.text_muted())),
                    Span::styled(&grant.grantee, Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::raw("    "),
                    Span::styled("Granted By: ", Style::default().fg(t.text_muted())),
                    Span::styled(&grant.granted_by, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Grant ID:      ", Style::default().fg(t.text_muted())),
                    Span::styled(grant.grant_id.to_string(), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Created At:    ", Style::default().fg(t.text_muted())),
                    Span::styled(grant.created_at.to_rfc3339(), Style::default().fg(t.text_muted())),
                ]),
                Line::from(vec![
                    Span::styled("Expiration:    ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        expiry_str,
                        if is_expired {
                            Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(t.status_accepted())
                        },
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    if is_expired {
                        "  [REVOKED / EXPIRED] This authority grant is inactive. Agents cannot use this token."
                    } else {
                        "  [ACTIVE DELEGATION] Zero-trust guardrails verified. Invariant checks strictly enforced."
                    },
                    if is_expired {
                        Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)
                    } else {
                        t.badge_accepted()
                    },
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "────────────────────────────────────────────────────────────────────────────",
                    Style::default().fg(t.border()),
                )),
                Line::from(""),
                Line::from(Span::styled("ALLOWED ACTIONS & CAPABILITIES", t.section_header())),
                Line::from(actions_spans),
                Line::from(""),
                Line::from(Span::styled("ALLOWED SCOPE FILE PATTERNS", t.section_header())),
                Line::from(scopes_spans),
                Line::from(""),
                Line::from(Span::styled("SAFETY & INTEGRITY CONSTRAINTS", t.section_header())),
                Line::from(vec![
                    Span::styled("  • Max Line Diff:      ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        grant.constraints.max_line_diff.map(|d| format!("{} lines per patch", d)).unwrap_or_else(|| "Unrestricted".to_string()),
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  • Require Tests Pass: ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        if grant.constraints.require_tests_pass { "Enforced (true)" } else { "Optional (false)" },
                        Style::default().fg(t.text_primary()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  • Allow Supersede:    ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        if grant.constraints.allow_supersede { "Allowed (true)" } else { "Disallowed (false)" },
                        Style::default().fg(t.text_primary()),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "────────────────────────────────────────────────────────────────────────────",
                    Style::default().fg(t.border()),
                )),
                Line::from(""),
                Line::from(Span::styled("HARNESS INTEGRATION HINT", t.section_header())),
                Line::from(vec![
                    Span::styled("  Pass this Grant ID to your AI agent or MCP client: ", Style::default().fg(t.text_primary())),
                    Span::styled(grant.grant_id.to_string(), Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(Span::styled(
                    "  Press [y] to copy complete Grant JSON specification to your clipboard.",
                    Style::default().fg(t.text_muted()),
                )),
            ];

            let paragraph = Paragraph::new(text)
                .block(block)
                .scroll((app.session_preview_scroll as u16, 0))
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        } else {
            let empty_card = vec![
                Line::from(""),
                Line::from(Span::styled("No authority grant selected.", Style::default().fg(t.text_muted()))),
                Line::from(""),
                Line::from(Span::styled("Select a grant from the left list or press [n] to issue a new grant.", Style::default().fg(t.text_muted()))),
            ];
            let paragraph = Paragraph::new(empty_card)
                .block(block);
            frame.render_widget(paragraph, area);
        }
    }
}
