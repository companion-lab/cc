use std::io::{self, stdout};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

pub mod app;
pub mod ui;

use app::App;

pub async fn run() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Create app state
    let mut app = App::new();

    // Run the main event loop
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Exit on Ctrl+C or ESC
                if key.code == KeyCode::Esc || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')) {
                    return Ok(());
                }
                
                // Handle input
                app.handle_key_event(key);
            }
        }
    }
}
