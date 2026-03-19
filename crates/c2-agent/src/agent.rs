use serde::{Deserialize, Serialize};
use c2_config::AgentConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub tools: Vec<String>,
}

impl From<AgentConfig> for Agent {
    fn from(cfg: AgentConfig) -> Self {
        Self {
            name: cfg.name,
            description: cfg.description,
            model: cfg.model,
            system_prompt: cfg.system_prompt,
            tools: cfg.tools,
        }
    }
}
