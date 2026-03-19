use std::io::{self, stdout, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use opentui_rust::{
    renderer::Renderer, 
    buffer::OptimizedBuffer, 
    color::Rgba, 
    style::Style,
    text::TextBuffer,
};
use tokio::sync::mpsc;

pub mod app;

use app::{App, AppEvent, AppMode, Message, Role};

pub async fn run() -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    
    // Create renderer
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
            // Run without provider
            let (_, dummy_rx) = tokio::sync::broadcast::channel(1);
            let mut dummy_rx = dummy_rx;
            let result = run_event_loop(&mut renderer, &mut app, None, None, None, &mut dummy_rx).await;
            cleanup_terminal()?;
            return result;
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
    result
}

fn cleanup_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
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
        // Draw the UI
        draw_ui(renderer, app)?;
        renderer.present()?;

        // Handle app events (user input, responses)
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
                        
                        // Save session
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
                            let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
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
        if event::poll(Duration::from_millis(50))? {
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
    
    // Clear background
    buffer.clear(Rgba::from_hex("#0c0c0c").unwrap());
    
    // Draw sidebar
    draw_sidebar(buffer, app, height);
    
    // Draw main content area
    let main_x = 20u32;
    let main_width = width - main_x;
    draw_messages(buffer, app, main_x, 0, main_width, height - 3);
    draw_input(buffer, app, main_x, height - 3, main_width, 3);
    draw_status_bar(buffer, app, main_x, height - 1, main_width, 1);
    
    Ok(())
}

fn draw_sidebar(buffer: &mut OptimizedBuffer, app: &App, height: u32) {
    let sidebar_width = 20u32;
    let bg_color = Rgba::from_hex("#1e1e1e").unwrap();
    let selected_bg = Rgba::from_hex("#264f78").unwrap();
    let text_color = Rgba::from_hex("#cccccc").unwrap();
    let selected_text = Rgba::WHITE;
    
    // Background
    for y in 0..height {
        for x in 0..sidebar_width {
            buffer.set(x as u32, y as u32, opentui_rust::cell::Cell::new(' ', Style::bg(bg_color)));
        }
    }
    
    // Title
    let title = " Sessions ";
    for (i, ch) in title.chars().enumerate() {
        if (i as u32) < sidebar_width - 1 {
            buffer.set((1 + i) as u32 as u32, 0 as u32, opentui_rust::cell::Cell::new(ch, 
                Style::builder().fg(Rgba::from_hex("#569cd6").unwrap()).bg(bg_color).bold().build()));
        }
    }
    
    // Sessions list
    for (idx, session) in app.sessions.iter().enumerate() {
        let y = (2 + idx) as u32;
        if y >= height - 1 {
            break;
        }
        
        let is_selected = idx == app.selected_session;
        let style = if is_selected {
            Style::builder().fg(selected_text).bg(selected_bg).bold().build()
        } else {
            Style::builder().fg(text_color).bg(bg_color).build()
        };
        
        // Draw session name
        let display_text = if session.len() > sidebar_width as usize - 3 {
            format!("> {}", &session[..(sidebar_width as usize) - 5])
        } else {
            format!("  {}", session)
        };
        
        for (i, ch) in display_text.chars().enumerate() {
            if i < sidebar_width as usize {
                buffer.set(1 + i as u32, y as u32, opentui_rust::cell::Cell::new(ch, style));
            }
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
    let bg_color = Rgba::from_hex("#0c0c0c").unwrap();
    
    // Clear message area
    for row in y..y + height {
        for col in x..x + width {
            if col < buffer.width() {
                buffer.set(col as u32, row as u32, opentui_rust::cell::Cell::new(' ', Style::bg(bg_color)));
            }
        }
    }
    
    let mut current_y = y + 1;
    
    for message in &app.messages {
        if current_y >= y + height - 1 {
            break;
        }
        
        let (prefix, color) = match message.role {
            Role::User => ("You", Rgba::from_hex("#4ec9b0").unwrap()),
            Role::Assistant => ("c2", Rgba::from_hex("#569cd6").unwrap()),
            Role::System => ("System", Rgba::from_hex("#dcdcaa").unwrap()),
        };
        
        // Draw prefix
        let prefix_text = format!("{}: ", prefix);
        for (i, ch) in prefix_text.chars().enumerate() {
            buffer.set(x + i as u32, current_y as u32, opentui_rust::cell::Cell::new(ch, 
                Style::builder().fg(color).bg(bg_color).bold().build()));
        }
        
        // Draw message text (wrap if needed)
        let prefix_len = prefix_text.len();
        let content_width = width.saturating_sub(prefix_len as u32 + 2);
        let mut content_x = x + prefix_len as u32;
        
        for line in message.content.lines() {
            if current_y >= y + height - 1 {
                break;
            }
            
            if line.is_empty() {
                current_y += 1;
                continue;
            }
            
            // Simple word wrapping
            let words: Vec<&str> = line.split_whitespace().collect();
            let mut current_line = String::new();
            
            for word in words {
                if current_line.is_empty() {
                    current_line = word.to_string();
                } else if current_line.len() + word.len() + 1 <= content_width as usize {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    // Draw current line
                    for (i, ch) in current_line.chars().enumerate() {
                        if content_x + (i as u32) < x + width {
                            buffer.set(content_x + i as u32, current_y as u32, opentui_rust::cell::Cell::new(ch, 
                                Style::builder().fg(color).bg(bg_color).build()));
                        }
                    }
                    current_y += 1;
                    current_line = word.to_string();
                }
                
                if current_y >= y + height - 1 {
                    break;
                }
            }
            
            // Draw remaining line
            if !current_line.is_empty() && current_y < y + height - 1 {
                for (i, ch) in current_line.chars().enumerate() {
                    if content_x + (i as u32) < x + width {
                        buffer.set(content_x + i as u32, current_y as u32, opentui_rust::cell::Cell::new(ch, 
                            Style::builder().fg(color).bg(bg_color).build()));
                    }
                }
                current_y += 1;
            }
            
            current_y += 1; // Extra space between messages
        }
    }
}

fn draw_input(
    buffer: &mut OptimizedBuffer,
    app: &App,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) {
    let bg_color = if app.mode == AppMode::Input {
        Rgba::from_hex("#0c0c0c").unwrap()
    } else {
        Rgba::from_hex("#1e1e1e").unwrap()
    };
    let border_color = if app.mode == AppMode::Input {
        Rgba::from_hex("#569cd6").unwrap()
    } else {
        Rgba::from_hex("#888888").unwrap()
    };
    let text_color = if app.mode == AppMode::Input {
        Rgba::WHITE
    } else {
        Rgba::from_hex("#888888").unwrap()
    };
    
    // Draw border
    for col in x..x + width {
        buffer.set(col as u32, y as u32, opentui_rust::cell::Cell::new('─', 
            Style::builder().fg(border_color).bg(bg_color).build()));
        if height > 1 {
            buffer.set(col as u32, y + height - 1 as u32, opentui_rust::cell::Cell::new('─', 
                Style::builder().fg(border_color).bg(bg_color).build()));
        }
    }
    for row in y..y + height {
        buffer.set(x as u32, row as u32, opentui_rust::cell::Cell::new('│', 
            Style::builder().fg(border_color).bg(bg_color).build()));
        buffer.set(x + width - 1 as u32, row as u32, opentui_rust::cell::Cell::new('│', 
            Style::builder().fg(border_color).bg(bg_color).build()));
    }
    
    // Corners
    buffer.set(x as u32, y as u32, opentui_rust::cell::Cell::new('┌', 
        Style::builder().fg(border_color).bg(bg_color).build()));
    buffer.set(x + width - 1 as u32, y as u32, opentui_rust::cell::Cell::new('┐', 
        Style::builder().fg(border_color).bg(bg_color).build()));
    if height > 1 {
        buffer.set(x as u32, y + height - 1 as u32, opentui_rust::cell::Cell::new('└', 
            Style::builder().fg(border_color).bg(bg_color).build()));
        buffer.set(x + width - 1 as u32, y + height - 1 as u32, opentui_rust::cell::Cell::new('┘', 
            Style::builder().fg(border_color).bg(bg_color).build()));
    }
    
    // Draw input text
    let max_input_width = width.saturating_sub(4);
    let input_text = if app.input.len() > max_input_width {
        format!("…{}", &app.input[app.input.len() - max_input_width + 1..])
    } else {
        app.input.clone()
    };
    
    for (i, ch) in input_text.chars().enumerate() {
        buffer.set(x + 2 + i as u32, y + 1 as u32, opentui_rust::cell::Cell::new(ch, 
            Style::builder().fg(text_color).bg(bg_color).build()));
    }
    
    // Draw title
    let title = match app.mode {
        AppMode::Input => " Input (Enter to send, Esc to exit) ",
        AppMode::Waiting => " Waiting for response... (Ctrl+C to cancel) ",
    };
    
    for (i, ch) in title.chars().enumerate() {
        let col = x + 2 + i;
        if col < x + width - 2 {
            buffer.set(col as u32, y as u32, opentui_rust::cell::Cell::new(ch, 
                Style::builder().fg(border_color).bg(bg_color).build()));
        }
    }
}

fn draw_status_bar(
    buffer: &mut OptimizedBuffer,
    app: &App,
    x: u32,
    y: u32,
    width: u32,
    _height: u32,
) {
    let bg_color = Rgba::from_hex("#007acc").unwrap();
    let text_color = Rgba::WHITE;
    
    // Background
    for col in x..x + width {
        buffer.set(col as u32, y as u32, opentui_rust::cell::Cell::new(' ', 
            Style::bg(bg_color)));
    }
    
    // Status text
    let status_icon = match app.mode {
        AppMode::Input => "●",
        AppMode::Waiting => "◌",
    };
    let status_text = format!("{} {} | Messages: {}", status_icon, app.status, app.messages.len());
    
    for (i, ch) in status_text.chars().enumerate() {
        if x + i < x + width {
            buffer.set(x + i as u32, y as u32, opentui_rust::cell::Cell::new(ch, 
                Style::builder().fg(text_color).bg(bg_color).build()));
        }
    }
}
