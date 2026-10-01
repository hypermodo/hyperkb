use crate::ui::app::{App, FocusedPane};
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

pub struct SettingsView;

impl SettingsView {
    pub fn render(frame: &mut Frame, app: &App, area: Rect) {
        let list_width = (area.width * 40 / 100).clamp(38, 65);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Min(40)])
            .split(area);

        Self::render_settings_list(frame, app, chunks[0]);
        Self::render_setting_detail(frame, app, chunks[1]);
    }

    fn render_settings_list(frame: &mut Frame, app: &App, area: Rect) {
        let border_color = if app.focused_pane == FocusedPane::List {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER
        };

        let settings = [
            ("Directives Ceiling (Rule of N)", format!("{} directives", app.manifest.settings.max_briefing_directives)),
            ("Stale Document Horizon", format!("{} days", app.manifest.settings.stale_days_threshold)),
            ("Max Document Line Ceiling", format!("{} lines", app.manifest.settings.audit_max_lines)),
            ("Max Folder Nesting Depth", format!("{} levels", app.manifest.settings.audit_max_depth)),
            ("Active Visual Theme", app.theme.as_str().to_string()),
            ("Mouse Navigation Mode", if app.mouse_capture { "Enabled".to_string() } else { "Disabled (Text Copy)".to_string() }),
        ];

        let items: Vec<ListItem> = settings
            .iter()
            .enumerate()
            .map(|(idx, (name, val))| {
                let is_selected = idx == app.settings_selected_idx;
                let title_style = if is_selected {
                    Theme::selected_row()
                } else {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("  {}  ", if is_selected { "●" } else { "○" }), if is_selected { Style::default().fg(Theme::ACCENT) } else { Style::default().fg(Theme::TEXT_MUTED) }),
                        Span::styled(name.to_string(), title_style),
                    ]),
                    Line::from(vec![
                        Span::raw("      Current: "),
                        Span::styled(val.clone(), Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(""),
                ])
            })
            .collect();

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::new(2, 2, 1, 1))
            .title(Span::styled(" Configurable Settings & Policy Knobs ", Theme::title()));

        let list = List::new(items).block(list_block);
        frame.render_widget(list, area);
    }

    fn render_setting_detail(frame: &mut Frame, app: &App, area: Rect) {
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
            .title(Span::styled(" Setting Details & Live Adjustment ", title_style));

        let (title, val_str, desc, impact) = match app.settings_selected_idx {
            0 => (
                "Directives Ceiling (Rule of N)",
                format!("{} directives", app.manifest.settings.max_briefing_directives),
                "Controls the maximum number of active directives injected into AI agent session briefings and pre-commit checks. Originally set to 5 (Rule of 5) to balance guardrail coverage against prompt context bloat.",
                "Lower values reduce LLM prompt token costs and decrease agent distraction. Higher values permit more specific policy rules to be active across broader codebase paths simultaneously."
            ),
            1 => (
                "Stale Document Horizon",
                format!("{} days", app.manifest.settings.stale_days_threshold),
                "Threshold in days before an unverified knowledge base document (Decision, Risk, or Spec) is flagged during 'hyperkb audit' and briefings as requiring architect re-verification.",
                "Prevents knowledge base drift and phantom architectural guidelines that no longer reflect the production codebase."
            ),
            2 => (
                "Max Document Line Ceiling",
                format!("{} lines", app.manifest.settings.audit_max_lines),
                "The maximum allowed line count for any individual KB markdown document before the anti-bloat linter flags it as oversized.",
                "Enforces concise, modular ADRs and specs. When documents grow beyond this ceiling, authors should split them into focused child decisions."
            ),
            3 => (
                "Max Folder Nesting Depth",
                format!("{} directory levels", app.manifest.settings.audit_max_depth),
                "Maximum directory nesting depth permitted within the knowledge base root before the linter raises a taxonomy hierarchy violation.",
                "Eliminates deep, labyrinthine folder trees in favor of shallow, discoverable topic groupings (e.g. decisions/001.md)."
            ),
            4 => (
                "Active Visual Theme",
                app.theme.as_str().to_string(),
                "User interface color palette and styling for the interactive terminal application.",
                "Themes include Cyberpunk (Terminal Neon), Modern (Electric Slate), Nord (Arctic Frost), Tokyo Night (Storm & Lavender), and Light (Paper & Indigo)."
            ),
            5 => (
                "Mouse Navigation Mode",
                if app.mouse_capture { "Enabled (Click to Navigate)".to_string() } else { "Disabled (Native Text Selection & Copy)".to_string() },
                "Controls terminal mouse event capture. When enabled, mouse clicks switch tabs, select items, and expand tree folders. When disabled, standard click-and-drag terminal text selection and copying are enabled.",
                "Press [m] at any time in any view to quickly toggle this without entering Settings."
            ),
            _ => ("Setting", "".to_string(), "", ""),
        };

        let text = vec![
            Line::from(vec![
                Span::styled("Setting: ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Current Value: ", Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
                Span::styled(val_str, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled("────── Adjustment Controls ──────────────────────────────────", Style::default().fg(Theme::BORDER))),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Use "),
                Span::styled("[←] / [→]", Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)),
                Span::raw(" or "),
                Span::styled("[h] / [l]", Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)),
                Span::raw(" or "),
                Span::styled("[-] / [+]", Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)),
                Span::raw(" to adjust value."),
            ]),
            Line::from(vec![
                Span::raw("  Press "),
                Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" to save changes to "),
                Span::styled("hyperkb.json", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("."),
            ]),
            Line::from(""),
            Line::from(Span::styled("────── Description & Rationale ──────────────────────────────", Style::default().fg(Theme::BORDER))),
            Line::from(""),
            Line::from(Span::styled(desc, Style::default().fg(Color::White))),
            Line::from(""),
            Line::from(Span::styled("────── Engineering Impact ───────────────────────────────────", Style::default().fg(Theme::BORDER))),
            Line::from(""),
            Line::from(Span::styled(impact, Style::default().fg(Theme::TEXT_MUTED))),
            Line::from(""),
            Line::from(Span::styled("────────────────────────────────────────────────────────────", Style::default().fg(Theme::BORDER))),
            Line::from(vec![
                Span::styled(
                    if app.settings_dirty {
                        "● Unsaved changes: press [Enter] to write to hyperkb.json"
                    } else {
                        "✔ All settings synced with hyperkb.json"
                    },
                    if app.settings_dirty {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Theme::STATUS_ACCEPTED)
                    },
                ),
            ]),
        ];

        let p = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        frame.render_widget(p, area);
    }
}
