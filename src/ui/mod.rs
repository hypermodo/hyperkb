pub mod app;
pub mod layout;
pub mod markdown;
pub mod theme;
pub mod views;

use app::{ActiveTab, App, ExploreTreeItem};
use crossterm::{
    event::{self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, Event, KeyCode, KeyModifiers, KeyboardEnhancementFlags, MouseButton, MouseEventKind, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use layout::{Footer, Header};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
    Terminal,
};
use std::io::{self, Write};
use std::panic;
use std::path::Path;
use std::time::Duration;
use views::{CommandDock, DirectivesView, ExploreView, HelpModal, IssueGrantModal, NewDirectiveModal, ReaderView, SessionsView, SettingsView, TaskTransitionModal, WorkView};
use crate::storage::Database;

pub fn run(root: &Path, db: &Database, collection_id: &str, profile_id: &str) -> io::Result<()> {
    // 1. Setup panic hook so terminal is ALWAYS restored safely if something panics
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableBracketedPaste, DisableMouseCapture, PopKeyboardEnhancementFlags);
        let _ = io::stdout().write_all(b"\x1b[>4;0m\x1b[>4m\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
        let _ = io::stdout().flush();
        original_hook(panic_info);
    }));

    // 2. Initialize App and load initial data
    let mut app = App::new_with_root(root, collection_id, profile_id);
    app.refresh_data(db);

    // 3. Setup terminal in raw mode & alternate screen buffer with mouse capture conditional on settings
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    );
    // Also enable xterm modifyOtherKeys=2 for maximum Shift+Enter compatibility
    let _ = stdout.write_all(b"\x1b[>4;2m");
    let _ = stdout.flush();

    if app.mouse_capture {
        execute!(stdout, EnterAlternateScreen, EnableBracketedPaste, EnableMouseCapture)?;
        let _ = stdout.flush();
    } else {
        execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
        let _ = stdout.write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
        let _ = stdout.flush();
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 4. Main Event Loop (Smooth 60 FPS non-blocking polling)
    let res = run_loop(root, &mut terminal, &mut app, db);

    // 5. Restore terminal state
    disable_raw_mode()?;
    let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    let _ = terminal.backend_mut().write_all(b"\x1b[>4;0m\x1b[>4m");
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableBracketedPaste, DisableMouseCapture)?;
    let _ = terminal.backend_mut().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
    let _ = terminal.backend_mut().flush();
    terminal.show_cursor()?;

    res
}

fn run_loop(
    root: &Path,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    db: &Database,
) -> io::Result<()> {
    let mut needs_redraw = true;
    let mut last_periodic_refresh = std::time::Instant::now();

    while !app.should_quit {
        // Periodic background database refresh (every 1.5s)
        // Keeps agent runs, action ledger, telemetry, and documents streaming in live
        if last_periodic_refresh.elapsed() >= std::time::Duration::from_millis(1500) {
            app.refresh_data(db);
            last_periodic_refresh = std::time::Instant::now();
            needs_redraw = true;
        }

        // Poll background AI agent thread non-blocking
        if let Some(ref rx) = app.agent_rx {
            match rx.try_recv() {
                Ok(entry) => {
                    if let Some(pos) = app.diagnostic_stream.iter().position(|e| e.id == "pending_agent_query") {
                        app.diagnostic_stream[pos] = entry;
                    } else {
                        app.diagnostic_stream.push(entry);
                    }
                    app.selected_diagnostic_idx = app.diagnostic_stream.len().saturating_sub(1);
                    app.diagnostic_scroll = 0;
                    app.agent_rx = None;
                    app.pending_agent_query = None;
                    app.status_message = Some("AI response received".to_string());
                    needs_redraw = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Running in background - keep responsive
                    needs_redraw = true;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    app.agent_rx = None;
                    app.pending_agent_query = None;
                    needs_redraw = true;
                }
            }
        }

        if needs_redraw {
            let mut highlighted_text: Option<String> = None;
            terminal.draw(|frame| {
                let area = frame.area();
                let t = app.theme;

                // Clear entire frame and fill with active theme background color
                frame.render_widget(Clear, area);
                let bg_block = Block::default().style(Style::default().bg(t.bg()).fg(t.text_primary()));
                frame.render_widget(bg_block, area);

                let input_lines = app.repl_input.lines().count().max(1);
                let prompt_h = if area.height >= 30 {
                    (input_lines as u16 + 5).clamp(6, 14)
                } else {
                    (input_lines as u16 + 4).clamp(5, 12)
                };
                let footer_h: u16 = if area.height >= 22 { 2 } else { 1 };
                let filtered_slash = app.filtered_slash_commands();
                let slash_h = if app.repl_active && !filtered_slash.is_empty() {
                    (filtered_slash.len() as u16 + 2).min(7)
                } else {
                    0
                };

                let dock_y = area.height.saturating_sub(footer_h).saturating_sub(prompt_h);
                let header_area = Rect { x: area.x, y: area.y, width: area.width, height: 3 };
                let main_tab_h = dock_y.saturating_sub(3) + 1; // + 1 so row dock_y is shared as the single unified frame divider!
                let main_tab_area = Rect { x: area.x, y: area.y + 3, width: area.width, height: main_tab_h };
                let dock_area = Rect { x: area.x, y: dock_y, width: area.width, height: prompt_h };
                let footer_area = Rect { x: area.x, y: dock_y + prompt_h, width: area.width, height: footer_h };

                Header::render(frame, app, header_area);

                frame.render_widget(Clear, main_tab_area);
                match app.active_tab {
                    ActiveTab::Work => WorkView::render(frame, app, main_tab_area),
                    ActiveTab::Explore => ExploreView::render(frame, app, main_tab_area),
                    ActiveTab::Directives => DirectivesView::render(frame, app, main_tab_area),
                    ActiveTab::Sessions => SessionsView::render(frame, app, main_tab_area),
                    ActiveTab::Settings => SettingsView::render(frame, app, main_tab_area),
                    ActiveTab::Reader => ReaderView::render(frame, app, main_tab_area),
                }

                let divider_x = match app.active_tab {
                    ActiveTab::Work => {
                        if app.work_tab_mode == crate::ui::app::WorkTabMode::Projects && !app.projects.is_empty() {
                            Some((area.width * 38 / 100).clamp(32, 60))
                        } else if app.work_tab_mode == crate::ui::app::WorkTabMode::Risks && !app.active_risks.is_empty() {
                            Some(area.width * 45 / 100)
                        } else {
                            None
                        }
                    }
                    ActiveTab::Explore | ActiveTab::Directives | ActiveTab::Sessions => {
                        Some((area.width * 38 / 100).clamp(36, 68))
                    }
                    ActiveTab::Settings => {
                        Some((area.width * 40 / 100).clamp(38, 65))
                    }
                    _ => None,
                };

                frame.render_widget(Clear, dock_area);
                CommandDock::render(frame, app, dock_area, divider_x);

                if slash_h > 0 {
                    let slash_y = dock_y.saturating_sub(slash_h);
                    let slash_area = Rect { x: area.x, y: slash_y, width: area.width, height: slash_h };
                    frame.render_widget(Clear, slash_area);
                    CommandDock::render_slash_menu(frame, app, &filtered_slash, slash_area);
                }

                Footer::render(frame, app, footer_area);

                if app.show_new_directive_modal {
                    NewDirectiveModal::render(frame, app, area);
                } else if app.show_issue_grant_modal {
                    IssueGrantModal::render(frame, app, area);
                } else if app.show_task_transition_modal {
                    TaskTransitionModal::render(frame, app, area);
                } else if app.show_help {
                    HelpModal::render(frame, app, area);
                }

                // Visual in-TUI mouse drag selection highlight
                if app.is_dragging {
                    if let (Some((start_col, start_row)), Some((curr_col, curr_row))) = (app.drag_start, app.drag_current) {
                        let (from, to) = if (start_row, start_col) <= (curr_row, curr_col) {
                            ((start_row, start_col), (curr_row, curr_col))
                        } else {
                            ((curr_row, curr_col), (start_row, start_col))
                        };

                        let buffer = frame.buffer_mut();
                        let buf_area = buffer.area;
                        let mut selected_text = String::new();

                        for r in from.0..=to.0 {
                            if r >= buf_area.height {
                                continue;
                            }
                            let c_start = if r == from.0 { from.1 } else { 0 };
                            let c_end = if r == to.0 { to.1 } else { buf_area.width.saturating_sub(1) };

                            let mut line_str = String::new();
                            for c in c_start..=c_end {
                                if c >= buf_area.width {
                                    continue;
                                }
                                let cell = &mut buffer[(c, r)];
                                line_str.push_str(cell.symbol());
                                cell.set_style(
                                    Style::default()
                                        .bg(t.accent())
                                        .fg(t.bg())
                                );
                            }
                            if !selected_text.is_empty() {
                                selected_text.push('\n');
                            }
                            selected_text.push_str(line_str.trim_end());
                        }
                        highlighted_text = Some(selected_text);
                    }
                }
            })?;

            if let Some(txt) = highlighted_text {
                app.last_selected_text = Some(txt);
            }
            needs_redraw = false;
        }

        // Event-driven reactive polling: 80ms while background AI agent runs, 250ms when idle (0% CPU)
        let poll_timeout = if app.agent_rx.is_some() {
            Duration::from_millis(80)
        } else {
            Duration::from_millis(250)
        };

        if event::poll(poll_timeout)? {
            let old_proj_idx = app.selected_project_idx;
            needs_redraw = true;
            match event::read()? {
                Event::Key(key) => {
                    // Global quit shortcuts
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.should_quit = true;
                        break;
                    }

                    if app.status_message.is_some() {
                        app.status_message = None;
                    }

                    if app.show_new_directive_modal {
                        match key.code {
                            KeyCode::Esc => {
                                app.show_new_directive_modal = false;
                            }
                            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                match app.draft_new_directive(db) {
                                    Ok(msg) => app.status_message = Some(msg),
                                    Err(err) => app.status_message = Some(format!("Error: {}", err)),
                                }
                            }
                            KeyCode::Tab => {
                                app.new_directive_field = (app.new_directive_field + 1) % 6;
                            }
                            KeyCode::BackTab => {
                                if app.new_directive_field == 0 {
                                    app.new_directive_field = 5;
                                } else {
                                    app.new_directive_field -= 1;
                                }
                            }
                            KeyCode::Char(' ') => {
                                if app.new_directive_field == 1 {
                                    app.new_directive_category_idx = (app.new_directive_category_idx + 1) % 4;
                                } else if app.new_directive_field == 3 {
                                    app.new_directive_enforcement_idx = (app.new_directive_enforcement_idx + 1) % 2;
                                } else if app.new_directive_field == 0 {
                                    app.new_directive_title.push(' ');
                                } else if app.new_directive_field == 2 {
                                    app.new_directive_scope.push(' ');
                                } else if app.new_directive_field == 4 {
                                    app.new_directive_rule.push(' ');
                                } else if app.new_directive_field == 5 {
                                    match app.draft_new_directive(db) {
                                        Ok(msg) => app.status_message = Some(msg),
                                        Err(err) => app.status_message = Some(format!("Error: {}", err)),
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if app.new_directive_field == 0 {
                                    app.new_directive_title.pop();
                                } else if app.new_directive_field == 2 {
                                    app.new_directive_scope.pop();
                                } else if app.new_directive_field == 4 {
                                    app.new_directive_rule.pop();
                                }
                            }
                            KeyCode::Enter => {
                                if app.new_directive_field == 4 {
                                    app.new_directive_rule.push('\n');
                                } else if app.new_directive_field == 0 {
                                    app.new_directive_field = 1;
                                } else {
                                    match app.draft_new_directive(db) {
                                        Ok(msg) => app.status_message = Some(msg),
                                        Err(err) => app.status_message = Some(format!("Error: {}", err)),
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if app.new_directive_field == 0 {
                                    app.new_directive_title.push(c);
                                } else if app.new_directive_field == 1 {
                                    app.new_directive_category_idx = (app.new_directive_category_idx + 1) % 4;
                                } else if app.new_directive_field == 2 {
                                    app.new_directive_scope.push(c);
                                } else if app.new_directive_field == 3 {
                                    app.new_directive_enforcement_idx = (app.new_directive_enforcement_idx + 1) % 2;
                                } else if app.new_directive_field == 4 {
                                    app.new_directive_rule.push(c);
                                }
                            }
                            _ => {}
                        }
                    } else if app.show_issue_grant_modal {
                        match key.code {
                            KeyCode::Esc => {
                                app.show_issue_grant_modal = false;
                            }
                            KeyCode::Tab => {
                                app.new_grant_field = (app.new_grant_field + 1) % 3;
                            }
                            KeyCode::BackTab => {
                                if app.new_grant_field == 0 {
                                    app.new_grant_field = 2;
                                } else {
                                    app.new_grant_field -= 1;
                                }
                            }
                            KeyCode::Char(' ') => {
                                if app.new_grant_field == 1 {
                                    app.new_grant_preset_idx = (app.new_grant_preset_idx + 1) % views::IssueGrantModal::PRESETS.len();
                                } else if app.new_grant_field == 2 {
                                    app.new_grant_ttl_hours = match app.new_grant_ttl_hours {
                                        0 => 1,
                                        1 => 4,
                                        4 => 8,
                                        8 => 24,
                                        _ => 0,
                                    };
                                } else if app.new_grant_field == 0 {
                                    app.new_grant_grantee.push(' ');
                                }
                            }
                            KeyCode::Backspace => {
                                if app.new_grant_field == 0 {
                                    app.new_grant_grantee.pop();
                                }
                            }
                            KeyCode::Enter => {
                                match app.issue_new_grant() {
                                    Ok(msg) => app.status_message = Some(msg),
                                    Err(err) => app.status_message = Some(format!("Error: {}", err)),
                                }
                            }
                            KeyCode::Char(c) => {
                                if app.new_grant_field == 0 {
                                    app.new_grant_grantee.push(c);
                                } else if app.new_grant_field == 1 {
                                    app.new_grant_preset_idx = (app.new_grant_preset_idx + 1) % views::IssueGrantModal::PRESETS.len();
                                } else if app.new_grant_field == 2 {
                                    app.new_grant_ttl_hours = match app.new_grant_ttl_hours {
                                        0 => 1,
                                        1 => 4,
                                        4 => 8,
                                        8 => 24,
                                        _ => 0,
                                    };
                                }
                            }
                            _ => {}
                        }
                    } else if app.show_task_transition_modal {
                        match key.code {
                            KeyCode::Esc => {
                                app.show_task_transition_modal = false;
                            }
                            KeyCode::Tab | KeyCode::Right => {
                                app.next_task_transition_target();
                            }
                            KeyCode::BackTab | KeyCode::Left => {
                                app.prev_task_transition_target();
                            }
                            KeyCode::Backspace => {
                                app.task_transition_reason.pop();
                            }
                            KeyCode::Enter => {
                                if let Err(err) = app.submit_task_transition(db) {
                                    app.status_message = Some(format!("Error: {}", err));
                                }
                            }
                            KeyCode::Char(c) => {
                                app.task_transition_reason.push(c);
                            }
                            _ => {}
                        }
                    } else if app.show_help {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                                app.show_help = false;
                            }
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                let plain = HelpModal::help_text_plain();
                                App::copy_text_to_clipboard(&plain);
                                app.status_message = Some("✔ Copied System Documentation to clipboard (Cmd+V to paste)".to_string());
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.help_scroll += 1;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.help_scroll = app.help_scroll.saturating_sub(1);
                            }
                            KeyCode::PageDown => {
                                app.help_scroll += 8;
                            }
                            KeyCode::PageUp => {
                                app.help_scroll = app.help_scroll.saturating_sub(8);
                            }
                            _ => {}
                        }
                    } else if app.is_filtering {
                        match key.code {
                            KeyCode::Esc => {
                                app.is_filtering = false;
                                app.filter_query.clear();
                            }
                            KeyCode::Enter => {
                                app.is_filtering = false;
                            }
                            KeyCode::Backspace => {
                                app.filter_query.pop();
                            }
                            KeyCode::Char(c) => {
                                app.filter_query.push(c);
                            }
                            _ => {}
                        }
                    } else if app.repl_active {
                        match key.code {
                            KeyCode::Esc => {
                                if !app.repl_input.is_empty() {
                                    app.repl_input.clear();
                                    app.slash_menu_selected_idx = 0;
                                } else {
                                    app.repl_active = false;
                                }
                            }
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.repl_input.clear();
                                app.slash_menu_selected_idx = 0;
                            }
                            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.repl_input.clear();
                                app.slash_menu_selected_idx = 0;
                            }
                            KeyCode::Enter => {
                                if key.modifiers.contains(KeyModifiers::SHIFT)
                                    || key.modifiers.contains(KeyModifiers::ALT)
                                    || key.modifiers.contains(KeyModifiers::CONTROL)
                                    || key.modifiers.contains(KeyModifiers::SUPER)
                                {
                                    // Multi-line support: Shift+Enter, Option+Enter, Ctrl+Enter, or Cmd+Enter adds newline
                                    app.repl_input.push('\n');
                                } else if app.repl_input.ends_with('\\') {
                                    // Trailing backslash continuation (shell standard)
                                    app.repl_input.pop();
                                    app.repl_input.push('\n');
                                } else {
                                    let filtered = app.filtered_slash_commands();
                                    if app.repl_input.starts_with('/') && !filtered.is_empty() && !app.repl_input.contains(' ') {
                                        let sel = app.slash_menu_selected_idx.min(filtered.len().saturating_sub(1));
                                        let cmd = filtered[sel].name;
                                        app.execute_repl_command(cmd, db);
                                    } else {
                                        let cmd = app.repl_input.clone();
                                        app.execute_repl_command(&cmd, db);
                                    }
                                }
                            }
                            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+J standard line feed
                                app.repl_input.push('\n');
                            }
                            KeyCode::Char('\n') | KeyCode::Char('\r') => {
                                // Direct newline / carriage return character
                                app.repl_input.push('\n');
                            }
                            KeyCode::Tab => {
                                let filtered = app.filtered_slash_commands();
                                if app.repl_input.starts_with('/') && !filtered.is_empty() && !app.repl_input.contains(' ') {
                                    let sel = app.slash_menu_selected_idx.min(filtered.len().saturating_sub(1));
                                    app.repl_input = format!("/{} ", filtered[sel].name);
                                }
                            }
                            KeyCode::BackTab => {
                                let filtered = app.filtered_slash_commands();
                                if app.repl_input.starts_with('/') && !filtered.is_empty() {
                                    if app.slash_menu_selected_idx == 0 {
                                        app.slash_menu_selected_idx = filtered.len().saturating_sub(1);
                                    } else {
                                        app.slash_menu_selected_idx -= 1;
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                app.repl_input.pop();
                                app.slash_menu_selected_idx = 0;
                            }
                            KeyCode::Up => {
                                let filtered = app.filtered_slash_commands();
                                if app.repl_input.starts_with('/') && !filtered.is_empty() {
                                    if app.slash_menu_selected_idx == 0 {
                                        app.slash_menu_selected_idx = filtered.len().saturating_sub(1);
                                    } else {
                                        app.slash_menu_selected_idx -= 1;
                                    }
                                } else if !app.repl_history.is_empty() {
                                    if app.repl_history_idx == 0 {
                                        app.repl_history_idx = app.repl_history.len().saturating_sub(1);
                                    } else {
                                        app.repl_history_idx = app.repl_history_idx.saturating_sub(1);
                                    }
                                    if let Some(hist) = app.repl_history.get(app.repl_history_idx) {
                                        app.repl_input = hist.clone();
                                    }
                                }
                            }
                            KeyCode::Down => {
                                let filtered = app.filtered_slash_commands();
                                if app.repl_input.starts_with('/') && !filtered.is_empty() {
                                    app.slash_menu_selected_idx = (app.slash_menu_selected_idx + 1) % filtered.len();
                                } else if !app.repl_history.is_empty() {
                                    app.repl_history_idx = (app.repl_history_idx + 1) % app.repl_history.len();
                                    if let Some(hist) = app.repl_history.get(app.repl_history_idx) {
                                        app.repl_input = hist.clone();
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if c == '\r' {
                                    app.repl_input.push('\n');
                                } else {
                                    app.repl_input.push(c);
                                }
                                app.slash_menu_selected_idx = 0;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => app.should_quit = true,
                            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.repl_active = true;
                                app.repl_input = "/".to_string();
                                app.slash_menu_selected_idx = 0;
                            }
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                match app.open_active_document_in_editor() {
                                    Ok(msg) => app.status_message = Some(msg),
                                    Err(err) => app.status_message = Some(err),
                                }
                            }
                            KeyCode::Char('?') | KeyCode::F(1) => app.toggle_help(),
                            KeyCode::Char('h') | KeyCode::Char('H') if app.active_tab == ActiveTab::Sessions => {
                                app.cycle_session_harness_filter();
                            }
                            KeyCode::Char('h') if app.active_tab != ActiveTab::Settings => app.toggle_help(),
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                match app.copy_active_content_to_clipboard() {
                                    Ok(desc) => {
                                        app.status_message = Some(format!("✔ Copied {} to clipboard (Cmd+V to paste)", desc));
                                    }
                                    Err(err) => {
                                        app.status_message = Some(format!("Warning: {}", err));
                                    }
                                }
                            }
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                let enabled = app.toggle_mouse();
                                if enabled {
                                    let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
                                    let _ = terminal.backend_mut().flush();
                                    app.status_message = Some("Mouse Mode: ON (Click to navigate, drag to highlight & copy)".to_string());
                                } else {
                                    let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                                    let _ = terminal.backend_mut().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
                                    let _ = terminal.backend_mut().flush();
                                    app.status_message = Some("Mouse Mode: OFF (Native terminal drag-selection enabled without modifier keys)".to_string());
                                }
                            }
                            KeyCode::Char('t') if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Projects => {
                                app.open_task_transition_modal();
                            }
                            KeyCode::Char('T') => {
                                app.next_theme();
                            }
                            KeyCode::Char('e') | KeyCode::Char('E') if app.active_tab == ActiveTab::Sessions => {
                                app.toggle_scoring_methodology();
                            }
                            KeyCode::Char('g') | KeyCode::Char('G') if app.active_tab == ActiveTab::Sessions => {
                                app.toggle_governance_tab_mode();
                            }
                            KeyCode::Char('1') => app.switch_tab(ActiveTab::Work),
                            KeyCode::Char('2') => app.switch_tab(ActiveTab::Explore),
                            KeyCode::Char('3') => app.switch_tab(ActiveTab::Directives),
                            KeyCode::Char('4') => app.switch_tab(ActiveTab::Sessions),
                            KeyCode::Char('5') => app.switch_tab(ActiveTab::Settings),
                            KeyCode::Tab => {
                                if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Console {
                                    app.next_diagnostic_file();
                                } else {
                                    app.toggle_pane();
                                }
                            }
                            KeyCode::BackTab => {
                                if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Console {
                                    app.prev_diagnostic_file();
                                } else {
                                    app.toggle_pane();
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.next();
                                if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Projects {
                                    app.refresh_project_tasks(db);
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.prev();
                                if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Projects {
                                    app.refresh_project_tasks(db);
                                }
                            }
                            KeyCode::Left | KeyCode::Char('h') if app.active_tab == ActiveTab::Settings => {
                                if app.focused_pane == crate::ui::app::FocusedPane::Detail {
                                    app.focused_pane = crate::ui::app::FocusedPane::List;
                                } else {
                                    let old_mouse = app.mouse_capture;
                                    app.adjust_setting(-1);
                                    if app.mouse_capture != old_mouse {
                                        if app.mouse_capture {
                                            let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
                                            let _ = terminal.backend_mut().flush();
                                        } else {
                                            let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                                            let _ = terminal.backend_mut().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
                                            let _ = terminal.backend_mut().flush();
                                        }
                                    }
                                }
                            }
                            KeyCode::Right | KeyCode::Char('l') if app.active_tab == ActiveTab::Settings => {
                                if app.focused_pane == crate::ui::app::FocusedPane::List {
                                    app.focused_pane = crate::ui::app::FocusedPane::Detail;
                                } else {
                                    let old_mouse = app.mouse_capture;
                                    app.adjust_setting(1);
                                    if app.mouse_capture != old_mouse {
                                        if app.mouse_capture {
                                            let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
                                            let _ = terminal.backend_mut().flush();
                                        } else {
                                            let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                                            let _ = terminal.backend_mut().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
                                            let _ = terminal.backend_mut().flush();
                                        }
                                    }
                                }
                            }
                            KeyCode::Left | KeyCode::Char('h') if app.active_tab == ActiveTab::Work => {
                                app.focused_pane = crate::ui::app::FocusedPane::List;
                            }
                            KeyCode::Right | KeyCode::Char('l') if app.active_tab == ActiveTab::Work => {
                                app.focused_pane = crate::ui::app::FocusedPane::Detail;
                            }
                            KeyCode::Char('g') | KeyCode::Home if app.active_tab == ActiveTab::Reader => {
                                app.scroll_reader_to_top();
                            }
                            KeyCode::Char('G') | KeyCode::End if app.active_tab == ActiveTab::Reader => {
                                app.scroll_reader_to_end();
                            }
                            KeyCode::Char('x') | KeyCode::Char('X') if app.active_tab == ActiveTab::Sessions => {
                                app.prune_stale_sessions(db);
                            }
                            KeyCode::Char('-') if app.active_tab == ActiveTab::Settings => {
                                let old_mouse = app.mouse_capture;
                                app.adjust_setting(-1);
                                if app.mouse_capture != old_mouse {
                                    if app.mouse_capture {
                                        let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
                                        let _ = terminal.backend_mut().flush();
                                    } else {
                                        let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                                        let _ = terminal.backend_mut().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
                                        let _ = terminal.backend_mut().flush();
                                    }
                                }
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') if app.active_tab == ActiveTab::Settings => {
                                let old_mouse = app.mouse_capture;
                                app.adjust_setting(1);
                                if app.mouse_capture != old_mouse {
                                    if app.mouse_capture {
                                        let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
                                        let _ = terminal.backend_mut().flush();
                                    } else {
                                        let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                                        let _ = terminal.backend_mut().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
                                    }
                                }
                            }
                            KeyCode::PageDown => app.page_down(),
                            KeyCode::PageUp => app.page_up(),
                            KeyCode::Home => app.scroll_to_top(),
                            KeyCode::End => app.scroll_to_bottom(),
                            KeyCode::Char('g') => app.scroll_to_top(),
                            KeyCode::Char('G') => app.scroll_to_bottom(),
                            KeyCode::Char('J') => app.scroll_preview_down(6),
                            KeyCode::Char('K') => app.scroll_preview_up(6),
                            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Projects && app.focused_pane == crate::ui::app::FocusedPane::Detail {
                                    app.scroll_preview_down(6);
                                } else {
                                    app.page_down();
                                }
                            }
                            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Projects && app.focused_pane == crate::ui::app::FocusedPane::Detail {
                                    app.scroll_preview_up(6);
                                } else {
                                    app.page_up();
                                }
                            }
                            KeyCode::Char(' ') => {
                                if app.active_tab == ActiveTab::Reader {
                                    app.page_down();
                                } else if app.active_tab == ActiveTab::Explore && app.explore_tree_mode {
                                    app.open_selected();
                                } else if app.active_tab == ActiveTab::Settings {
                                    if app.settings_selected_idx == 5 {
                                        let old_mouse = app.mouse_capture;
                                        app.adjust_setting(1);
                                        if app.mouse_capture != old_mouse {
                                            if app.mouse_capture {
                                                let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
                                                let _ = terminal.backend_mut().flush();
                                            } else {
                                                let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                                                let _ = terminal.backend_mut().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
                                                let _ = terminal.backend_mut().flush();
                                            }
                                        }
                                    } else if app.settings_selected_idx == 4 {
                                        app.adjust_setting(1);
                                    } else if app.focused_pane == crate::ui::app::FocusedPane::List && (app.settings_selected_idx == 7 || app.settings_selected_idx == 6) {
                                        app.focused_pane = crate::ui::app::FocusedPane::Detail;
                                    }
                                } else {
                                    // Unified entry to Command Dock
                                    app.repl_active = true;
                                    app.repl_input = "/".to_string();
                                    app.slash_menu_selected_idx = 0;
                                }
                            }
                            KeyCode::Enter => {
                                if app.active_tab == ActiveTab::Settings {
                                    if app.settings_selected_idx == 5 {
                                        let old_mouse = app.mouse_capture;
                                        app.adjust_setting(1);
                                        if app.mouse_capture != old_mouse {
                                            if app.mouse_capture {
                                                let _ = execute!(terminal.backend_mut(), EnableMouseCapture);
                                                let _ = terminal.backend_mut().flush();
                                            } else {
                                                let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                                                let _ = terminal.backend_mut().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
                                                let _ = terminal.backend_mut().flush();
                                            }
                                        }
                                    } else if app.settings_selected_idx == 4 {
                                        app.adjust_setting(1);
                                    } else if app.focused_pane == crate::ui::app::FocusedPane::List && (app.settings_selected_idx == 7 || app.settings_selected_idx == 6) {
                                        app.focused_pane = crate::ui::app::FocusedPane::Detail;
                                    } else {
                                        let _ = app.save_settings(root);
                                    }
                                } else if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Console {
                                    app.repl_active = true;
                                } else {
                                    app.open_selected();
                                }
                            }
                            KeyCode::Esc => {
                                if app.active_tab == ActiveTab::Settings && app.focused_pane == crate::ui::app::FocusedPane::Detail {
                                    app.focused_pane = crate::ui::app::FocusedPane::List;
                                } else {
                                    app.go_back();
                                }
                            }
                            KeyCode::Char('p') | KeyCode::Char('P') if app.active_tab == ActiveTab::Work => {
                                app.work_tab_mode = crate::ui::app::WorkTabMode::Projects;
                                if app.projects.is_empty() {
                                    app.refresh_data(db);
                                }
                            }
                            KeyCode::Char('w') | KeyCode::Char('W') if app.active_tab == ActiveTab::Work => {
                                app.work_tab_mode = crate::ui::app::WorkTabMode::Risks;
                            }
                            KeyCode::Char(':') if app.active_tab == ActiveTab::Work => {
                                app.work_tab_mode = crate::ui::app::WorkTabMode::Console;
                                app.repl_active = true;
                            }
                            KeyCode::Left | KeyCode::Char('[') if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Console => {
                                app.prev_diagnostic_entry();
                            }
                            KeyCode::Right | KeyCode::Char(']') if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Console => {
                                app.next_diagnostic_entry();
                            }
                            KeyCode::Char('t') if app.active_tab == ActiveTab::Explore => {
                                app.toggle_explore_tree_mode();
                            }
                            KeyCode::Char('v') => app.toggle_raw_view(),
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                if app.active_tab == ActiveTab::Explore {
                                    app.next_category(db);
                                } else if app.active_tab == ActiveTab::Directives {
                                    app.next_directive_category(db);
                                } else if app.active_tab == ActiveTab::Work {
                                    app.cycle_work_tab_mode(db);
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') if app.active_tab == ActiveTab::Directives => {
                                app.show_new_directive_modal = true;
                                app.new_directive_field = 0;
                                app.new_directive_title.clear();
                                app.new_directive_rule.clear();
                                app.new_directive_scope = "*".to_string();
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                if app.active_tab == ActiveTab::Directives {
                                    match app.toggle_selected_directive_status(db) {
                                        Ok(msg) => app.status_message = Some(msg),
                                        Err(err) => app.status_message = Some(err),
                                    }
                                } else if app.active_tab == ActiveTab::Sessions && app.governance_tab_mode == app::GovernanceTabMode::Grants {
                                    match app.revoke_selected_grant() {
                                        Ok(msg) => app.status_message = Some(msg),
                                        Err(err) => app.status_message = Some(err),
                                    }
                                } else if app.active_tab == ActiveTab::Work {
                                    match app.execute_action_palette_item("check_work", db) {
                                        Ok(msg) => app.status_message = Some(msg),
                                        Err(err) => app.status_message = Some(format!("Error: {}", err)),
                                    }
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') if app.active_tab == ActiveTab::Sessions && app.governance_tab_mode == app::GovernanceTabMode::Grants => {
                                app.show_issue_grant_modal = true;
                                app.new_grant_field = 0;
                                app.new_grant_grantee.clear();
                            }
                            KeyCode::Char('a') | KeyCode::Char('A') => {
                                if app.active_tab == ActiveTab::Directives {
                                    app.show_new_directive_modal = true;
                                    app.new_directive_field = 0;
                                } else if app.active_tab == ActiveTab::Work {
                                    match app.execute_action_palette_item("audit_kb", db) {
                                        Ok(msg) => app.status_message = Some(msg),
                                        Err(err) => app.status_message = Some(format!("Error: {}", err)),
                                    }
                                }
                            }
                            KeyCode::Char('/') => {
                                app.repl_active = true;
                                app.repl_input = "/".to_string();
                                app.slash_menu_selected_idx = 0;
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    // When mouse capture is disabled, DO NOT process any mouse events.
                    // This ensures terminal emulator mouse-drag text selection operations are never intercepted or interrupted by HyperKB.
                    if !app.mouse_capture {
                        continue;
                    }

                    let col = mouse.column;
                    let row = mouse.row;
                    let size = terminal.size()?;
                    let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);


                    // If New Directive modal is open, dismiss when clicking outside
                    if app.show_new_directive_modal {
                        let modal = NewDirectiveModal::modal_area(area);
                        let inside = col >= modal.x
                            && col < modal.x + modal.width
                            && row >= modal.y
                            && row < modal.y + modal.height;
                        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                            if !inside {
                                app.show_new_directive_modal = false;
                            }
                        }
                        continue;
                    }

                    // If Issue Grant modal is open, dismiss when clicking outside
                    if app.show_issue_grant_modal {
                        let modal = IssueGrantModal::modal_area(area);
                        let inside = col >= modal.x
                            && col < modal.x + modal.width
                            && row >= modal.y
                            && row < modal.y + modal.height;
                        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                            if !inside {
                                app.show_issue_grant_modal = false;
                            }
                        }
                        continue;
                    }

                    // If Help modal is currently open, isolate all mouse actions to the modal overlay.
                    if app.show_help {
                        let modal = HelpModal::modal_area(area);
                        let inside = col >= modal.x
                            && col < modal.x + modal.width
                            && row >= modal.y
                            && row < modal.y + modal.height;

                        match mouse.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                if !inside {
                                    // Clicking outside the help modal dismisses it safely!
                                    app.show_help = false;
                                } else {
                                    app.drag_start = Some((col, row));
                                    app.drag_current = Some((col, row));
                                    app.is_dragging = false;
                                }
                                continue;
                            }
                            MouseEventKind::Drag(MouseButton::Left) => {
                                if let Some((sc, sr)) = app.drag_start {
                                    if col != sc || row != sr {
                                        app.is_dragging = true;
                                        app.drag_current = Some((col, row));
                                    }
                                }
                                continue;
                            }
                            MouseEventKind::Up(MouseButton::Left) => {
                                if app.is_dragging {
                                    if let Some(text) = app.last_selected_text.take() {
                                        let trimmed = text.trim();
                                        if !trimmed.is_empty() {
                                            App::copy_text_to_clipboard(trimmed);
                                            let preview = if trimmed.len() > 32 {
                                                format!("{}...", &trimmed[..32])
                                            } else {
                                                trimmed.to_string()
                                            };
                                            app.status_message = Some(format!("✔ Copied '{}' to clipboard (Cmd+V to paste)", preview));
                                        }
                                    }
                                    app.is_dragging = false;
                                    app.drag_start = None;
                                    app.drag_current = None;
                                } else if inside {
                                    let is_top_bar = row == modal.y;
                                    let is_bottom_bar = row == modal.y + modal.height.saturating_sub(1);
                                    if is_top_bar || is_bottom_bar {
                                        app.show_help = false;
                                    }
                                    app.drag_start = None;
                                    app.drag_current = None;
                                }
                                continue;
                            }
                            MouseEventKind::ScrollDown => {
                                app.help_scroll += 2;
                                continue;
                            }
                            MouseEventKind::ScrollUp => {
                                app.help_scroll = app.help_scroll.saturating_sub(2);
                                continue;
                            }
                            _ => {
                                continue;
                            }
                        }
                    }

                    let footer_h: u16 = if area.height >= 22 { 2 } else { 1 };

                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            app.drag_start = Some((col, row));
                            app.drag_current = Some((col, row));
                            app.is_dragging = false;
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            if let Some((sc, sr)) = app.drag_start {
                                if col != sc || row != sr {
                                    app.is_dragging = true;
                                    app.drag_current = Some((col, row));
                                }
                            }
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            if app.is_dragging {
                                if let Some(text) = app.last_selected_text.take() {
                                    let trimmed = text.trim();
                                    if !trimmed.is_empty() {
                                        App::copy_text_to_clipboard(trimmed);
                                        let preview = if trimmed.len() > 32 {
                                            format!("{}...", &trimmed[..32])
                                        } else {
                                            trimmed.to_string()
                                        };
                                        app.status_message = Some(format!("✔ Copied '{}' to clipboard (Cmd+V to paste)", preview));
                                    }
                                }
                                app.is_dragging = false;
                                app.drag_start = None;
                                app.drag_current = None;
                            } else {
                                app.drag_start = None;
                                app.drag_current = None;

                                if app.status_message.is_some() {
                                    app.status_message = None;
                                }

                                // 1. Header clicks (row 0: tabs, row 1: context sub-bar)
                                if row <= 2 {
                                    Header::handle_click(app, db, col, row);
                                }
                                // 2. Main content & Universal Command Dock clicks
                                else if row >= 3 && row < area.height.saturating_sub(footer_h) {
                                    let input_lines = app.repl_input.lines().count().max(1);
                                    let prompt_h = if area.height >= 30 {
                                        (input_lines as u16 + 5).clamp(6, 14)
                                    } else {
                                        (input_lines as u16 + 4).clamp(5, 12)
                                    };
                                    let filtered_slash = app.filtered_slash_commands();
                                    let slash_h = if app.repl_active && !filtered_slash.is_empty() {
                                        (filtered_slash.len() as u16 + 2).min(7)
                                    } else {
                                        0
                                    };
                                    let prompt_y = area.height.saturating_sub(footer_h).saturating_sub(prompt_h);

                                    if slash_h > 0 {
                                        let slash_y = prompt_y.saturating_sub(slash_h);
                                        if row >= slash_y && row < prompt_y {
                                            let rel_slash_row = row.saturating_sub(slash_y + 1) as usize;
                                            if rel_slash_row < filtered_slash.len() {
                                                let cmd = filtered_slash[rel_slash_row].name;
                                                app.execute_repl_command(cmd, db);
                                            }
                                            continue;
                                        }
                                    }

                                    if row >= prompt_y {
                                        app.repl_active = true;
                                        continue;
                                    }

                                    if app.repl_active {
                                        app.repl_active = false;
                                    }

                                    let list_width = match app.active_tab {
                                        ActiveTab::Work => {
                                            if app.work_tab_mode == crate::ui::app::WorkTabMode::Projects {
                                                area.width * 38 / 100
                                            } else {
                                                area.width * 45 / 100
                                            }
                                        }
                                        ActiveTab::Reader => 0,
                                        ActiveTab::Settings => (area.width * 40 / 100).clamp(38, 65),
                                        _ => (area.width * 38 / 100).clamp(36, 68),
                                    };

                                    let rel_row = row.saturating_sub(3);
                                    if col < list_width {
                                        app.focused_pane = crate::ui::app::FocusedPane::List;
                                        if rel_row >= 2 {
                                            match app.active_tab {
                                                ActiveTab::Work => {
                                                    if app.work_tab_mode == crate::ui::app::WorkTabMode::Projects {
                                                        let item_idx = app.projects_list_state.offset() + ((rel_row - 2) / 3) as usize;
                                                        if item_idx < app.projects.len() {
                                                            app.set_selected_project(item_idx, db);
                                                        }
                                                    } else {
                                                        let item_idx = app.risks_list_state.offset() + ((rel_row - 2) / 3) as usize;
                                                        if item_idx < app.active_risks.len() {
                                                            app.selected_risk_idx = item_idx;
                                                        }
                                                    }
                                                }
                                                ActiveTab::Directives => {
                                                    let item_idx = app.directives_list_state.offset() + ((rel_row - 2) / 3) as usize;
                                                    if item_idx < app.directives.len() {
                                                        app.selected_directive_idx = item_idx;
                                                    }
                                                }
                                                ActiveTab::Sessions => {
                                                    let item_idx = app.sessions_list_state.offset() + ((rel_row - 2) / 3) as usize;
                                                    if item_idx < app.filtered_sessions().len() {
                                                        app.selected_session_idx = item_idx;
                                                    }
                                                }
                                                ActiveTab::Settings => {
                                                    let item_idx = ((rel_row - 2) / 3) as usize;
                                                    if item_idx < 8 {
                                                        app.settings_selected_idx = item_idx;
                                                    }
                                                }
                                                ActiveTab::Explore => {
                                                    if app.explore_tree_mode {
                                                        let tree_idx = app.tree_list_state.offset() + ((rel_row - 2) / 2) as usize;
                                                        let tree = app.build_explore_tree();
                                                        if tree_idx < tree.len() {
                                                            if app.selected_tree_idx == tree_idx {
                                                                app.open_selected();
                                                            } else {
                                                                app.selected_tree_idx = tree_idx;
                                                                if let ExploreTreeItem::Doc { doc_idx, .. } = &tree[tree_idx] {
                                                                    app.selected_doc_idx = *doc_idx;
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        let item_idx = app.documents_list_state.offset() + ((rel_row - 2) / 3) as usize;
                                                        if item_idx < app.documents.len() {
                                                            app.selected_doc_idx = item_idx;
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    } else {
                                        app.focused_pane = crate::ui::app::FocusedPane::Detail;
                                        if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Projects {
                                            let overview_h: u16 = if area.height < 28 { 5 } else { 8 };
                                            let rem_h = area.height.saturating_sub(footer_h).saturating_sub(3).saturating_sub(overview_h);
                                            let task_box_h = rem_h * 55 / 100;
                                            let task_start_y = overview_h + 1;
                                            let task_end_y = overview_h + task_box_h;
                                            if rel_row >= task_start_y && rel_row < task_end_y {
                                                let task_idx = app.project_tasks_list_state.offset() + ((rel_row - task_start_y) / 2) as usize;
                                                if task_idx < app.project_tasks.len() {
                                                    app.selected_project_task_idx = task_idx;
                                                    app.cockpit_preview_scroll = 0;
                                                }
                                            }
                                        } else if app.active_tab == ActiveTab::Settings && app.settings_selected_idx == 7 {
                                            if rel_row >= 5 {
                                                let clicked_harness = ((rel_row - 5) / 2) as usize;
                                                if clicked_harness < app.harnesses.len() {
                                                    app.selected_harness_idx = clicked_harness;
                                                    let sel_id = app.harnesses[clicked_harness].id.clone();
                                                    app.manifest.harnesses.active_harness_id = Some(sel_id);
                                                    app.settings_dirty = true;
                                                    app.status_message = Some(format!(
                                                        "Active AI Harness: {}",
                                                        app.harnesses[clicked_harness].name
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                                // 3. Footer clicks
                                else if row >= area.height.saturating_sub(footer_h) && col >= area.width.saturating_sub(12) {
                                    app.should_quit = true;
                                }
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if app.show_help {
                                app.help_scroll += 2;
                            } else {
                                let list_width = match app.active_tab {
                                    ActiveTab::Work => {
                                        if app.work_tab_mode == crate::ui::app::WorkTabMode::Projects {
                                            area.width * 38 / 100
                                        } else {
                                            area.width * 45 / 100
                                        }
                                    }
                                    ActiveTab::Reader => 0,
                                    ActiveTab::Settings => (area.width * 40 / 100).clamp(38, 65),
                                    _ => (area.width * 38 / 100).clamp(36, 68),
                                };
                                if col < list_width {
                                    app.next();
                                } else {
                                    let rel_row = row.saturating_sub(3);
                                    let overview_h: u16 = if area.height < 28 { 5 } else { 8 };
                                    let rem_h = area.height.saturating_sub(footer_h).saturating_sub(3).saturating_sub(overview_h);
                                    let task_box_h = rem_h * 55 / 100;
                                    let task_end_y = overview_h + task_box_h;
                                    if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Projects && rel_row < task_end_y {
                                        app.next_project_task();
                                    } else {
                                        app.scroll_preview_down(3);
                                    }
                                }
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if app.show_help {
                                app.help_scroll = app.help_scroll.saturating_sub(2);
                            } else {
                                let list_width = match app.active_tab {
                                    ActiveTab::Work => {
                                        if app.work_tab_mode == crate::ui::app::WorkTabMode::Projects {
                                            area.width * 38 / 100
                                        } else {
                                            area.width * 45 / 100
                                        }
                                    }
                                    ActiveTab::Reader => 0,
                                    ActiveTab::Settings => (area.width * 40 / 100).clamp(38, 65),
                                    _ => (area.width * 38 / 100).clamp(36, 68),
                                };
                                if col < list_width {
                                    app.prev();
                                } else {
                                    let rel_row = row.saturating_sub(3);
                                    let overview_h: u16 = if area.height < 28 { 5 } else { 8 };
                                    let rem_h = area.height.saturating_sub(footer_h).saturating_sub(3).saturating_sub(overview_h);
                                    let task_box_h = rem_h * 55 / 100;
                                    let task_end_y = overview_h + task_box_h;
                                    if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Projects && rel_row < task_end_y {
                                        app.prev_project_task();
                                    } else {
                                        app.scroll_preview_up(3);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::Paste(text) => {
                    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
                    if app.show_new_directive_modal {
                        if app.new_directive_field == 0 {
                            app.new_directive_title.push_str(&normalized);
                        } else if app.new_directive_field == 2 {
                            app.new_directive_scope.push_str(&normalized);
                        } else if app.new_directive_field == 4 {
                            app.new_directive_rule.push_str(&normalized);
                        }
                    } else if app.is_filtering {
                        app.filter_query.push_str(&normalized);
                    } else {
                        // Automatically focus the command dock on paste!
                        app.repl_active = true;
                        // Strip trailing newlines so that pasting text NEVER auto-submits!
                        let clean = normalized.trim_end_matches('\n');
                        app.repl_input.push_str(clean);
                        app.slash_menu_selected_idx = 0;
                    }
                }
                _ => {}
            }

            if app.active_tab == ActiveTab::Work
                && app.work_tab_mode == crate::ui::app::WorkTabMode::Projects
                && app.selected_project_idx != old_proj_idx
            {
                app.refresh_project_tasks(db);
                app.sync_list_states();
            }
        }
    }

    Ok(())
}
