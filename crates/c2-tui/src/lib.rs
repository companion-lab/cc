use std::io::{self, stdout};
use std::path::PathBuf;
use std::sync::Arc;

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub mod app;
pub mod ui;

use app::{App, AppEvent};

pub async fn run() -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = App::new(cwd.clone());

    // Load configuration
    let config = match c2_config::load(&cwd).await {
        Ok(c) => c,
        Err(e) => {
            app.add_system_message(format!("Warning: Could not load config: {}. Using defaults.", e));
            c2_config::Config::default()
        }
    };

    // Initialize provider
    let registry = match c2_provider::ProviderRegistry::from_config(&config).await {
        Ok(r) => r,
        Err(e) => {
            app.add_system_message(format!("Error: {}. Please set your API key in ~/.c2/config.json", e));
            let _ = run_without_provider(&mut terminal, &mut app).await;
            restore_terminal()?;
            return Ok(());
        }
    };

    let model = registry.model();
    let data_dir = c2_config::Paths::user_data_dir();
    let db = Arc::new(c2_storage::Db::open(&data_dir).await?);
    let bus = Arc::new(c2_core::bus::Bus::new());

    // Subscribe to bus events
    let mut bus_rx = bus.subscribe();

    // Run the main event loop
    let result = run_event_loop(&mut terminal, &mut app, model, db, bus, &mut bus_rx).await;

    restore_terminal()?;
    
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("TUI Error: {}", e);
            Ok(())
        }
    }
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

async fn run_without_provider<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Esc {
                    return Ok(());
                }
                app.handle_key_event(key);
            }
        }
    }
}

async fn run_event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    model: Arc<dyn c2_provider::LanguageModel>,
    db: Arc<c2_storage::Db>,
    bus: Arc<c2_core::bus::Bus>,
    bus_rx: &mut tokio::sync::broadcast::Receiver<c2_core::Event>,
) -> anyhow::Result<()> {
    let mut last_status = app.status.clone();
    
    loop {
        // Draw the UI
        terminal.draw(|f| ui::draw(f, app))?;

        // Check for status changes
        if app.status != last_status {
            last_status = app.status.clone();
        }

        // Handle app events (user input, responses)
        while let Ok(app_event) = app.rx.try_recv() {
            match app_event {
                AppEvent::UserInput(prompt) => {
                    // Spawn agent task
                    let title: String = prompt.chars().take(60).collect();
                    let session = c2_core::session::Session::new(
                        app.cwd.to_string_lossy().to_string(),
                        title,
                    );
                    
                    // Save session
                    match session.save(&db).await {
                        Ok(_) => {
                            app.current_session_id = Some(session.id.to_string());
                        }
                        Err(e) => {
                            app.add_error(format!("Failed to save session: {}", e));
                            continue;
                        }
                    }

                    app.status = "Processing...".to_string();
                    
                    // Spawn the agent processor
                    let model_clone = model.clone();
                    let db_clone = db.clone();
                    let bus_clone = bus.clone();
                    let bus_for_events = bus.clone();
                    let session_clone = session.clone();

                    tokio::spawn(async move {
                        let (_abort_tx, abort_rx) = tokio::sync::watch::channel(false);
                        let processor = c2_agent::processor::Processor::new(
                            model_clone,
                            db_clone,
                            bus_clone,
                        );

                        match processor.run(&session_clone, prompt, abort_rx).await {
                            Ok(_) => {
                                bus_for_events.emit(c2_core::Event::AgentDone {
                                    session_id: session_clone.id,
                                });
                            }
                            Err(e) => {
                                bus_for_events.emit(c2_core::Event::AgentError {
                                    session_id: session_clone.id,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });
                }
                AppEvent::ResponseDelta(delta) => {
                    app.append_to_last_assistant_message(&delta);
                }
                AppEvent::ResponseDone => {
                    app.mode = app::AppMode::Input;
                    app.status = "Ready".to_string();
                }
                AppEvent::Error(error) => {
                    app.add_error(error);
                }
            }
        }

        // Handle bus events (streaming responses from agent)
        loop {
            match bus_rx.try_recv() {
                Ok(c2_core::Event::TextDelta { delta, .. }) => {
                    app.append_to_last_assistant_message(&delta);
                }
                Ok(c2_core::Event::AgentDone { .. }) => {
                    app.handle_app_event(AppEvent::ResponseDone);
                }
                Ok(c2_core::Event::AgentError { error, .. }) => {
                    app.add_error(error);
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
                _ => {}
            }
        }

        // Handle keyboard input
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => return Ok(()),
                    KeyCode::Enter => {
                        if app.mode == app::AppMode::Input {
                            app.send_message();
                        }
                    }
                    _ => app.handle_key_event(key),
                }
            }
        }
    }
}
