use crate::ui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

pub struct ActionPaletteModal;

impl ActionPaletteModal {
    pub fn modal_area(area: Rect) -> Rect {
        let width = (area.width * 75 / 100).clamp(70, 105).min(area.width);
        let height = (area.height * 75 / 100).clamp(18, 26).min(area.height);

        let horiz_pad = area.width.saturating_sub(width) / 2;
        let vert_pad = area.height.saturating_sub(height) / 2;

        Rect::new(
            area.x + horiz_pad,
            area.y + vert_pad,
            width,
            height,
        )
    }

    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let t = &app.theme;
        let modal_area = Self::modal_area(area);

        // 1. Clear background behind modal
        frame.render_widget(Clear, modal_area);

        // 2. Outer container block
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" ⚡ Action Palette & Tool Launcher ", t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [↑/↓] ", t.key_badge()),
                Span::styled("Navigate  •  ", Style::default().fg(t.text_muted())),
                Span::styled("[Enter] ", t.key_badge()),
                Span::styled("Execute  •  ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] ", t.key_badge()),
                Span::styled("Dismiss", Style::default().fg(t.text_muted())),
            ]));

        let inner_area = outer_block.inner(modal_area);
        frame.render_widget(outer_block, modal_area);

        // 3. Layout: Search Bar (height 3) + Actions List (flex) + Detail Inspector (height 5)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar
                Constraint::Min(6),    // List of actions
                Constraint::Length(5), // Bottom detail inspector
            ])
            .split(inner_area);

        // 4. Render Search Bar
        let search_text = vec![
            Line::from(vec![
                Span::styled("  🔍 ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled(
                    if app.action_palette_query.is_empty() {
                        "Type a command name, keyword, or CLI shortcut to filter...".to_string()
                    } else {
                        app.action_palette_query.clone()
                    },
                    if app.action_palette_query.is_empty() {
                        Style::default().fg(t.text_muted())
                    } else {
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                    },
                ),
                Span::styled("▌", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
            ]),
        ];
        let search_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(t.border()));
        let search_widget = Paragraph::new(search_text).block(search_block);
        frame.render_widget(search_widget, chunks[0]);

        // 5. Render Filtered Actions List
        let actions = app.filtered_actions();
        let selected_idx = if actions.is_empty() {
            0
        } else {
            app.action_palette_selected_idx.min(actions.len().saturating_sub(1))
        };

        if actions.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "   No matching actions found for query.",
                    Style::default().fg(t.text_muted()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "   Try keywords like 'check', 'directive', 'grant', 'editor', 'audit', or 'theme'.",
                    Style::default().fg(t.text_muted()),
                )),
            ];
            let empty_p = Paragraph::new(empty_text);
            frame.render_widget(empty_p, chunks[1]);
        } else {
            let list_width = chunks[1].width as usize;

            let list_items: Vec<ListItem> = actions
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let is_selected = idx == selected_idx;

                    let prefix = if is_selected { " ▶ " } else { "   " };
                    let shortcut_str = format!(" [{}] ", item.shortcut);
                    let prefix_len = 3;
                    let shortcut_len = shortcut_str.chars().count();

                    let max_title_len = list_width.saturating_sub(prefix_len + shortcut_len + 4);
                    let display_title = if item.title.chars().count() > max_title_len {
                        let mut s: String = item.title.chars().take(max_title_len.saturating_sub(1)).collect();
                        s.push('…');
                        s
                    } else {
                        item.title.to_string()
                    };
                    let title_len = display_title.chars().count();

                    let spacing = list_width.saturating_sub(prefix_len + title_len + shortcut_len + 1);
                    let pad = " ".repeat(spacing);

                    let row_style = if is_selected {
                        t.selected_row()
                    } else {
                        Style::default().fg(t.text_primary())
                    };

                    let title_style = if is_selected {
                        row_style.add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                    };

                    let shortcut_style = if is_selected {
                        Style::default()
                            .fg(t.bg())
                            .bg(t.accent())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        t.key_badge()
                    };

                    let bg_color = if is_selected {
                        row_style.bg.unwrap_or(Color::Rgb(30, 41, 59))
                    } else {
                        Color::Reset
                    };

                    let line = Line::from(vec![
                        Span::styled(
                            prefix,
                            if is_selected {
                                Style::default().fg(t.accent()).bg(bg_color).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(t.text_muted())
                            },
                        ),
                        Span::styled(display_title, title_style),
                        Span::styled(pad, Style::default().bg(bg_color)),
                        Span::styled(shortcut_str, shortcut_style),
                    ]);

                    ListItem::new(line).style(row_style)
                })
                .collect();

            let action_list = List::new(list_items);
            frame.render_widget(action_list, chunks[1]);
        }

        // 6. Render Bottom Detail Inspector
        let selected_item = actions.get(selected_idx);
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border()))
            .style(Style::default().bg(t.bg_panel()))
            .padding(Padding::new(2, 2, 0, 0));

        let detail_text = if let Some(item) = selected_item {
            vec![
                Line::from(vec![
                    Span::styled(" ℹ  ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(item.description, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("    Shortcut: ", Style::default().fg(t.text_muted())),
                    Span::styled(format!(" [{}] ", item.shortcut), t.key_badge()),
                    Span::styled("   •   CLI Equivalent: ", Style::default().fg(t.text_muted())),
                    Span::styled(item.cli_command, Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
                ]),
            ]
        } else {
            vec![
                Line::from(Span::styled("  No action selected.", Style::default().fg(t.text_muted()))),
            ]
        };

        let detail_p = Paragraph::new(detail_text)
            .block(detail_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(detail_p, chunks[2]);
    }
}
