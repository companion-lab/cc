use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Input,
    Waiting,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogMode {
    None,
    CommandPalette,
    ModelSelect,
    AgentSelect,
    McpManager,
    McpMarketplace,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub shortcut: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub provider_id: String,
    pub model_id: String,
    pub name: String,
    pub description: String,
    pub is_free: bool,
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    pub mode: String,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct McpServerInfo {
    pub name: String,
    pub status: McpStatus,
    pub server_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum McpStatus {
    Connected,
    Disconnected,
    Failed,
    Loading,
}

pub struct AppState {
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
    pub show_thinking: bool,

    // Command palette
    pub commands: Vec<Command>,
    pub command_dialog_selection: usize,

    // Model selection state
    pub current_model: ModelInfo,
    pub available_models: Vec<ModelInfo>,
    pub recent_models: Vec<ModelInfo>,
    pub favorite_models: Vec<ModelInfo>,
    pub model_dialog_selection: usize,

    // Agent selection state
    pub current_agent: AgentInfo,
    pub available_agents: Vec<AgentInfo>,
    pub agent_dialog_selection: usize,

    // MCP state
    pub mcp_servers: HashMap<String, McpServerInfo>,
    pub mcp_dialog_selection: usize,

    // MCP Marketplace state
    pub marketplace_servers: Vec<crate::mcp_marketplace::McpMarketplaceServer>,
    pub marketplace_dialog_selection: usize,

    // Dialog state
    pub dialog_mode: DialogMode,
    pub dialog_filter: String,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    UserInput(String),
    ResponseDelta(String),
    ResponseDone,
    Error(String),
    ModelChanged(ModelInfo),
    AgentChanged(AgentInfo),
    McpToggled(String, bool),
    CommandExecuted(String),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub thinking: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl AppState {
    pub fn new(cwd: PathBuf, model: ModelInfo, agents: Vec<AgentInfo>, mcp_servers: HashMap<String, McpServerInfo>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Load recent models from state file
        let recent_models = Self::load_recent_models();
        let favorite_models = Self::load_favorite_models();

        // Define available commands
        let commands = vec![
            Command { name: "model".to_string(), description: "Switch model".to_string(), shortcut: "Ctrl+M".to_string(), category: "Agent".to_string() },
            Command { name: "agent".to_string(), description: "Switch agent/mode".to_string(), shortcut: "Tab".to_string(), category: "Agent".to_string() },
            Command { name: "research".to_string(), description: "Enter research mode".to_string(), shortcut: "".to_string(), category: "Agent".to_string() },
            Command { name: "thinking".to_string(), description: "Toggle thinking display".to_string(), shortcut: "".to_string(), category: "Display".to_string() },
            Command { name: "mcp".to_string(), description: "Manage installed MCP servers".to_string(), shortcut: "Ctrl+T".to_string(), category: "MCP".to_string() },
            Command { name: "marketplace".to_string(), description: "Browse MCP marketplace".to_string(), shortcut: "".to_string(), category: "MCP".to_string() },
            Command { name: "clear".to_string(), description: "Clear conversation".to_string(), shortcut: "".to_string(), category: "Session".to_string() },
            Command { name: "help".to_string(), description: "Show help".to_string(), shortcut: "".to_string(), category: "System".to_string() },
            Command { name: "quit".to_string(), description: "Exit c2".to_string(), shortcut: "Ctrl+C".to_string(), category: "System".to_string() },
        ];

        Self {
            messages: vec![Message {
                role: Role::System,
                content: "Type / to list commands".to_string(),
                thinking: None,
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
            show_thinking: true,
            commands,
            command_dialog_selection: 0,
            current_model: model,
            available_models: Vec::new(),
            recent_models,
            favorite_models,
            model_dialog_selection: 0,
            current_agent: agents.first().cloned().unwrap_or(AgentInfo {
                name: "build".to_string(),
                description: "Default agent".to_string(),
                model: None,
                mode: "primary".to_string(),
                hidden: false,
            }),
            available_agents: agents,
            agent_dialog_selection: 0,
            mcp_servers,
            mcp_dialog_selection: 0,
            marketplace_servers: crate::mcp_marketplace::McpMarketplace::new().servers().to_vec(),
            marketplace_dialog_selection: 0,
            dialog_mode: DialogMode::None,
            dialog_filter: String::new(),
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        // Handle dialog navigation first
        if self.dialog_mode != DialogMode::None {
            self.handle_dialog_key(key);
            return;
        }

        match self.mode {
            AppMode::Input => {
                match key.code {
                    KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.open_dialog(DialogMode::ModelSelect);
                    }
                    KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.open_dialog(DialogMode::McpManager);
                    }
                    KeyCode::Tab => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            self.cycle_agent(-1);
                        } else {
                            self.cycle_agent(1);
                        }
                    }
                    KeyCode::Char('/') if self.input.is_empty() => {
                        self.open_dialog(DialogMode::CommandPalette);
                    }
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

    fn handle_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.close_dialog();
            }
            KeyCode::Up => {
                self.move_dialog_selection(-1);
            }
            KeyCode::Down => {
                self.move_dialog_selection(1);
            }
            KeyCode::Enter => {
                self.select_dialog_item();
            }
            KeyCode::Char(' ') if self.dialog_mode == DialogMode::McpMarketplace => {
                self.toggle_marketplace_item();
            }
            KeyCode::Char(c) if c != ' ' || self.dialog_mode != DialogMode::McpMarketplace => {
                self.dialog_filter.push(c);
                self.reset_dialog_selection();
            }
            KeyCode::Backspace => {
                self.dialog_filter.pop();
                self.reset_dialog_selection();
            }
            _ => {}
        }
    }

    fn toggle_marketplace_item(&mut self) {
        let filtered = self.get_filtered_marketplace_servers();
        if let Some(server) = filtered.get(self.marketplace_dialog_selection) {
            let id = server.id.clone();
            let mut msg = String::new();
            if let Some(s) = self.marketplace_servers.iter_mut().find(|s| s.id == id) {
                s.enabled = !s.enabled;
                if s.enabled {
                    let cmd_str = s.command.join(" ");
                    msg = format!("Enabled: {} ({})", s.name, cmd_str);
                } else {
                    msg = format!("Disabled: {}", s.name);
                }
            }
            if !msg.is_empty() {
                self.add_system_message(msg);
            }
        }
    }

    fn open_dialog(&mut self, mode: DialogMode) {
        self.dialog_mode = mode;
        self.dialog_filter.clear();
        self.reset_dialog_selection();
    }

    fn close_dialog(&mut self) {
        self.dialog_mode = DialogMode::None;
        self.dialog_filter.clear();
    }

    fn move_dialog_selection(&mut self, delta: i32) {
        let count = match self.dialog_mode {
            DialogMode::CommandPalette => self.get_filtered_commands().len(),
            DialogMode::ModelSelect => self.get_filtered_models().len(),
            DialogMode::AgentSelect => self.get_filtered_agents().len(),
            DialogMode::McpManager => self.get_filtered_mcp_servers().len(),
            DialogMode::McpMarketplace => self.get_filtered_marketplace_servers().len(),
            DialogMode::None => 0,
        };

        if count == 0 {
            return;
        }

        let current = match self.dialog_mode {
            DialogMode::CommandPalette => &mut self.command_dialog_selection,
            DialogMode::ModelSelect => &mut self.model_dialog_selection,
            DialogMode::AgentSelect => &mut self.agent_dialog_selection,
            DialogMode::McpManager => &mut self.mcp_dialog_selection,
            DialogMode::McpMarketplace => &mut self.marketplace_dialog_selection,
            DialogMode::None => return,
        };

        let new_val = (*current as i32 + delta).rem_euclid(count as i32);
        *current = new_val as usize;
    }

    fn reset_dialog_selection(&mut self) {
        match self.dialog_mode {
            DialogMode::CommandPalette => self.command_dialog_selection = 0,
            DialogMode::ModelSelect => self.model_dialog_selection = 0,
            DialogMode::AgentSelect => self.agent_dialog_selection = 0,
            DialogMode::McpManager => self.mcp_dialog_selection = 0,
            DialogMode::McpMarketplace => self.marketplace_dialog_selection = 0,
            DialogMode::None => {}
        }
    }

    fn select_dialog_item(&mut self) {
        match self.dialog_mode {
            DialogMode::CommandPalette => {
                let commands = self.get_filtered_commands();
                if let Some(cmd) = commands.get(self.command_dialog_selection) {
                    let name = cmd.name.clone();
                    self.close_dialog();
                    self.execute_command(&name);
                    return;
                }
            }
            DialogMode::ModelSelect => {
                let models = self.get_filtered_models();
                if let Some(model) = models.get(self.model_dialog_selection) {
                    let model = model.clone();
                    self.set_model(model);
                }
            }
            DialogMode::AgentSelect => {
                let agents = self.get_filtered_agents();
                if let Some(agent) = agents.get(self.agent_dialog_selection) {
                    let agent = agent.clone();
                    self.set_agent(agent);
                }
            }
            DialogMode::McpManager => {
                let servers = self.get_filtered_mcp_servers();
                if let Some(server_name) = servers.get(self.mcp_dialog_selection) {
                    let name = server_name.clone();
                    self.toggle_mcp(&name);
                }
            }
            DialogMode::McpMarketplace => {
                // Toggle the selected marketplace item on Enter
                self.toggle_marketplace_item();
            }
            DialogMode::None => {}
        }
        self.close_dialog();
    }

    pub fn execute_command(&mut self, name: &str) {
        match name {
            "model" => self.open_dialog(DialogMode::ModelSelect),
            "agent" => self.open_dialog(DialogMode::AgentSelect),
            "research" => {
                if let Some(agent) = self.available_agents.iter().find(|a| a.name == "research").cloned() {
                    self.set_agent(agent);
                } else {
                    self.add_system_message("Research mode not configured. Run with a research agent.".to_string());
                }
            }
            "thinking" => {
                self.show_thinking = !self.show_thinking;
                if self.show_thinking {
                    self.add_system_message("Thinking display enabled".to_string());
                } else {
                    self.add_system_message("Thinking display disabled".to_string());
                }
            }
            "mcp" => self.open_dialog(DialogMode::McpManager),
            "marketplace" => self.open_dialog(DialogMode::McpMarketplace),
            "clear" => {
                self.messages.clear();
                self.add_system_message("Conversation cleared.".to_string());
            }
            "help" => {
                self.add_system_message(
                    "Available commands:\n\
                    /model - Switch model (Ctrl+M)\n\
                    /agent - Switch agent/mode (Tab)\n\
                    /research - Enter research mode\n\
                    /thinking - Toggle thinking display\n\
                    /mcp - Manage installed MCP servers (Ctrl+T)\n\
                    /marketplace - Browse MCP marketplace\n\
                    /clear - Clear conversation\n\
                    /help - Show this help\n\
                    /quit - Exit c2 (Ctrl+C)\n\n\
                    Shortcuts:\n\
                    Enter - Send message\n\
                    Tab/Shift+Tab - Cycle agents\n\
                    Space - Toggle MCP in marketplace\n\
                    Esc - Close dialog".to_string()
                );
            }
            "quit" => {
                let _ = self.tx.send(AppEvent::CommandExecuted("quit".to_string()));
            }
            _ => {
                self.add_system_message(format!("Unknown command: {}", name));
            }
        }
    }

    pub fn get_filtered_commands(&self) -> Vec<Command> {
        let filter = self.dialog_filter.to_lowercase();
        let mut commands = self.commands.clone();

        if !filter.is_empty() {
            commands.retain(|c| {
                c.name.to_lowercase().contains(&filter) ||
                c.description.to_lowercase().contains(&filter)
            });
        }

        commands
    }

    pub fn set_model(&mut self, model: ModelInfo) {
        self.current_model = model.clone();
        self.add_to_recent_models(model.clone());
        self.status = format!("Model: {}", model.name);
        let _ = self.tx.send(AppEvent::ModelChanged(model));
    }

    pub fn set_agent(&mut self, agent: AgentInfo) {
        self.current_agent = agent.clone();
        self.status = format!("Agent: {}", agent.name);
        let _ = self.tx.send(AppEvent::AgentChanged(agent));
    }

    pub fn cycle_agent(&mut self, direction: i32) {
        if self.available_agents.is_empty() {
            return;
        }

        let current_idx = self.available_agents
            .iter()
            .position(|a| a.name == self.current_agent.name)
            .unwrap_or(0);

        let new_idx = (current_idx as i32 + direction)
            .rem_euclid(self.available_agents.len() as i32) as usize;

        let agent = self.available_agents[new_idx].clone();
        self.set_agent(agent);
    }

    pub fn toggle_mcp(&mut self, name: &str) {
        if let Some(server) = self.mcp_servers.get_mut(name) {
            let new_status = match server.status {
                McpStatus::Connected => {
                    server.status = McpStatus::Disconnected;
                    false
                }
                McpStatus::Disconnected | McpStatus::Failed => {
                    server.status = McpStatus::Loading;
                    true
                }
                McpStatus::Loading => return,
            };
            let _ = self.tx.send(AppEvent::McpToggled(name.to_string(), new_status));
        }
    }

    pub fn get_filtered_models(&self) -> Vec<ModelInfo> {
        let filter = self.dialog_filter.to_lowercase();
        let mut models = self.available_models.clone();

        if !filter.is_empty() {
            models.retain(|m| {
                m.name.to_lowercase().contains(&filter) ||
                m.model_id.to_lowercase().contains(&filter) ||
                m.provider_id.to_lowercase().contains(&filter)
            });
        }

        // Sort: favorites first, then recent, then alphabetical
        models.sort_by(|a, b| {
            let a_fav = self.favorite_models.iter().any(|f| f.model_id == a.model_id && f.provider_id == a.provider_id);
            let b_fav = self.favorite_models.iter().any(|f| f.model_id == b.model_id && f.provider_id == b.provider_id);
            let a_recent = self.recent_models.iter().any(|r| r.model_id == a.model_id && r.provider_id == a.provider_id);
            let b_recent = self.recent_models.iter().any(|r| r.model_id == b.model_id && r.provider_id == b.provider_id);

            match (a_fav, b_fav) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a_recent, b_recent) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }
        });

        models
    }

    pub fn get_filtered_agents(&self) -> Vec<AgentInfo> {
        let filter = self.dialog_filter.to_lowercase();
        let mut agents: Vec<AgentInfo> = self.available_agents
            .iter()
            .filter(|a| !a.hidden)
            .cloned()
            .collect();

        if !filter.is_empty() {
            agents.retain(|a| {
                a.name.to_lowercase().contains(&filter) ||
                a.description.to_lowercase().contains(&filter)
            });
        }

        agents
    }

    pub fn get_filtered_mcp_servers(&self) -> Vec<String> {
        let filter = self.dialog_filter.to_lowercase();
        let mut servers: Vec<String> = self.mcp_servers.keys().cloned().collect();
        servers.sort();

        if !filter.is_empty() {
            servers.retain(|s| s.to_lowercase().contains(&filter));
        }

        servers
    }

    pub fn get_filtered_marketplace_servers(&self) -> Vec<crate::mcp_marketplace::McpMarketplaceServer> {
        let filter = self.dialog_filter.to_lowercase();
        let mut servers = self.marketplace_servers.clone();

        if !filter.is_empty() {
            servers.retain(|s| {
                s.name.to_lowercase().contains(&filter)
                    || s.description.to_lowercase().contains(&filter)
                    || s.category.to_lowercase().contains(&filter)
            });
        }

        // Sort by installs (descending)
        servers.sort_by(|a, b| b.installs.cmp(&a.installs));
        servers
    }

    fn add_to_recent_models(&mut self, model: ModelInfo) {
        // Remove if already exists
        self.recent_models.retain(|m| !(m.model_id == model.model_id && m.provider_id == model.provider_id));
        // Add to front
        self.recent_models.insert(0, model);
        // Keep only last 10
        self.recent_models.truncate(10);
        Self::save_recent_models(&self.recent_models);
    }

    fn load_recent_models() -> Vec<ModelInfo> {
        let state_file = Self::get_state_file();
        if let Ok(content) = std::fs::read_to_string(&state_file) {
            if let Ok(state) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(recent) = state.get("recent_models").and_then(|v| v.as_array()) {
                    return recent.iter().filter_map(|v| {
                        Some(ModelInfo {
                            provider_id: v.get("provider_id")?.as_str()?.to_string(),
                            model_id: v.get("model_id")?.as_str()?.to_string(),
                            name: v.get("name")?.as_str()?.to_string(),
                            description: String::new(),
                            is_free: v.get("is_free").and_then(|v| v.as_bool()).unwrap_or(false),
                        })
                    }).collect();
                }
            }
        }
        Vec::new()
    }

    fn save_recent_models(models: &[ModelInfo]) {
        let state_file = Self::get_state_file();
        let mut state = serde_json::json!({});
        state["recent_models"] = serde_json::to_value(models.iter().map(|m| {
            serde_json::json!({
                "provider_id": m.provider_id,
                "model_id": m.model_id,
                "name": m.name,
                "is_free": m.is_free,
            })
        }).collect::<Vec<_>>()).unwrap_or_default();

        if let Some(parent) = state_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&state_file, serde_json::to_string_pretty(&state).unwrap_or_default());
    }

    fn load_favorite_models() -> Vec<ModelInfo> {
        let state_file = Self::get_state_file();
        if let Ok(content) = std::fs::read_to_string(&state_file) {
            if let Ok(state) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(favorites) = state.get("favorite_models").and_then(|v| v.as_array()) {
                    return favorites.iter().filter_map(|v| {
                        Some(ModelInfo {
                            provider_id: v.get("provider_id")?.as_str()?.to_string(),
                            model_id: v.get("model_id")?.as_str()?.to_string(),
                            name: v.get("name")?.as_str()?.to_string(),
                            description: String::new(),
                            is_free: v.get("is_free").and_then(|v| v.as_bool()).unwrap_or(false),
                        })
                    }).collect();
                }
            }
        }
        Vec::new()
    }

    fn get_state_file() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".local/share/c2/state.json")
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
            thinking: None,
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
            thinking: None,
        });
    }

    pub fn add_error(&mut self, error: String) {
        self.messages.push(Message {
            role: Role::System,
            content: format!("Error: {}", error),
            thinking: None,
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
                self.status = format!("Ready | {} | {}", self.current_model.name, self.current_agent.name);
            }
            AppEvent::Error(err) => {
                self.add_error(err);
            }
            AppEvent::ModelChanged(model) => {
                self.current_model = model;
            }
            AppEvent::AgentChanged(agent) => {
                self.current_agent = agent;
            }
            AppEvent::McpToggled(name, connected) => {
                if let Some(server) = self.mcp_servers.get_mut(&name) {
                    server.status = if connected { McpStatus::Connected } else { McpStatus::Disconnected };
                }
            }
            AppEvent::CommandExecuted(cmd) => {
                if cmd == "quit" {
                    // Handle quit in the event loop
                }
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
            thinking: None,
        });
    }

    pub fn append_thinking_to_last_assistant_message(&mut self, delta: &str) {
        // Check if the last message is from Assistant
        if let Some(last) = self.messages.last_mut() {
            if last.role == Role::Assistant {
                match &mut last.thinking {
                    Some(thinking) => thinking.push_str(delta),
                    None => last.thinking = Some(delta.to_string()),
                }
                return;
            }
        }

        // Otherwise, create a new assistant message with thinking
        self.messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            thinking: Some(delta.to_string()),
        });
    }
}
