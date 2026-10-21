pub mod actions;
pub mod state;
pub mod view;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use state::TuiState;
use std::io::stdout;
use std::time::Duration;
use view::TuiView;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Enter Terminal Alternate Screen & Raw Mode
    terminal::enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, cursor::Hide)?;

    let mut state = TuiState::default();
    state.refresh().await;

    let tick_rate = Duration::from_millis(1000);
    let mut last_tick = std::time::Instant::now();

    // 2. Main Event Loop
    let run_res = run_loop(&mut state, tick_rate, &mut last_tick).await;

    // 3. Graceful Restore Terminal
    let _ = execute!(stdout(), cursor::Show, LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();

    run_res
}

async fn run_loop(
    state: &mut TuiState,
    tick_rate: Duration,
    last_tick: &mut std::time::Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Render current frame
        TuiView::render(state)?;

        // Calculate poll timeout
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        let old_tab = state.active_tab;
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                            KeyCode::Tab | KeyCode::Right => {
                                state.active_tab = state.active_tab.next();
                            }
                            KeyCode::BackTab | KeyCode::Left => {
                                state.active_tab = state.active_tab.prev();
                            }
                            KeyCode::Char('1') => state.active_tab = state::ActiveTab::Overview,
                            KeyCode::Char('2') => state.active_tab = state::ActiveTab::Bridge,
                            KeyCode::Char('3') => state.active_tab = state::ActiveTab::Browser,
                            KeyCode::Char('4') => state.active_tab = state::ActiveTab::Automa,
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                state.status_message = "Refreshing state...".to_string();
                                state.refresh().await;
                                state.status_message = "Refreshed live state.".to_string();
                            }
                            KeyCode::Char('b') | KeyCode::Char('B') => {
                                actions::toggle_bridge(state).await;
                            }
                            KeyCode::Char('k') | KeyCode::Char('K') => {
                                actions::kill_browsers(state).await;
                            }
                            _ => {}
                        }
                        if state.active_tab != old_tab {
                            let _ = execute!(stdout(), terminal::Clear(terminal::ClearType::All));
                        }
                    }
                }
                Event::Resize(_, _) => {
                    let _ = execute!(stdout(), terminal::Clear(terminal::ClearType::All));
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            state.refresh().await;
            *last_tick = std::time::Instant::now();
        }
    }

    Ok(())
}
