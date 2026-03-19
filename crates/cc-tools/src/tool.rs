use anyhow::Result;
use async_trait::async_trait;
use cc_core::bus::Bus;
use cc_permissions::PermissionGate;
use cc_storage::SessionId;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

/// Context passed into every tool call.
#[derive(Clone)]
pub struct ToolContext {
    pub session_id: SessionId,
    pub working_dir: PathBuf,
    pub permissions: Arc<PermissionGate>,
    pub bus: Arc<Bus>,
}

/// Result from a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn ok(content: impl Into<String>) -> Self {
        Self { content: content.into(), is_error: false }
    }

    pub fn err(content: impl Into<String>) -> Self {
        Self { content: content.into(), is_error: true }
    }
}

/// Trait implemented by every built-in and MCP-injected tool.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> Value;

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult>;
}
