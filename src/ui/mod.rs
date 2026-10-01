pub mod app;
pub mod layout;
pub mod markdown;
pub mod theme;
pub mod views;

use app::{ActiveTab, App, ExploreTreeItem};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind},
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
use views::{DirectivesView, ExploreView, HelpModal, ReaderView, SessionsView, SettingsView, WorkView};
use crate::storage::Database;

pub fn run(root: &Path, db: &Database, collection_id: &str, profile_id: &str) -> io::Result<()> {
    // 1. Setup panic hook so terminal is ALWAYS restored safely if something panics
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
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
    if app.mouse_capture {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
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

            if app.show_help {
                HelpModal::render(frame, app, area);
            }
        })?;

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

                    if app.show_help {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                                app.show_help = false;
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
                    } else {
                        match key.code {
                            KeyCode::Char('q') => app.should_quit = true,
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
                                    app.status_message = Some("Mouse Capture: ON (Click to Navigate)".to_string());
                                } else {
                                    let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
                                    let _ = terminal.backend_mut().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l");
                                    let _ = terminal.backend_mut().flush();
                                    app.status_message = Some("Mouse Capture: OFF (Terminal drag-selection enabled. Or hold Shift for instant bypass)".to_string());
                                }
                            }
                            KeyCode::Char('T') => {
                                app.next_theme();
                            }
                            KeyCode::Char('e') | KeyCode::Char('E') if app.active_tab == ActiveTab::Sessions => {
                                app.toggle_scoring_methodology();
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
                                }
                            }
                            KeyCode::Enter => {
                                if app.active_tab == ActiveTab::Settings {
                                    let _ = app.save_settings(root);
                                } else {
                                    app.open_selected();
                                }
                            }
                            KeyCode::Esc => app.go_back(),
                            KeyCode::Char('t') if app.active_tab == ActiveTab::Explore => {
                                app.toggle_explore_tree_mode();
                            }
                            KeyCode::Char('v') => app.toggle_raw_view(),
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                if app.active_tab == ActiveTab::Explore {
                                    app.next_category(db);
                                } else if app.active_tab == ActiveTab::Directives {
                                    app.next_directive_category(db);
                                }
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') if app.active_tab == ActiveTab::Directives => {
                                let _ = app.retire_selected_directive(root, db);
                            }
                            KeyCode::Char('a') | KeyCode::Char('A') if app.active_tab == ActiveTab::Directives => {
                                app.status_message = Some("To create a directive: run 'hyperkb directive new' or MCP tool 'draft_directive'".to_string());
                            }
                            KeyCode::Char('/') => {
                                app.is_filtering = true;
                                app.filter_query.clear();
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

                    // If Help modal is currently open, isolate all mouse actions to the modal overlay.
                    if app.show_help {
                        match mouse.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                let modal = HelpModal::modal_area(area);
                                let inside = col >= modal.x
                                    && col < modal.x + modal.width
                                    && row >= modal.y
                                    && row < modal.y + modal.height;

                                if !inside {
                                    // Clicking outside the help modal dismisses it safely!
                                    // Under no circumstances should this click trigger footer [q] quit or underlying view clicks!
                                    app.show_help = false;
                                } else {
                                    // If clicked on the top title bar or bottom bar of modal:
                                    let is_top_bar = row == modal.y;
                                    let is_bottom_bar = row == modal.y + modal.height.saturating_sub(1);
                                    if is_top_bar || is_bottom_bar {
                                        app.show_help = false;
                                    }
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
                            if app.status_message.is_some() {
                                app.status_message = None;
                            }

                            // 1. Header clicks (row 1: tabs, row 3: taxonomy/category filter pills)
                            if row <= 4 {
                                Header::handle_click(app, db, col, row);
                            }
                            // 2. Main content clicks
                            else if row >= 5 && row < area.height.saturating_sub(2) {
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
                                                if item_idx < 6 {
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
                            // 3. Footer clicks
                            else if row >= area.height.saturating_sub(2) && col >= area.width.saturating_sub(12) {
                                app.should_quit = true;
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if app.show_help {
                                app.help_scroll += 2;
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
