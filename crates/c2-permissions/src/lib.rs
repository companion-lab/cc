use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionMode {
    Allow,
    Deny,
    Ask,
}

pub struct PermissionGate {
    /// tool name → mode
    rules: HashMap<String, PermissionMode>,
    /// session cache for "ask once" decisions
    session_cache: Arc<RwLock<HashMap<String, bool>>>,
    /// in non-interactive mode, default to allow
    non_interactive: bool,
}

impl PermissionGate {
    pub fn new(non_interactive: bool) -> Self {
        let mut rules = HashMap::new();
        // Defaults — prompt for destructive ops
        rules.insert("bash".to_string(), PermissionMode::Ask);
        rules.insert("write".to_string(), PermissionMode::Allow);
        rules.insert("edit".to_string(), PermissionMode::Allow);

        Self {
            rules,
            session_cache: Arc::new(RwLock::new(HashMap::new())),
            non_interactive,
        }
    }

    pub fn allow_all() -> Self {
        Self {
            rules: HashMap::new(),
            session_cache: Arc::new(RwLock::new(HashMap::new())),
            non_interactive: true,
        }
    }

    /// Returns true if the tool call is allowed to proceed.
    pub async fn check(&self, tool: &str, _input: &Value) -> bool {
        let mode = self.rules.get(tool).unwrap_or(&PermissionMode::Allow);
        match mode {
            PermissionMode::Allow => true,
            PermissionMode::Deny => false,
            PermissionMode::Ask => {
                if self.non_interactive {
                    return true;
                }
                // Check session cache
                let cache = self.session_cache.read().await;
                if let Some(&cached) = cache.get(tool) {
                    return cached;
                }
                drop(cache);
                // TODO: surface TUI permission modal via bus event
                // For now, allow in non-TUI mode
                true
            }
        }
    }

    pub async fn set_cached(&self, tool: &str, allowed: bool) {
        self.session_cache.write().await.insert(tool.to_string(), allowed);
    }
}
