use crate::ui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

pub struct NewTaskModal;

impl NewTaskModal {
    pub fn modal_area(area: Rect) -> Rect {
        let width = (area.width * 70 / 100).clamp(52, 84);
        let height = (area.height * 55 / 100).clamp(14, 18);

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

        frame.render_widget(Clear, modal_area);

        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" [n] Create Tactical Task ", t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [Tab] ", t.key_badge()),
                Span::styled("Next Field • ", Style::default().fg(t.text_muted())),
                Span::styled("[Enter] ", t.key_badge()),
                Span::styled("Create Task • ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] ", t.key_badge()),
                Span::styled("Cancel", Style::default().fg(t.text_muted())),
            ]));

        let inner_area = outer_block.inner(modal_area);
        frame.render_widget(outer_block, modal_area);

        let proj_name = app.projects.get(app.selected_project_idx).map(|p| p.name.as_str()).unwrap_or("default");

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(3),
            ])
            .split(inner_area);

        let header_lines = vec![
            Line::from(vec![
                Span::styled("Target Project: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                Span::styled(proj_name, Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
            ]),
        ];
        frame.render_widget(Paragraph::new(header_lines), chunks[0]);

        let title_focused = app.new_task_field_idx == 0;
        let title_border = if title_focused { t.border_focused() } else { t.border() };
        let title_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(title_border))
            .title(Span::styled(" Task Title ", if title_focused { t.title() } else { Style::default().fg(t.text_muted()) }));
        let title_text = if app.new_task_title.is_empty() && !title_focused {
            Span::styled("Enter task title...", Style::default().fg(t.text_muted()))
        } else {
            let cursor = if title_focused { "█" } else { "" };
            Span::styled(format!("{}{}", app.new_task_title, cursor), Style::default().fg(t.text_primary()))
        };
        frame.render_widget(Paragraph::new(Line::from(title_text)).block(title_block), chunks[1]);

        let pri_focused = app.new_task_field_idx == 1;
        let pri_border = if pri_focused { t.border_focused() } else { t.border() };
        let pri_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(pri_border))
            .title(Span::styled(" Priority [←/→/Space: Adjust] ", if pri_focused { t.title() } else { Style::default().fg(t.text_muted()) }));
        let pri_spans = vec![
            Span::styled(format!(" Priority: {} ", app.new_task_priority), if pri_focused { t.selected_row() } else { Style::default().fg(t.accent()).add_modifier(Modifier::BOLD) }),
            Span::styled("  (100=Critical, 90=High, 80=Standard, 50=Low)", Style::default().fg(t.text_muted())),
        ];
        frame.render_widget(Paragraph::new(Line::from(pri_spans)).block(pri_block), chunks[2]);

        let desc_focused = app.new_task_field_idx == 2;
        let desc_border = if desc_focused { t.border_focused() } else { t.border() };
        let desc_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(desc_border))
            .title(Span::styled(" Description / Acceptance Criteria ", if desc_focused { t.title() } else { Style::default().fg(t.text_muted()) }));
        let desc_text = if app.new_task_description.is_empty() && !desc_focused {
            Span::styled("Optional task description or notes...", Style::default().fg(t.text_muted()))
        } else {
            let cursor = if desc_focused { "█" } else { "" };
            Span::styled(format!("{}{}", app.new_task_description, cursor), Style::default().fg(t.text_primary()))
        };
        frame.render_widget(Paragraph::new(Line::from(desc_text)).block(desc_block), chunks[3]);
    }
}
