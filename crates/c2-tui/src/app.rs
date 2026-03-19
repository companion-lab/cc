use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Input,
    Waiting,
}

pub struct App {
    pub messages: Vec<Message>,
    pub input: String,
    pub sessions: Vec<String>,
    pub selected_session: usize,
    pub current_session_id: Option<String>,
    pub mode: AppMode,
    pub scroll_offset: usize,
    pub status: String,
    pub tx: UnboundedSender<AppEvent>,
    pub rx: UnboundedReceiver<AppEvent>,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    UserInput(String),
    ResponseDelta(String),
    ResponseDone,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl App {
    pub fn new(cwd: PathBuf) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            messages: vec![Message {
                role: Role::System,
                content: "Welcome to c2 - your AI coding assistant. Type your message and press Enter to send.".to_string(),
            }],
            input: String::new(),
            sessions: vec!["New Chat".to_string()],
            selected_session: 0,
            current_session_id: None,
            mode: AppMode::Input,
            scroll_offset: 0,
            status: "Ready".to_string(),
            tx,
            rx,
            cwd,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match self.mode {
            AppMode::Input => {
                match key.code {
                    KeyCode::Char(c) => {
                        self.input.push(c);
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
                    }
                    KeyCode::Up => {
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                    }
                    KeyCode::Down => {
                        self.scroll_offset += 1;
                    }
                    KeyCode::Left | KeyCode::Right => {}
                    _ => {}
                }
            }
            AppMode::Waiting => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    self.mode = AppMode::Input;
                    self.status = "Cancelled".to_string();
                }
            }
        }
    }

    pub fn send_message(&mut self) {
        if self.input.is_empty() || self.mode != AppMode::Input {
            return;
        }

        let prompt = self.input.clone();
        
        // Add user message
        self.messages.push(Message {
            role: Role::User,
            content: prompt.clone(),
        });
        
        // Clear input and switch to waiting mode
        self.input.clear();
        self.mode = AppMode::Waiting;
        self.status = "Waiting for response...".to_string();
        
        // Send to app event channel
        let _ = self.tx.send(AppEvent::UserInput(prompt));
    }

    pub fn add_system_message(&mut self, content: String) {
        self.messages.push(Message {
            role: Role::System,
            content,
        });
    }

    pub fn add_error(&mut self, error: String) {
        self.messages.push(Message {
            role: Role::System,
            content: format!("Error: {}", error),
        });
        self.mode = AppMode::Input;
        self.status = "Error".to_string();
    }

    pub fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::ResponseDelta(delta) => {
                self.append_to_last_assistant_message(&delta);
            }
            AppEvent::ResponseDone => {
                self.mode = AppMode::Input;
                self.status = "Ready".to_string();
            }
            AppEvent::Error(err) => {
                self.add_error(err);
            }
            _ => {}
        }
    }

    pub fn append_to_last_assistant_message(&mut self, delta: &str) {
        // Check if the last message is from Assistant
        if let Some(last) = self.messages.last_mut() {
            if last.role == Role::Assistant {
                last.content.push_str(delta);
                return;
            }
        }
        
        // Otherwise, create a new assistant message
        self.messages.push(Message {
            role: Role::Assistant,
            content: delta.to_string(),
        });
    }
}
