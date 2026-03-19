use std::io::{self, stdout};
use std::path::PathBuf;
use std::sync::Arc;

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use opentui_rust::renderer::Renderer;

pub mod app;
pub mod theme;
pub mod ui;

use app::{App, AppEvent, AppMode};
use theme::Theme;
use ui::{
    sidebar::draw_sidebar,
    header::draw_header,
    messages::draw_messages,
    input::{draw_input, draw_input_help},
    status_bar::draw_status_bar,
};

const SIDEBAR_WIDTH: u32 = 22;
const HEADER_HEIGHT: u32 = 1;
const INPUT_HEIGHT: u32 = 3;
const HELP_HEIGHT: u32 = 1;
const STATUS_HEIGHT: u32 = 1;

pub async fn run() -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    // Create renderer with proper dimensions
    let (width, height) = crossterm::terminal::size()?;
    let mut renderer = Renderer::new(width as u32, height as u32)?;

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
            let _ = run_without_provider(&mut renderer, &mut app).await;
            cleanup_terminal()?;
            return Ok(());
        }
    };

    let model = registry.model();

    let data_dir = c2_config::Paths::user_data_dir();
    let db = Arc::new(c2_storage::Db::open(&data_dir).await?);
    let bus = Arc::new(c2_core::bus::Bus::new());
    let mut bus_rx = bus.subscribe();

    // Run the main event loop
    let result = run_event_loop(
        &mut renderer,
        &mut app,
        Some(model),
        Some(db),
        Some(bus),
        &mut bus_rx,
    ).await;

    cleanup_terminal()?;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("TUI Error: {}", e);
            Ok(())
        }
    }
}

fn cleanup_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

async fn run_without_provider(
    renderer: &mut Renderer,
    app: &mut App,
) -> io::Result<()> {
    let theme = Theme::dark();

    loop {
        draw_ui(renderer, app, &theme)?;
        renderer.present()?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => return Ok(()),
                    _ => app.handle_key_event(key),
                }
            }
        }
    }
}

async fn run_event_loop(
    renderer: &mut Renderer,
    app: &mut App,
    model: Option<Arc<dyn c2_provider::LanguageModel>>,
    db: Option<Arc<c2_storage::Db>>,
    bus: Option<Arc<c2_core::bus::Bus>>,
    bus_rx: &mut tokio::sync::broadcast::Receiver<c2_core::Event>,
) -> anyhow::Result<()> {
    let theme = Theme::dark();

    loop {
        draw_ui(renderer, app, &theme)?;
        renderer.present()?;

        // Handle app events
        while let Ok(app_event) = app.rx.try_recv() {
            match app_event {
                AppEvent::UserInput(prompt) => {
                    if let (Some(model), Some(db), Some(bus)) = (&model, &db, &bus) {
                        // Create session
                        let title: String = prompt.chars().take(60).collect();
                        let session = c2_core::session::Session::new(
                            app.cwd.to_string_lossy().to_string(),
                            title,
                        );

                        match session.save(db).await {
                            Ok(_) => {
                                app.current_session_id = Some(session.id.to_string());
                            }
                            Err(e) => {
                                app.add_error(format!("Failed to save session: {}", e));
                                continue;
                            }
                        }

                        app.status = "Processing...".to_string();

                        // Spawn agent processor
                        let model_clone = model.clone();
                        let db_clone = db.clone();
                        let bus_for_events = bus.clone();
                        let bus_for_proc = bus.clone();
                        let session_clone = session.clone();

                        tokio::spawn(async move {
                            let (_abort_tx, abort_rx) = tokio::sync::watch::channel(false);
                            let processor = c2_agent::processor::Processor::new(
                                model_clone,
                                db_clone,
                                bus_for_proc,
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
                }
                AppEvent::ResponseDelta(delta) => {
                    app.append_to_last_assistant_message(&delta);
                }
                AppEvent::ResponseDone => {
                    app.mode = AppMode::Input;
                    app.status = "Ready".to_string();
                }
                AppEvent::Error(error) => {
                    app.add_error(error);
                }
            }
        }

        // Handle bus events (streaming responses)
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
                        if app.mode == AppMode::Input {
                            app.send_message();
                        }
                    }
                    _ => app.handle_key_event(key),
                }
            }
        }
    }
}

fn draw_ui(renderer: &mut Renderer, app: &App, theme: &Theme) -> io::Result<()> {
    let buffer = renderer.buffer();
    let width = buffer.width();
    let height = buffer.height();

    // Clear background
    buffer.clear(theme.bg_dark);

    // Calculate layout
    let main_x = SIDEBAR_WIDTH + 1;
    let main_width = width.saturating_sub(main_x).saturating_sub(1);
    let content_height = height
        .saturating_sub(HEADER_HEIGHT)
        .saturating_sub(INPUT_HEIGHT)
        .saturating_sub(HELP_HEIGHT)
        .saturating_sub(STATUS_HEIGHT);

    // Draw sidebar
    draw_sidebar(
        buffer,
        SIDEBAR_WIDTH,
        height,
        &app.sessions,
        app.selected_session,
        theme,
    );

    // Draw header
    let mode_text = match app.mode {
        AppMode::Input => "Ready",
        AppMode::Waiting => "Processing",
    };
    draw_header(
        buffer,
        main_x,
        0,
        main_width,
        &format!("c2 - {}", app.cwd.file_name().unwrap_or_default().to_string_lossy()),
        mode_text,
        theme,
    );

    // Draw messages
    draw_messages(
        buffer,
        main_x,
        HEADER_HEIGHT,
        main_width,
        content_height,
        &app.messages,
        app.scroll_offset,
        theme,
    );

    // Draw input area
    let input_y = HEADER_HEIGHT + content_height;
    draw_input(
        buffer,
        main_x,
        input_y,
        main_width,
        INPUT_HEIGHT,
        &app.input,
        app.mode == AppMode::Waiting,
        theme,
    );

    // Draw input help
    let help_y = input_y + INPUT_HEIGHT;
    draw_input_help(
        buffer,
        main_x,
        help_y,
        main_width,
        theme,
    );

    // Draw status bar
    let status_y = height - 1;
    draw_status_bar(
        buffer,
        main_x,
        status_y,
        main_width,
        &app.status,
        app.mode == AppMode::Waiting,
        app.messages.len(),
        theme,
    );

    Ok(())
}
