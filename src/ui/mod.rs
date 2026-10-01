pub mod app;
pub mod layout;
pub mod markdown;
pub mod theme;
pub mod views;

use app::{ActiveTab, App};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use layout::{Footer, Header};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::io;
use std::panic;
use std::time::Duration;
use views::{ExploreView, ReaderView, WorkView};
use crate::storage::Database;

pub fn run(db: &Database) -> io::Result<()> {
    // 1. Setup panic hook so terminal is ALWAYS restored safely if something panics
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // 2. Setup terminal in raw mode & alternate screen buffer
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. Initialize App and load initial data
    let mut app = App::new("local_collection", "default_profile");
    app.refresh_data(db);

    // 4. Main Event Loop (Smooth 60 FPS non-blocking polling)
    let res = run_loop(&mut terminal, &mut app);

    // 5. Restore terminal state
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| {
            let area = frame.area();

            // Main vertical layout: Header (3 rows), Content (Remaining), Footer (2 rows)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(2),
                ])
                .split(area);

            Header::render(frame, app, chunks[0]);

            match app.active_tab {
                ActiveTab::Work => WorkView::render(frame, app, chunks[1]),
                ActiveTab::Explore => ExploreView::render(frame, app, chunks[1]),
                ActiveTab::Reader => ReaderView::render(frame, app, chunks[1]),
                ActiveTab::Memory => WorkView::render(frame, app, chunks[1]),
            }

            Footer::render(frame, app, chunks[2]);
        })?;

        // Non-blocking poll with 16ms timeout (~60 FPS response time, 0% CPU when idle)
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                // Global quit shortcuts
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.should_quit = true;
                    break;
                }

                if app.is_filtering {
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
                        KeyCode::Char('1') => app.switch_tab(ActiveTab::Work),
                        KeyCode::Char('2') => app.switch_tab(ActiveTab::Explore),
                        KeyCode::Char('3') => app.switch_tab(ActiveTab::Reader),
                        KeyCode::Tab => app.toggle_pane(),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Up | KeyCode::Char('k') => app.prev(),
                        KeyCode::PageDown => app.page_down(),
                        KeyCode::PageUp => app.page_up(),
                        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => app.page_down(),
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => app.page_up(),
                        KeyCode::Char(' ') if app.active_tab == ActiveTab::Reader => app.page_down(),
                        KeyCode::Enter => app.open_selected(),
                        KeyCode::Esc => app.go_back(),
                        KeyCode::Char('v') => app.toggle_raw_view(),
                        KeyCode::Char('/') => {
                            app.is_filtering = true;
                            app.filter_query.clear();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
