use crate::ui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
    Frame,
};

pub struct ActionPaletteModal;

impl ActionPaletteModal {
    pub fn modal_area(area: Rect) -> Rect {
        let width = (area.width * 70 / 100).clamp(55, 90);
        let height = (area.height * 65 / 100).clamp(16, 26);

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
                Span::styled(" [↑ / ↓] ", t.key_badge()),
                Span::styled("Navigate • ", Style::default().fg(t.text_muted())),
                Span::styled("[Enter] ", t.key_badge()),
                Span::styled("Execute • ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] ", t.key_badge()),
                Span::styled("Dismiss", Style::default().fg(t.text_muted())),
            ]));

        let inner_area = outer_block.inner(modal_area);
        frame.render_widget(outer_block, modal_area);

        // 3. Layout: Search Bar (height 3) + Divider/List (remaining)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(5)])
            .split(inner_area);

        // 4. Render Search Bar
        let search_text = vec![
            Line::from(vec![
                Span::styled(" > ", Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
                Span::styled(
                    if app.action_palette_query.is_empty() {
                        "Type a command or filter actions...".to_string()
                    } else {
                        app.action_palette_query.clone()
                    },
                    if app.action_palette_query.is_empty() {
                        Style::default().fg(t.text_muted())
                    } else {
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                    },
                ),
                Span::styled("▌", Style::default().fg(t.accent())),
            ]),
        ];
        let search_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(t.border()));
        let search_widget = Paragraph::new(search_text).block(search_block);
        frame.render_widget(search_widget, chunks[0]);

        // 5. Render Filtered Actions List
        let actions = app.filtered_actions();
        if actions.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No matching actions found for query.",
                    Style::default().fg(t.text_muted()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Try keywords like 'check', 'directive', 'grant', 'editor', 'audit', or 'theme'.",
                    Style::default().fg(t.text_muted()),
                )),
            ];
            let empty_p = Paragraph::new(empty_text);
            frame.render_widget(empty_p, chunks[1]);
        } else {
            let selected_idx = app.action_palette_selected_idx.min(actions.len().saturating_sub(1));
            let list_items: Vec<ListItem> = actions
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let is_selected = idx == selected_idx;

                    let prefix = if is_selected { " ▶ " } else { "   " };
                    let title_style = if is_selected {
                        t.selected_row().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                    };

                    let shortcut_span = Span::styled(
                        format!("[{}] ", item.shortcut),
                        if is_selected {
                            Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
                        } else {
                            t.key_badge()
                        },
                    );

                    let title_span = Span::styled(format!("{:<30}", item.title), title_style);

                    let desc_span = Span::styled(
                        item.description,
                        if is_selected {
                            Style::default().fg(t.text_primary())
                        } else {
                            Style::default().fg(t.text_muted())
                        },
                    );

                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(prefix, Style::default().fg(t.accent())),
                            shortcut_span,
                            title_span,
                            Span::raw(" "),
                            desc_span,
                        ]),
                    ])
                })
                .collect();

            let action_list = List::new(list_items);
            frame.render_widget(action_list, chunks[1]);
        }
    }
}
