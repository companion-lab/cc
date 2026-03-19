use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevData {
    pub providers: HashMap<String, ModelsDevProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevProvider {
    pub id: String,
    pub name: String,
    pub env: Vec<String>,
    pub api: String,
    pub models: HashMap<String, ModelsDevModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevModel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub tool_call: bool,
    #[serde(default)]
    pub temperature: bool,
    #[serde(default)]
    pub cost: ModelsDevCost,
    #[serde(default)]
    pub limit: ModelsDevLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelsDevCost {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelsDevLimit {
    #[serde(default)]
    pub context: u32,
    #[serde(default)]
    pub output: u32,
}

pub struct ModelsFetcher {
    cache_path: PathBuf,
    data: Option<ModelsDevData>,
}

impl ModelsFetcher {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let cache_path = PathBuf::from(home).join(".cache/c2/models.json");

        Self {
            cache_path,
            data: None,
        }
    }

    pub fn load_or_fetch(&mut self) -> Result<&ModelsDevData, String> {
        // Try to load from cache first
        if self.data.is_none() {
            if let Ok(content) = std::fs::read_to_string(&self.cache_path) {
                if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&content) {
                    self.data = Some(self.parse_raw_data(&raw));
                }
            }

            // If no cache, try to fetch
            if self.data.is_none() {
                match self.fetch_from_api() {
                    Ok(raw) => {
                        self.data = Some(self.parse_raw_data(&raw));
                        // Save to cache
                        if let Some(parent) = self.cache_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&self.cache_path, serde_json::to_string_pretty(&raw).unwrap_or_default());
                    }
                    Err(e) => {
                        return Err(format!("Failed to fetch models: {}", e));
                    }
                }
            }
        }

        self.data.as_ref().ok_or_else(|| "No model data available".to_string())
    }

    fn fetch_from_api(&self) -> Result<serde_json::Value, String> {
        // Use curl to fetch models.dev API
        let output = std::process::Command::new("curl")
            .args(["-s", "-f", "--max-time", "10", "https://models.dev/api.json"])
            .output()
            .map_err(|e| format!("Failed to run curl: {}", e))?;

        if !output.status.success() {
            return Err("Failed to fetch models.dev API".to_string());
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Failed to parse models.dev response: {}", e))
    }

    fn parse_raw_data(&self, raw: &serde_json::Value) -> ModelsDevData {
        let mut providers = HashMap::new();

        if let Some(obj) = raw.as_object() {
            for (provider_id, provider_data) in obj {
                if let Some(provider_obj) = provider_data.as_object() {
                    let name = provider_obj.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(provider_id)
                        .to_string();

                    let env = provider_obj.get("env")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();

                    let api = provider_obj.get("api")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let mut models = HashMap::new();
                    if let Some(models_obj) = provider_obj.get("models").and_then(|v| v.as_object()) {
                        for (model_id, model_data) in models_obj {
                            if let Some(model_obj) = model_data.as_object() {
                                let model = ModelsDevModel {
                                    id: model_id.clone(),
                                    name: model_obj.get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(model_id)
                                        .to_string(),
                                    family: model_obj.get("family")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    attachment: model_obj.get("attachment")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                    reasoning: model_obj.get("reasoning")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                    tool_call: model_obj.get("tool_call")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                    temperature: model_obj.get("temperature")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                    cost: model_obj.get("cost").and_then(|v| v.as_object()).map(|c| {
                                        ModelsDevCost {
                                            input: c.get("input").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                            output: c.get("output").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                        }
                                    }).unwrap_or_default(),
                                    limit: model_obj.get("limit").and_then(|v| v.as_object()).map(|l| {
                                        ModelsDevLimit {
                                            context: l.get("context").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                            output: l.get("output").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                        }
                                    }).unwrap_or_default(),
                                };
                                models.insert(model_id.clone(), model);
                            }
                        }
                    }

                    providers.insert(provider_id.clone(), ModelsDevProvider {
                        id: provider_id.clone(),
                        name,
                        env,
                        api,
                        models,
                    });
                }
            }
        }

        ModelsDevData { providers }
    }

    pub fn get_free_models(&self) -> Vec<(String, String, ModelsDevModel)> {
        let mut free_models = Vec::new();

        if let Some(data) = &self.data {
            for (provider_id, provider) in &data.providers {
                for (model_id, model) in &provider.models {
                    if model.cost.input == 0.0 && model.cost.output == 0.0 {
                        free_models.push((
                            provider_id.clone(),
                            provider.name.clone(),
                            model.clone(),
                        ));
                    }
                }
            }
        }

        // Sort by provider name, then model name
        free_models.sort_by(|a, b| {
            a.1.cmp(&b.1).then_with(|| a.2.name.cmp(&b.2.name))
        });

        free_models
    }

    pub fn get_models_for_provider(&self, provider_id: &str) -> Vec<ModelsDevModel> {
        if let Some(data) = &self.data {
            if let Some(provider) = data.providers.get(provider_id) {
                return provider.models.values().cloned().collect();
            }
        }
        Vec::new()
    }

    pub fn get_providers(&self) -> Vec<(String, String)> {
        let mut providers = Vec::new();

        if let Some(data) = &self.data {
            for (id, provider) in &data.providers {
                providers.push((id.clone(), provider.name.clone()));
            }
        }

        providers.sort_by(|a, b| a.1.cmp(&b.1));
        providers
    }
}
