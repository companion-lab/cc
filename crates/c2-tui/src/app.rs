use crossterm::event::{KeyCode, KeyEvent};

#[derive(Default)]
pub struct App {
    /// Holds the messages sent and received
    pub messages: Vec<String>,
    /// The user's input buffer
    pub input: String,
    /// Selected session title or id
    pub current_session: Option<String>,
    /// Sidebar sessions list
    pub sessions: Vec<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            messages: vec![
                "Assistant: Hello! I am c2. How can I help you?".to_string(),
            ],
            input: String::new(),
            current_session: Some("Session #1".to_string()),
            sessions: vec!["Session #1".to_string(), "New Session".to_string()],
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    let msg = format!("User: {}", self.input.clone());
                    self.messages.push(msg);
                    self.input.clear();
                }
            }
            _ => {}
        }
    }
}
