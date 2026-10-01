use crate::ui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

pub struct NewDirectiveModal;

impl NewDirectiveModal {
    pub fn modal_area(area: Rect) -> Rect {
        let width = (area.width * 78 / 100).clamp(60, 92);
        let height = (area.height * 78 / 100).clamp(20, 30);

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
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" [n] Draft New Directive (Governance Cockpit) ", t.title()))
            .title_bottom(Line::from(vec![
                Span::styled(" [Tab] ", t.key_badge()),
                Span::styled("Next Field • ", Style::default().fg(t.text_muted())),
                Span::styled("[Enter] ", t.key_badge()),
                Span::styled("Newline (in Rule) • ", Style::default().fg(t.text_muted())),
                Span::styled("[Ctrl+S] ", t.key_badge()),
                Span::styled("Save & Enforce • ", Style::default().fg(t.text_muted())),
                Span::styled("[Esc] ", t.key_badge()),
                Span::styled("Cancel", Style::default().fg(t.text_muted())),
            ]));

        let inner_area = outer_block.inner(modal_area);
        frame.render_widget(outer_block, modal_area);

        // 3. Layout: Subtitle + Form Rows
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Subtitle / boundary note
                Constraint::Length(3), // Field 0: Title
                Constraint::Length(3), // Field 1: Category & Field 3: Enforcement (side-by-side or stacked)
                Constraint::Length(3), // Field 2: Scope
                Constraint::Min(5),    // Field 4: Policy rule statement (multi-line)
                Constraint::Length(2), // Field 5: Save & Enforce action button
            ])
            .split(inner_area);

        // Subtitle & Product boundary reminder
        let subtitle = vec![
            Line::from(vec![
                Span::styled(
                    "Register a mandatory invariant or standing policy. ",
                    Style::default().fg(t.text_primary()),
                ),
                Span::styled(
                    "[TUI: Governance & Guardrails | IDE: Rich Prose ([o])]",
                    Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
                ),
            ]),
        ];
        frame.render_widget(Paragraph::new(subtitle), chunks[0]);

        // Field 0: Title
        let is_f0 = app.new_directive_field == 0;
        let f0_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_f0 {
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.border())
            })
            .title(Span::styled(
                if is_f0 { " Title (Press Tab when done) ▶ " } else { " Title " },
                if is_f0 { t.title() } else { Style::default().fg(t.text_muted()) },
            ));
        let title_content = vec![Line::from(vec![
            Span::styled(
                if app.new_directive_title.is_empty() && !is_f0 {
                    "e.g. Disallow Raw SQL Queries".to_string()
                } else {
                    app.new_directive_title.clone()
                },
                if app.new_directive_title.is_empty() {
                    Style::default().fg(t.text_muted())
                } else {
                    Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD)
                },
            ),
            if is_f0 {
                Span::styled("▌", Style::default().fg(t.accent()))
            } else {
                Span::raw("")
            },
        ])];
        frame.render_widget(Paragraph::new(title_content).block(f0_block), chunks[1]);

        // Row 2: Category (Field 1) and Enforcement (Field 3)
        let row2_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        // Field 1: Category
        let is_f1 = app.new_directive_field == 1;
        let categories = &app.manifest.taxonomy.categories;
        let selected_cat = if categories.is_empty() {
            "behavior"
        } else {
            &categories[app.new_directive_category_idx % categories.len()].id
        };
        let cat_label = if categories.is_empty() {
            "Code & Agent Behavior"
        } else {
            &categories[app.new_directive_category_idx % categories.len()].label
        };

        let f1_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_f1 {
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.border())
            })
            .title(Span::styled(
                if is_f1 { " Category ([Space] to cycle) ▶ " } else { " Category " },
                if is_f1 { t.title() } else { Style::default().fg(t.text_muted()) },
            ));
        let cat_content = vec![Line::from(vec![
            Span::styled(" ◆ ", Style::default().fg(t.accent())),
            Span::styled(
                selected_cat.to_uppercase(),
                Style::default().fg(t.status_proposed()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({})", cat_label),
                Style::default().fg(t.text_muted()),
            ),
        ])];
        frame.render_widget(Paragraph::new(cat_content).block(f1_block), row2_chunks[0]);

        // Field 3: Enforcement
        let is_f3 = app.new_directive_field == 3;
        let enforcements = &["mandatory", "advisory"];
        let selected_enf = enforcements[app.new_directive_enforcement_idx % enforcements.len()];
        let f3_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_f3 {
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.border())
            })
            .title(Span::styled(
                if is_f3 { " Enforcement ([Space] to toggle) ▶ " } else { " Enforcement " },
                if is_f3 { t.title() } else { Style::default().fg(t.text_muted()) },
            ));
        let enf_content = vec![Line::from(vec![
            Span::styled(" 🛡 ", Style::default().fg(t.accent())),
            Span::styled(
                selected_enf.to_uppercase(),
                if selected_enf == "mandatory" {
                    Style::default().fg(t.status_risk()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.status_accepted()).add_modifier(Modifier::BOLD)
                },
            ),
        ])];
        frame.render_widget(Paragraph::new(enf_content).block(f3_block), row2_chunks[1]);

        // Field 2: Scope
        let is_f2 = app.new_directive_field == 2;
        let f2_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_f2 {
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.border())
            })
            .title(Span::styled(
                if is_f2 { " Scope Pattern (* for global, or path globs) ▶ " } else { " Scope Pattern " },
                if is_f2 { t.title() } else { Style::default().fg(t.text_muted()) },
            ));
        let scope_content = vec![Line::from(vec![
            Span::styled(
                if app.new_directive_scope.is_empty() && !is_f2 {
                    "*".to_string()
                } else {
                    app.new_directive_scope.clone()
                },
                Style::default().fg(t.text_primary()).add_modifier(Modifier::BOLD),
            ),
            if is_f2 {
                Span::styled("▌", Style::default().fg(t.accent()))
            } else {
                Span::raw("")
            },
        ])];
        frame.render_widget(Paragraph::new(scope_content).block(f2_block), chunks[3]);

        // Field 4: Policy rule text (multi-line)
        let is_f4 = app.new_directive_field == 4;
        let f4_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_f4 {
                Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.border())
            })
            .title(Span::styled(
                if is_f4 { " Invariant Rule Statement ([Enter] for newline, [Tab] to Save) ▶ " } else { " Invariant Rule Statement " },
                if is_f4 { t.title() } else { Style::default().fg(t.text_muted()) },
            ));

        let rule_content = if app.new_directive_rule.is_empty() && !is_f4 {
            vec![Line::from(Span::styled(
                "e.g. All SQLite access must go through the Queries struct.\nNever write raw SQL strings in handlers.",
                Style::default().fg(t.text_muted()),
            ))]
        } else {
            let mut lines = Vec::new();
            let text = if app.new_directive_rule.is_empty() { "" } else { &app.new_directive_rule };
            let split_lines: Vec<&str> = text.split('\n').collect();
            let num_lines = split_lines.len();

            for (idx, line_str) in split_lines.iter().enumerate() {
                let is_last = idx == num_lines - 1;
                if is_last && is_f4 {
                    lines.push(Line::from(vec![
                        Span::styled(line_str.to_string(), Style::default().fg(t.text_primary())),
                        Span::styled("▌", Style::default().fg(t.accent())),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(
                        line_str.to_string(),
                        Style::default().fg(t.text_primary()),
                    )));
                }
            }
            lines
        };

        frame.render_widget(
            Paragraph::new(rule_content)
                .block(f4_block)
                .wrap(Wrap { trim: false }),
            chunks[4],
        );

        // Field 5: Action Button
        let is_f5 = app.new_directive_field == 5;
        let btn_style = if is_f5 {
            t.selected_row().add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)
        };
        let btn_line = Line::from(vec![
            Span::styled(
                if is_f5 { " ▶ [ Save & Enforce Directive (Press Enter) ] " } else { "   [ Save & Enforce Directive ] " },
                btn_style,
            ),
            Span::styled("  •  Tip: Press [Ctrl+S] anytime to save & enforce immediately", Style::default().fg(t.text_muted())),
        ]);
        frame.render_widget(Paragraph::new(btn_line), chunks[5]);
    }
}
