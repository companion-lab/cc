use std::io::{self, stdout};
use std::path::PathBuf;
use std::sync::Arc;

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use opentui_rust::{
    renderer::Renderer,
    buffer::OptimizedBuffer,
    color::Rgba,
    style::Style,
    cell::Cell,
};

pub mod app;

use app::{App, AppEvent, AppMode, Role};

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
        &mut bus_rx
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
    loop {
        draw_ui(renderer, app)?;
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
    loop {
        draw_ui(renderer, app)?;
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

fn draw_ui(renderer: &mut Renderer, app: &App) -> io::Result<()> {
    let buffer = renderer.buffer();
    let width = buffer.width();
    let height = buffer.height();
    
    // Dark theme with accents
    let bg_dark = Rgba::from_hex("#0a0a0a").unwrap();
    let bg_panel = Rgba::from_hex("#0f0f0f").unwrap();
    let bg_input = Rgba::from_hex("#121212").unwrap();
    let accent_blue = Rgba::from_hex("#2d7bd9").unwrap();
    let accent_green = Rgba::from_hex("#2db84d").unwrap();
    let accent_yellow = Rgba::from_hex("#e6b450").unwrap();
    let text_primary = Rgba::from_hex("#e6e6e6").unwrap();
    let text_secondary = Rgba::from_hex("#8e8e93").unwrap();
    let border_subtle = Rgba::from_hex("#2a2a2a").unwrap();
    
    buffer.clear(bg_dark);
    
    // Sidebar
    let sidebar_width = 22u32;
    draw_panel(buffer, 0, 0, sidebar_width, height, bg_panel, border_subtle);
    
    // Sidebar header
    let header = " Sessions ";
    buffer.draw_text(2, 1, header, Style::builder().fg(accent_blue).bg(bg_panel).bold().build());
    buffer.set(sidebar_width - 1, 0, Cell::new('┐', Style::builder().bg(bg_panel).fg(border_subtle).build()));
    buffer.set(sidebar_width - 1, height - 1, Cell::new('┘', Style::builder().bg(bg_panel).fg(border_subtle).build()));
    
    // Sidebar items
    for (idx, session) in app.sessions.iter().enumerate() {
        let y = 3 + idx as u32;
        if y >= height - 1 {
            break;
        }
        
        let is_selected = idx == app.selected_session;
        let text_color = if is_selected { Rgba::WHITE } else { text_secondary };
        let bg_color = if is_selected { Rgba::from_hex("#2d2d30").unwrap() } else { bg_panel };
        
        let label = if is_selected { "▶ " } else { "  " };
        let truncated = if session.len() > 17 { format!("{}…", &session[..16]) } else { session.clone() };
        
        buffer.draw_text(1, y, &format!("{}{}", label, truncated),
            Style::builder().fg(text_color).bg(bg_color).build());
    }
    
    // Main content area
    let main_x = sidebar_width + 1;
    let main_width = width.saturating_sub(main_x).saturating_sub(1);
    draw_panel(buffer, main_x, 0, main_width, height.saturating_sub(4), bg_panel, border_subtle);
    
    // Messages
    draw_messages(buffer, app, main_x, 2, main_width, height.saturating_sub(6));
    
    // Input area
    let input_y = height - 4;
    draw_panel(buffer, main_x, input_y, main_width, 3, bg_input, border_subtle);
    
    // Status bar
    let status_y = height - 1;
    for x in main_x..main_x + main_width {
        buffer.set(x, status_y, Cell::new(' ', Style::builder().bg(accent_blue).build()));
    }
    
    let status_icon = match app.mode {
        AppMode::Input => "●",
        AppMode::Waiting => "◌",
    };
    let status_color = match app.mode {
        AppMode::Input => Rgba::WHITE,
        AppMode::Waiting => Rgba::from_hex("#e6b450").unwrap(),
    };
    
    let status_text = format!(" {} {} | Sessions: {} | Messages: {}", 
        status_icon, app.status, app.sessions.len(), app.messages.len());
    
    buffer.draw_text(main_x + 1, status_y, &status_text,
        Style::builder().fg(status_color).bg(accent_blue).build());
    
    Ok(())
}

fn draw_panel(
    buffer: &mut OptimizedBuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    bg: Rgba,
    border: Rgba,
) {
    buffer.set(x, y, Cell::new('┌', Style::builder().fg(border).bg(bg).build()));
    for col in x + 1..x + width - 1 {
        buffer.set(col, y, Cell::new('─', Style::builder().fg(border).bg(bg).build()));
    }
    buffer.set(x + width - 1, y, Cell::new('┐', Style::builder().fg(border).bg(bg).build()));
    
    buffer.set(x, y + height - 1, Cell::new('└', Style::builder().fg(border).bg(bg).build()));
    for col in x + 1..x + width - 1 {
        buffer.set(col, y + height - 1, Cell::new('─', Style::builder().fg(border).bg(bg).build()));
    }
    buffer.set(x + width - 1, y + height - 1, Cell::new('┘', Style::builder().fg(border).bg(bg).build()));
    
    for row in y + 1..y + height - 1 {
        buffer.set(x, row, Cell::new('│', Style::builder().fg(border).bg(bg).build()));
        buffer.set(x + width - 1, row, Cell::new('│', Style::builder().fg(border).bg(bg).build()));
        
        for col in x + 1..x + width - 1 {
            buffer.set(col, row, Cell::new(' ', Style::builder().bg(bg).build()));
        }
    }
}

fn draw_messages(
    buffer: &mut OptimizedBuffer,
    app: &App,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) {
    let bg_panel = Rgba::from_hex("#0f0f0f").unwrap();
    let accent_green = Rgba::from_hex("#2db84d").unwrap();
    let accent_blue = Rgba::from_hex("#2d7bd9").unwrap();
    let accent_yellow = Rgba::from_hex("#e6b450").unwrap();
    let text_primary = Rgba::from_hex("#e6e6e6").unwrap();
    
    let mut msg_y = y;
    for message in &app.messages {
        if msg_y >= y + height {
            break;
        }
        
        let (prefix, color) = match message.role {
            Role::User => ("You", accent_green),
            Role::Assistant => ("c2", accent_blue),
            Role::System => ("● System", accent_yellow),
        };
        
        let prefix_text = format!("{}: ", prefix);
        buffer.draw_text(x + 1, msg_y, &prefix_text,
            Style::builder().fg(color).bg(bg_panel).bold().build());
        
        let mut lines_drawn = 0u32;
        for line in message.content.lines() {
            if msg_y + lines_drawn >= y + height {
                break;
            }
            
            let content_start = x + 1 + prefix_text.len() as u32;
            let max_width = width.saturating_sub(content_start - x + 2) as usize;
            
            let words: Vec<&str> = line.split_whitespace().collect();
            let mut current_line = String::new();
            let mut current_x = content_start;
            
            for &word in &words {
                let word_len = word.chars().count();
                let can_add = current_line.is_empty() || 
                    (current_line.len() + word_len + 1) <= (width - (current_x - x)) as usize;
                
                if can_add {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                } else {
                    buffer.draw_text(current_x, msg_y + lines_drawn, &current_line,
                        Style::builder().fg(text_primary).bg(bg_panel).build());
                    lines_drawn += 1;
                    current_line = word.to_string();
                    current_x = content_start;
                    
                    if msg_y + lines_drawn >= y + height {
                        break;
                    }
                }
            }
            
            if !current_line.is_empty() && msg_y + lines_drawn < y + height {
                buffer.draw_text(current_x, msg_y + lines_drawn, &current_line,
                    Style::builder().fg(text_primary).bg(bg_panel).build());
                lines_drawn += 1;
            }
            
            if msg_y + lines_drawn >= y + height {
                break;
            }
        }
        
        msg_y += lines_drawn + 1;
    }
}
