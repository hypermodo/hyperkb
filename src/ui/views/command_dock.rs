use crate::ui::app::{App, SlashCommand};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

pub struct CommandDock;

impl CommandDock {
    pub fn render(frame: &mut Frame, app: &App, area: Rect, divider_x: Option<u16>) {
        frame.render_widget(Clear, area);
        let t = &app.theme;

        let (border_color, is_focused) = if app.repl_active {
            (t.accent(), true)
        } else {
            (t.border(), false)
        };

        let pending_info = app.pending_agent_query.as_ref().map(|(name, start)| {
            let elapsed = start.elapsed().as_secs_f32();
            (name.as_str(), elapsed)
        });

        let (_context_label, context_placeholder) = app.active_tab_context();

        let title_line = if let Some((h_name, elapsed)) = pending_info {
            Line::from(vec![
                Span::styled(" Thinking (", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{} • {:.1}s", h_name, elapsed), Style::default().fg(t.status_proposed())),
                Span::styled(") ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
            ])
        } else if is_focused {
            Line::from(vec![
                Span::styled(" Ask AI or Command [/] ", t.title()),
            ])
        } else {
            Line::from(vec![
                Span::styled(" Ask AI or Command [/] ", Style::default().fg(t.text_muted())),
            ])
        };

        let title_bottom = Line::from("");

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_focused {
                Style::default().fg(border_color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(border_color)
            })
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(title_line)
            .title_bottom(title_bottom);

        let mut lines = Vec::new();

        let clean_input = app.repl_input.replace("\r\n", "\n").replace('\r', "\n");
        if !is_focused {
            if let Some((h_name, elapsed)) = pending_info {
                lines.push(Line::from(vec![
                    Span::styled(" ⏳ ", Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("Running headless {} query ({:.1}s)... Results stream to Logs [c]", h_name, elapsed),
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else if clean_input.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("❯ ", Style::default().fg(t.text_muted())),
                    Span::styled(
                        context_placeholder,
                        Style::default().fg(t.text_muted()),
                    ),
                ]));
            } else {
                let input_lines: Vec<&str> = clean_input.split('\n').collect();
                let show_line_numbers = input_lines.len() > 1;
                for (idx, line_str) in input_lines.iter().enumerate() {
                    let prefix = if show_line_numbers {
                        format!("{:>2} │ ", idx + 1)
                    } else {
                        "❯ ".to_string()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(t.text_muted())),
                        Span::styled(*line_str, Style::default().fg(t.text_primary())),
                    ]));
                }
            }
        } else if clean_input.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("❯ ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled("▌", Style::default().fg(t.accent())),
                Span::styled(
                    format!(" {}", context_placeholder),
                    Style::default().fg(t.text_muted()),
                ),
            ]));
        } else {
            let input_lines: Vec<&str> = clean_input.split('\n').collect();
            let show_line_numbers = input_lines.len() > 1;
            for (idx, line_str) in input_lines.iter().enumerate() {
                let is_last = idx == input_lines.len() - 1;
                let prefix = if show_line_numbers {
                    format!("{:>2} │ ", idx + 1)
                } else {
                    "❯ ".to_string()
                };

                let line_fg = if line_str.starts_with('/') {
                    t.status_proposed()
                } else {
                    t.text_primary()
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
                    Span::styled(*line_str, Style::default().fg(line_fg).add_modifier(Modifier::BOLD)),
                ];
                if is_last {
                    spans.push(Span::styled("▌", Style::default().fg(t.accent())));
                }
                lines.push(Line::from(spans));
            }
        }

        let p = Paragraph::new(lines).block(block);
        frame.render_widget(p, area);

        // Seamlessly unify the frame with the upper panes on area.y:
        let buf = frame.buffer_mut();
        let y = area.y;
        let junction_style = if is_focused {
            Style::default().fg(border_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(border_color)
        };

        // 1. Left T-junction: connects left pane border with dock left border
        if area.width > 0 && y < buf.area.height {
            let left_cell = &mut buf[(area.x, y)];
            left_cell.set_symbol("├");
            left_cell.set_style(junction_style);
        }

        // 2. Right T-junction: connects right pane border with dock right border
        if area.width > 1 && y < buf.area.height {
            let right_x = area.x + area.width - 1;
            let right_cell = &mut buf[(right_x, y)];
            right_cell.set_symbol("┤");
            right_cell.set_style(junction_style);
        }

        // 3. Middle vertical divider junction(s) from upper panes
        if let Some(split_x) = divider_x {
            if split_x > area.x && split_x < area.x + area.width - 1 && y < buf.area.height {
                if split_x > area.x + 1 {
                    let cell_left = &mut buf[(split_x - 1, y)];
                    cell_left.set_symbol("┴");
                    cell_left.set_style(junction_style);
                }
                let cell_right = &mut buf[(split_x, y)];
                cell_right.set_symbol("┴");
                cell_right.set_style(junction_style);
            }
        }
    }

    pub fn render_slash_menu(
        frame: &mut Frame,
        app: &App,
        filtered: &[&'static SlashCommand],
        area: Rect,
    ) {
        frame.render_widget(Clear, area);
        let t = &app.theme;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(1, 1, 0, 0))
            .title(Span::styled(" Command Palette & AI Harnesses ", t.title()))
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
                    Span::styled(format!("{:<14}", cmd.name), t.selected_row()),
                    Span::styled(format!("  {}", cmd.description), Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("   /", Style::default().fg(t.text_muted())),
                    Span::styled(format!("{:<14}", cmd.name), Style::default().fg(t.status_proposed())),
                    Span::styled(format!("  {}", cmd.description), Style::default().fg(t.text_muted())),
                ]));
            }
        }

        let p = Paragraph::new(lines).block(block);
        frame.render_widget(p, area);
    }
}
