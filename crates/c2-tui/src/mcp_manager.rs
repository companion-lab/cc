use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub command: Vec<String>,
    pub env: HashMap<String, String>,
    pub enabled: bool,
}

pub struct McpManager {
    config_path: PathBuf,
    servers: Vec<McpServerConfig>,
}

impl McpManager {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let config_path = PathBuf::from(home).join(".config/c2/mcp.json");

        let mut manager = Self {
            config_path,
            servers: Vec::new(),
        };

        manager.load();
        manager
    }

    pub fn load(&mut self) {
        if let Ok(content) = std::fs::read_to_string(&self.config_path) {
            if let Ok(servers) = serde_json::from_str::<Vec<McpServerConfig>>(&content) {
                self.servers = servers;
            }
        }
    }

    pub fn save(&self) {
        if let Some(parent) = self.config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(&self.servers).unwrap_or_default();
        let _ = std::fs::write(&self.config_path, content);
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.servers.iter().any(|s| s.id == id && s.enabled)
    }

    pub fn enable(&mut self, id: &str, name: &str, command: Vec<String>, env: HashMap<String, String>) {
        if let Some(server) = self.servers.iter_mut().find(|s| s.id == id) {
            server.enabled = true;
            server.command = command;
            server.env = env;
        } else {
            self.servers.push(McpServerConfig {
                id: id.to_string(),
                name: name.to_string(),
                command,
                env,
                enabled: true,
            });
        }
        self.save();
    }

    pub fn disable(&mut self, id: &str) {
        if let Some(server) = self.servers.iter_mut().find(|s| s.id == id) {
            server.enabled = false;
        }
        self.save();
    }

    pub fn toggle(&mut self, id: &str, name: &str, command: Vec<String>, env: HashMap<String, String>) -> bool {
        if self.is_enabled(id) {
            self.disable(id);
            false
        } else {
            self.enable(id, name, command, env);
            true
        }
    }

    pub fn get_enabled(&self) -> Vec<&McpServerConfig> {
        self.servers.iter().filter(|s| s.enabled).collect()
    }

    pub fn get_all(&self) -> &[McpServerConfig] {
        &self.servers
    }
}
