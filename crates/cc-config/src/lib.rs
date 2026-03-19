use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Top-level configuration for cc.
///
/// Loaded in priority order (highest wins):
///   1. System-managed  (/etc/cc/config.json or %ProgramData%\cc\config.json)
///   2. User global     (~/.config/cc/config.json)
///   3. Project local   (.cc/config.json in working directory)
///   4. Environment     (CC_API_KEY, CC_MODEL, …)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub model: Option<String>,
    pub provider: Option<ProviderConfig>,
    pub mcp: HashMap<String, McpServerConfig>,
    pub agents: Vec<AgentConfig>,
    pub keybindings: Option<KeybindingConfig>,
    pub experimental: Option<ExperimentalConfig>,
    // Companion-specific
    pub vexa: Option<VexaConfig>,
    pub hivemind: Option<HivemindConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum McpServerConfig {
    #[serde(rename = "stdio")]
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    #[serde(rename = "sse")]
    Sse {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
    #[serde(rename = "http")]
    Http {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub mode: AgentMode,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    #[default]
    Primary,
    Subagent,
    All,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeybindingConfig {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentalConfig {
    pub continue_loop_on_deny: Option<bool>,
    pub bash_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VexaConfig {
    pub base_url: String,
    pub api_key: String,
    /// If true, automatically inject live transcripts into active sessions.
    #[serde(default = "default_true")]
    pub auto_inject: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HivemindConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

// ── Paths ─────────────────────────────────────────────────────────────────────

pub struct Paths;

impl Paths {
    /// ~/.config/cc
    pub fn user_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("cc")
    }

    /// ~/.local/share/cc
    pub fn user_data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("cc")
    }

    /// ~/.config/cc/config.json
    pub fn user_config_file() -> PathBuf {
        Self::user_config_dir().join("config.json")
    }
}

// ── Loader ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config at {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
}

/// Load and merge configuration from all sources.
/// Returns the merged Config, highest priority source wins per field.
pub async fn load(working_dir: &PathBuf) -> Result<Config, ConfigError> {
    let mut merged = Config::default();

    // 1. System-managed
    let system_path = system_config_path();
    if system_path.exists() {
        let cfg = read_json(&system_path)?;
        merge(&mut merged, cfg);
    }

    // 2. User global
    let user_path = Paths::user_config_file();
    if user_path.exists() {
        let cfg = read_json(&user_path)?;
        merge(&mut merged, cfg);
    }

    // 3. Project-local (.cc/config.json)
    let project_path = working_dir.join(".cc").join("config.json");
    if project_path.exists() {
        let cfg = read_json(&project_path)?;
        merge(&mut merged, cfg);
    }

    // 4. Environment variable overrides
    apply_env(&mut merged);

    Ok(merged)
}

fn read_json(path: &PathBuf) -> Result<Config, ConfigError> {
    let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.clone(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| ConfigError::Parse {
        path: path.clone(),
        source,
    })
}

fn merge(base: &mut Config, override_cfg: Config) {
    if override_cfg.model.is_some() {
        base.model = override_cfg.model;
    }
    if override_cfg.provider.is_some() {
        base.provider = override_cfg.provider;
    }
    base.mcp.extend(override_cfg.mcp);
    base.agents.extend(override_cfg.agents);
    if override_cfg.vexa.is_some() {
        base.vexa = override_cfg.vexa;
    }
    if override_cfg.hivemind.is_some() {
        base.hivemind = override_cfg.hivemind;
    }
    if override_cfg.experimental.is_some() {
        base.experimental = override_cfg.experimental;
    }
}

fn apply_env(config: &mut Config) {
    if let Ok(key) = std::env::var("CC_API_KEY") {
        let provider = config.provider.get_or_insert_with(|| ProviderConfig {
            id: "anthropic".to_string(),
            api_key: None,
            base_url: None,
        });
        provider.api_key = Some(key);
    }
    if let Ok(model) = std::env::var("CC_MODEL") {
        config.model = Some(model);
    }
    if let Ok(provider_id) = std::env::var("CC_PROVIDER") {
        let provider = config.provider.get_or_insert_with(|| ProviderConfig {
            id: provider_id.clone(),
            api_key: None,
            base_url: None,
        });
        provider.id = provider_id;
    }
}

fn system_config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("C:\\ProgramData\\cc\\config.json")
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/etc/cc/config.json")
    }
}
