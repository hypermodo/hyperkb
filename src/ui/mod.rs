pub mod app;
pub mod layout;
pub mod markdown;
pub mod theme;
pub mod views;

use app::{ActiveTab, App, ExploreTreeItem};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, KeyboardEnhancementFlags, MouseButton, MouseEventKind, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use layout::{Footer, Header};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::Block,
    Terminal,
};
use std::io::{self, Write};
use std::panic;
use std::path::Path;
use std::time::Duration;
use views::{ActionPaletteModal, DirectivesView, ExploreView, HelpModal, IssueGrantModal, NewDirectiveModal, ReaderView, SessionsView, SettingsView, WorkView};
use crate::storage::Database;

pub fn run(root: &Path, db: &Database, collection_id: &str, profile_id: &str) -> io::Result<()> {
    // 1. Setup panic hook so terminal is ALWAYS restored safely if something panics
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture, PopKeyboardEnhancementFlags);
        let _ = io::stdout().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
        let _ = io::stdout().flush();
        original_hook(panic_info);
    }));

    // 2. Initialize App and load initial data
    let mut app = App::new(collection_id, profile_id);
    app.refresh_data(db);

    // 3. Setup terminal in raw mode & alternate screen buffer with mouse capture conditional on settings
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    );
    if app.mouse_capture {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let _ = stdout.flush();
    } else {
        execute!(stdout, EnterAlternateScreen)?;
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
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
    while !app.should_quit {
        let mut highlighted_text: Option<String> = None;
        terminal.draw(|frame| {
            let area = frame.area();
            let t = &app.theme;

            // Fill entire frame with active theme background color and primary text color
            let bg_block = Block::default().style(Style::default().bg(t.bg()).fg(t.text_primary()));
            frame.render_widget(bg_block, area);

            // Main vertical layout: Header (5 rows with generous breathing room), Content (Remaining), Footer (2 rows)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5),
                    Constraint::Min(5),
                    Constraint::Length(2),
                ])
                .split(area);

            Header::render(frame, app, chunks[0]);

            match app.active_tab {
                ActiveTab::Work => WorkView::render(frame, app, chunks[1]),
                ActiveTab::Explore => ExploreView::render(frame, app, chunks[1]),
                ActiveTab::Directives => DirectivesView::render(frame, app, chunks[1]),
                ActiveTab::Sessions => SessionsView::render(frame, app, chunks[1]),
                ActiveTab::Settings => SettingsView::render(frame, app, chunks[1]),
                ActiveTab::Reader => ReaderView::render(frame, app, chunks[1]),
            }

            Footer::render(frame, app, chunks[2]);

            if app.show_action_palette {
                ActionPaletteModal::render(frame, app, area);
            } else if app.show_new_directive_modal {
                NewDirectiveModal::render(frame, app, area);
            } else if app.show_issue_grant_modal {
                IssueGrantModal::render(frame, app, area);
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

        // Non-blocking poll with 16ms timeout (~60 FPS response time, 0% CPU when idle)
        if event::poll(Duration::from_millis(16))? {
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

                    if app.show_action_palette {
                        match key.code {
                            KeyCode::Esc => {
                                app.show_action_palette = false;
                            }
                            KeyCode::Down | KeyCode::Tab => {
                                let actions = app.filtered_actions();
                                if !actions.is_empty() {
                                    app.action_palette_selected_idx = (app.action_palette_selected_idx + 1) % actions.len();
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                let actions = app.filtered_actions();
                                if !actions.is_empty() {
                                    app.action_palette_selected_idx = (app.action_palette_selected_idx + 1) % actions.len();
                                }
                            }
                            KeyCode::Up | KeyCode::BackTab => {
                                let actions = app.filtered_actions();
                                if !actions.is_empty() {
                                    if app.action_palette_selected_idx == 0 {
                                        app.action_palette_selected_idx = actions.len() - 1;
                                    } else {
                                        app.action_palette_selected_idx -= 1;
                                    }
                                }
                            }
                            KeyCode::Char('p') | KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                let actions = app.filtered_actions();
                                if !actions.is_empty() {
                                    if app.action_palette_selected_idx == 0 {
                                        app.action_palette_selected_idx = actions.len() - 1;
                                    } else {
                                        app.action_palette_selected_idx -= 1;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                let actions = app.filtered_actions();
                                if let Some(item) = actions.get(app.action_palette_selected_idx) {
                                    let id = item.id;
                                    match app.execute_action_palette_item(id, db) {
                                        Ok(msg) => app.status_message = Some(msg),
                                        Err(err) => app.status_message = Some(format!("Error: {}", err)),
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                app.action_palette_query.pop();
                                app.action_palette_selected_idx = 0;
                            }
                            KeyCode::Char(c) => {
                                app.action_palette_query.push(c);
                                app.action_palette_selected_idx = 0;
                            }
                            _ => {}
                        }
                    } else if app.show_new_directive_modal {
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
                    } else if app.active_tab == ActiveTab::Work && app.repl_active {
                        match key.code {
                            KeyCode::Esc => {
                                if app.repl_input.starts_with('/') && !app.repl_input.is_empty() {
                                    app.repl_input.clear();
                                    app.slash_menu_selected_idx = 0;
                                } else {
                                    app.repl_active = false;
                                }
                            }
                            KeyCode::Enter => {
                                if key.modifiers.contains(KeyModifiers::SHIFT)
                                    || key.modifiers.contains(KeyModifiers::ALT)
                                    || key.modifiers.contains(KeyModifiers::CONTROL)
                                {
                                    // Multi-line support: Shift+Enter, Option+Enter, or Ctrl+Enter adds newline
                                    app.repl_input.push('\n');
                                } else if app.repl_input.ends_with('\\') {
                                    // Trailing backslash continuation (shell standard)
                                    app.repl_input.pop();
                                    app.repl_input.push('\n');
                                } else {
                                    let filtered = app.filtered_slash_commands();
                                    if app.repl_input.starts_with('/') && !filtered.is_empty() {
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
                            KeyCode::Tab => {
                                let filtered = app.filtered_slash_commands();
                                if app.repl_input.starts_with('/') && !filtered.is_empty() {
                                    let sel = app.slash_menu_selected_idx.min(filtered.len().saturating_sub(1));
                                    app.repl_input = format!("/{}", filtered[sel].name);
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
                                app.repl_input.push(c);
                                app.slash_menu_selected_idx = 0;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => app.should_quit = true,
                            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.show_action_palette = true;
                                app.action_palette_query.clear();
                                app.action_palette_selected_idx = 0;
                            }
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                match app.open_active_document_in_editor() {
                                    Ok(msg) => app.status_message = Some(msg),
                                    Err(err) => app.status_message = Some(err),
                                }
                            }
                            KeyCode::Char('?') | KeyCode::F(1) => app.toggle_help(),
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
                            KeyCode::Tab => app.toggle_pane(),
                            KeyCode::Down | KeyCode::Char('j') => app.next(),
                            KeyCode::Up | KeyCode::Char('k') => app.prev(),
                            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('-') if app.active_tab == ActiveTab::Settings => {
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
                            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('+') | KeyCode::Char('=') if app.active_tab == ActiveTab::Settings => {
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
                            KeyCode::PageDown => app.page_down(),
                            KeyCode::PageUp => app.page_up(),
                            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => app.page_down(),
                            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => app.page_up(),
                            KeyCode::Char(' ') => {
                                if app.active_tab == ActiveTab::Reader {
                                    app.page_down();
                                } else if app.active_tab == ActiveTab::Explore && app.explore_tree_mode {
                                    app.open_selected();
                                } else if app.active_tab == ActiveTab::Settings {
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
                                } else {
                                    app.show_action_palette = true;
                                    app.action_palette_query.clear();
                                    app.action_palette_selected_idx = 0;
                                }
                            }
                            KeyCode::Enter => {
                                if app.active_tab == ActiveTab::Settings {
                                    let _ = app.save_settings(root);
                                } else if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Console {
                                    app.repl_active = true;
                                } else {
                                    app.open_selected();
                                }
                            }
                            KeyCode::Esc => app.go_back(),
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
                                    app.work_tab_mode = crate::ui::app::WorkTabMode::Console;
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
                                if app.active_tab == ActiveTab::Work {
                                    app.work_tab_mode = crate::ui::app::WorkTabMode::Console;
                                    app.repl_active = true;
                                    app.repl_input = "/".to_string();
                                    app.slash_menu_selected_idx = 0;
                                } else {
                                    app.is_filtering = true;
                                    app.filter_query.clear();
                                }
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

                    // If Action Palette modal is open, dismiss when clicking outside or select/execute when clicking inside
                    if app.show_action_palette {
                        let modal = ActionPaletteModal::modal_area(area);
                        let inside = col >= modal.x
                            && col < modal.x + modal.width
                            && row >= modal.y
                            && row < modal.y + modal.height;

                        match mouse.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                if !inside {
                                    app.show_action_palette = false;
                                } else {
                                    let list_start_y = modal.y + 5;
                                    let actions = app.filtered_actions();
                                    if row >= list_start_y && (row as usize) < list_start_y as usize + actions.len() {
                                        let clicked_idx = (row - list_start_y) as usize;
                                        if clicked_idx < actions.len() {
                                            if app.action_palette_selected_idx == clicked_idx {
                                                let id = actions[clicked_idx].id;
                                                match app.execute_action_palette_item(id, db) {
                                                    Ok(msg) => app.status_message = Some(msg),
                                                    Err(err) => app.status_message = Some(format!("Error: {}", err)),
                                                }
                                            } else {
                                                app.action_palette_selected_idx = clicked_idx;
                                            }
                                        }
                                    }
                                }
                            }
                            MouseEventKind::ScrollDown => {
                                let actions = app.filtered_actions();
                                if !actions.is_empty() {
                                    app.action_palette_selected_idx = (app.action_palette_selected_idx + 1) % actions.len();
                                }
                            }
                            MouseEventKind::ScrollUp => {
                                let actions = app.filtered_actions();
                                if !actions.is_empty() {
                                    if app.action_palette_selected_idx == 0 {
                                        app.action_palette_selected_idx = actions.len() - 1;
                                    } else {
                                        app.action_palette_selected_idx -= 1;
                                    }
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }

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

                                // 1. Header clicks (row 1: tabs, row 3: taxonomy/category filter pills)
                                if row <= 4 {
                                    Header::handle_click(app, db, col, row);
                                }
                                // 2. Main content clicks
                                else if row >= 5 && row < area.height.saturating_sub(2) {
                                    if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Console {
                                        let input_lines = app.repl_input.lines().count().max(1);
                                        let prompt_h = (input_lines as u16 + 8).clamp(10, 16);
                                        let filtered_slash = app.filtered_slash_commands();
                                        let prompt_y = area.height.saturating_sub(2).saturating_sub(prompt_h);

                                        if app.repl_active && !filtered_slash.is_empty() {
                                            let slash_h = (filtered_slash.len() as u16 + 2).min(7);
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
                                        } else {
                                            app.repl_active = false;
                                        }
                                    } else {
                                        let list_width = match app.active_tab {
                                            ActiveTab::Work => area.width * 45 / 100,
                                            ActiveTab::Reader => 0,
                                            ActiveTab::Settings => (area.width * 40 / 100).clamp(38, 65),
                                            _ => (area.width * 38 / 100).clamp(36, 68),
                                        };

                                        if col < list_width {
                                            app.focused_pane = crate::ui::app::FocusedPane::List;
                                            let rel_row = row.saturating_sub(5);
                                            if rel_row >= 2 {
                                                match app.active_tab {
                                                    ActiveTab::Work => {
                                                        let item_idx = ((rel_row - 2) / 3) as usize;
                                                        if item_idx < app.active_risks.len() {
                                                            app.selected_risk_idx = item_idx;
                                                        }
                                                    }
                                                    ActiveTab::Directives => {
                                                        let item_idx = ((rel_row - 2) / 3) as usize;
                                                        if item_idx < app.directives.len() {
                                                            app.selected_directive_idx = item_idx;
                                                        }
                                                    }
                                                    ActiveTab::Sessions => {
                                                        let item_idx = ((rel_row - 2) / 3) as usize;
                                                        if item_idx < app.sessions.len() {
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
                                                            let tree_idx = ((rel_row - 2) / 2) as usize;
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
                                                            let item_idx = ((rel_row - 2) / 3) as usize;
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
                                        }
                                    }
                                }
                                // 3. Footer clicks
                                else if row >= area.height.saturating_sub(2) && col >= area.width.saturating_sub(12) {
                                    app.should_quit = true;
                                }
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if app.show_help {
                                app.help_scroll += 2;
                            } else if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Console {
                                app.diagnostic_scroll += 2;
                            } else {
                                let list_width = (area.width * 38 / 100).clamp(36, 68);
                                if col < list_width {
                                    app.next();
                                } else {
                                    app.page_down();
                                }
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if app.show_help {
                                app.help_scroll = app.help_scroll.saturating_sub(2);
                            } else if app.active_tab == ActiveTab::Work && app.work_tab_mode == crate::ui::app::WorkTabMode::Console {
                                app.diagnostic_scroll = app.diagnostic_scroll.saturating_sub(2);
                            } else {
                                let list_width = (area.width * 38 / 100).clamp(36, 68);
                                if col < list_width {
                                    app.prev();
                                } else {
                                    app.page_up();
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
