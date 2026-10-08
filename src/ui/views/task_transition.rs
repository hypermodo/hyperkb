use crate::ui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub struct TaskTransitionModal;

impl TaskTransitionModal {
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

        // 1. Clear background
        frame.render_widget(Clear, modal_area);

        // 2. Outer container
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent()).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(t.bg_panel()).fg(t.text_primary()))
            .title(Span::styled(" [t] Transition Task Status ", t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [←/→/Tab] ", t.key_badge()),
                Span::styled("Select Target • ", Style::default().fg(t.text_muted())),
                Span::styled("[Enter] ", t.key_badge()),
                Span::styled("Confirm Transition • ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] ", t.key_badge()),
                Span::styled("Cancel", Style::default().fg(t.text_muted())),
            ]));

        let inner_area = outer_block.inner(modal_area);
        frame.render_widget(outer_block, modal_area);

        let proj_name = app.projects.get(app.selected_project_idx).map(|p| p.name.as_str()).unwrap_or("unknown");
        let (task_title, task_status_str) = if let Some(task) = app.native_tasks.get(app.selected_native_task_idx) {
            (task.title.clone(), format!("{:?}", task.status))
        } else if let Some(task_doc) = app.project_tasks.get(app.selected_project_task_idx) {
            (task_doc.title.clone(), format!("{:?}", task_doc.status))
        } else {
            ("No task selected".to_string(), "Unknown".to_string())
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Project & Task Title
                Constraint::Length(4), // Target Status Selector
                Constraint::Length(3), // Reason input
            ])
            .split(inner_area);

        // Header info
        let header_lines = vec![
            Line::from(vec![
                Span::styled("Project: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}   ", proj_name), Style::default().fg(t.accent())),
                Span::styled("Current: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{} ", task_status_str), Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Task: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)),
                Span::styled(task_title, Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)),
            ]),
        ];
        frame.render_widget(Paragraph::new(header_lines), chunks[0]);

        // Target Status Buttons
        let status_options = App::TASK_STATUS_TARGETS;
        let mut status_spans = Vec::new();
        status_spans.push(Span::styled("Target: ", Style::default().fg(t.text_muted()).add_modifier(Modifier::BOLD)));
        for (idx, &status_name) in status_options.iter().enumerate() {
            let is_selected = idx == app.task_transition_target_idx;
            let icon = match status_name {
                "in_progress" => "▶ ",
                "completed" => "✔ ",
                "blocked" => "✖ ",
                _ => "○ ",
            };
            let label = format!(" {}{} ", icon, status_name);
            if is_selected {
                status_spans.push(Span::styled(label, Style::default().bg(t.accent()).fg(t.bg()).add_modifier(Modifier::BOLD)));
            } else {
                status_spans.push(Span::styled(label, Style::default().fg(t.text_muted())));
            }
            status_spans.push(Span::styled(" ", Style::default()));
        }

        let selector_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border_focused()))
            .title(Span::styled(" Target Status ", t.header()));
        let selector_p = Paragraph::new(vec![Line::from(""), Line::from(status_spans)]).block(selector_block);
        frame.render_widget(selector_p, chunks[1]);

        // Reason input
        let reason_display = if app.task_transition_reason.is_empty() {
            Span::styled("Optional reason for transition (type to enter)", Style::default().fg(t.text_muted()))
        } else {
            Span::styled(&app.task_transition_reason, Style::default().fg(t.text_primary()))
        };
        let reason_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border()))
            .title(Span::styled(" Transition Reason ", t.header()));
        let reason_p = Paragraph::new(vec![Line::from(reason_display)]).block(reason_block);
        frame.render_widget(reason_p, chunks[2]);
    }
}
