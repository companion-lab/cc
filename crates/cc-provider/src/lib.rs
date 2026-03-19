use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

// ── Model message types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ModelMessage {
    User { content: Vec<ContentPart> },
    Assistant { content: Vec<ContentPart> },
    Tool { tool_use_id: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String },
}

// ── Streaming events ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StreamEvent {
    TextDelta(String),
    ReasoningDelta(String),
    ToolCallStart { id: String, name: String },
    ToolCallDelta { id: String, json_delta: String },
    ToolCallEnd { id: String },
    Usage(UsageInfo),
    Done,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageInfo {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
}

// ── Stream options ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StreamOptions {
    pub system_prompt: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

// ── Provider trait ─────────────────────────────────────────────────────────────

pub type ProviderStream = Pin<Box<dyn Stream<Item = anyhow::Result<StreamEvent>> + Send>>;

#[async_trait]
pub trait LanguageModel: Send + Sync {
    fn id(&self) -> &str;
    fn provider_id(&self) -> &str;
    fn context_length(&self) -> u32;
    fn supports_tools(&self) -> bool;

    async fn stream(
        &self,
        messages: Vec<ModelMessage>,
        options: StreamOptions,
    ) -> anyhow::Result<ProviderStream>;
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub mod anthropic;
pub mod openai;
pub mod registry;

pub use registry::ProviderRegistry;
