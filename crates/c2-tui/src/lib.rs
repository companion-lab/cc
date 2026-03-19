use std::io::{self, stdout};
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use opentui_rust::renderer::Renderer;

pub mod app;
pub mod theme;
pub mod ui;
pub mod models_fetcher;

use app::{AppState, AppEvent, AppMode, DialogMode, ModelInfo, AgentInfo, McpServerInfo, McpStatus};
use theme::Theme;
use ui::{
    sidebar::draw_sidebar,
    header::draw_header,
    messages::draw_messages,
    input::{draw_input, draw_input_help},
    status_bar::draw_status_bar,
    dialog::draw_dialog,
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

    // Load configuration
    let config = match c2_config::load(&cwd).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not load config: {}. Using defaults.", e);
            c2_config::Config::default()
        }
    };

    // Initialize provider
    let registry = match c2_provider::ProviderRegistry::from_config(&config).await {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("Warning: {}. Running without provider.", e);
            None
        }
    };

    // Build model info from config
    let current_model = ModelInfo {
        provider_id: config.provider.as_ref().map(|p| p.id.clone()).unwrap_or_else(|| "openai-compatible".to_string()),
        model_id: config.model.clone().unwrap_or_else(|| "unknown".to_string()),
        name: config.model.clone().unwrap_or_else(|| "Default Model".to_string()),
        description: "Configured model".to_string(),
        is_free: false,
    };

    // Build available models list from provider registry
    let mut available_models = Vec::new();
    if let Some(reg) = &registry {
        let model = reg.model();
        available_models.push(ModelInfo {
            provider_id: model.provider_id().to_string(),
            model_id: model.id().to_string(),
            name: model.id().to_string(),
            description: format!("Context: {}k tokens", model.context_length() / 1000),
            is_free: false,
        });
    }

    // Build agents from config
    let mut available_agents = vec![
        AgentInfo {
            name: "build".to_string(),
            description: "Primary agent - full access to all tools".to_string(),
            model: None,
            mode: "primary".to_string(),
            hidden: false,
        },
        AgentInfo {
            name: "plan".to_string(),
            description: "Planning mode - read-only, creates implementation plans".to_string(),
            model: None,
            mode: "primary".to_string(),
            hidden: false,
        },
    ];

    // Add custom agents from config
    for agent in &config.agents {
        let mode_str = match &agent.mode {
            c2_config::AgentMode::Primary => "primary",
            c2_config::AgentMode::Subagent => "subagent",
            c2_config::AgentMode::All => "all",
        };
        available_agents.push(AgentInfo {
            name: agent.name.clone(),
            description: agent.system_prompt.as_ref()
                .map(|p| p.chars().take(50).collect())
                .unwrap_or_else(|| "Custom agent".to_string()),
            model: agent.model.clone(),
            mode: mode_str.to_string(),
            hidden: false,
        });
    }

    // Build MCP servers from config
    let mut mcp_servers = HashMap::new();
    for (name, server_config) in &config.mcp {
        let (server_type, status) = match server_config {
            c2_config::McpServerConfig::Stdio { .. } => ("stdio", McpStatus::Disconnected),
            c2_config::McpServerConfig::Sse { .. } => ("sse", McpStatus::Disconnected),
            c2_config::McpServerConfig::Http { .. } => ("http", McpStatus::Disconnected),
        };
        mcp_servers.insert(name.clone(), McpServerInfo {
            name: name.clone(),
            status,
            server_type: server_type.to_string(),
        });
    }

    let mut app = AppState::new(cwd.clone(), current_model, available_agents, mcp_servers);
    app.available_models = available_models;

    let model = registry.map(|r| r.model());

    let data_dir = c2_config::Paths::user_data_dir();
    let db = match c2_storage::Db::open(&data_dir).await {
        Ok(db) => Some(Arc::new(db)),
        Err(e) => {
            app.add_system_message(format!("Warning: Could not open database: {}", e));
            None
        }
    };
    let bus = Arc::new(c2_core::bus::Bus::new());
    let mut bus_rx = bus.subscribe();

    // Run the main event loop
    let result = run_event_loop(
        &mut renderer,
        &mut app,
        model,
        db,
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
    app: &mut AppState,
) -> io::Result<()> {
    let theme = Theme::dark();

    loop {
        draw_ui(renderer, app, &theme)?;
        renderer.present()?;

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Resize(new_width, new_height) => {
                    *renderer = Renderer::new(new_width as u32, new_height as u32)?;
                    renderer.buffer().clear(theme.bg_dark);
                    renderer.present()?;
                    continue;
                }
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Esc => return Ok(()),
                        _ => app.handle_key_event(key),
                    }
                }
                _ => {}
            }
        }
    }
}

async fn run_event_loop(
    renderer: &mut Renderer,
    app: &mut AppState,
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

                        app.status = format!("Processing with {}...", app.current_model.name);

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
                    app.status = format!("Ready | {} | {}", app.current_model.name, app.current_agent.name);
                }
                AppEvent::Error(error) => {
                    app.add_error(error);
                }
                AppEvent::ModelChanged(model) => {
                    app.current_model = model;
                }
                AppEvent::AgentChanged(agent) => {
                    app.current_agent = agent;
                }
                AppEvent::McpToggled(name, connected) => {
                    if let Some(server) = app.mcp_servers.get_mut(&name) {
                        server.status = if connected { McpStatus::Connected } else { McpStatus::Disconnected };
                    }
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

        // Handle keyboard input and resize
        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Resize(new_width, new_height) => {
                    // Recreate renderer with new dimensions
                    *renderer = Renderer::new(new_width as u32, new_height as u32)?;
                    // Clear and present immediately to avoid leftover characters
                    renderer.buffer().clear(theme.bg_dark);
                    renderer.present()?;
                    continue;
                }
                Event::Key(key) => {
                    // Handle dialog close on Esc
                    if key.code == KeyCode::Esc && app.dialog_mode != DialogMode::None {
                        app.handle_key_event(key);
                        continue;
                    }

                    // Handle exit when no dialog
                    if key.code == KeyCode::Esc && app.dialog_mode == DialogMode::None {
                        return Ok(());
                    }

                    // Handle Ctrl+C exit
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        return Ok(());
                    }

                    // Handle Enter for sending message
                    if key.code == KeyCode::Enter && app.mode == AppMode::Input && app.dialog_mode == DialogMode::None {
                        app.send_message();
                        continue;
                    }

                    app.handle_key_event(key);
                }
                _ => {}
            }
        }
    }
}

fn draw_ui(renderer: &mut Renderer, app: &AppState, theme: &Theme) -> io::Result<()> {
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

    // Draw header with model and agent info
    let status_indicator = match app.mode {
        AppMode::Input => format!("Ready | {} | {}", app.current_model.name, app.current_agent.name),
        AppMode::Waiting => format!("Processing | {} | {}", app.current_model.name, app.current_agent.name),
    };
    draw_header(
        buffer,
        main_x,
        0,
        main_width,
        &format!("c2 - {}", app.cwd.file_name().unwrap_or_default().to_string_lossy()),
        &status_indicator,
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

    // Draw dialog if open
    if app.dialog_mode != DialogMode::None {
        draw_dialog(buffer, app, theme);
    }

    Ok(())
}
