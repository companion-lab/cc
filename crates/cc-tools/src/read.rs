use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str { "read" }
    fn description(&self) -> &str {
        "Read the contents of a file. Supports optional line range (offset/limit)."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string", "description": "Absolute or relative path to the file" },
                "offset": { "type": "integer", "description": "Line number to start reading from (1-indexed)" },
                "limit": { "type": "integer", "description": "Maximum number of lines to return" }
            }
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path_str = match input["path"].as_str() {
            Some(p) => p,
            None => return Ok(ToolResult::err("path is required")),
        };
        let path = ctx.working_dir.join(path_str);

        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => return Ok(ToolResult::err(format!("Failed to read {path_str}: {e}"))),
        };

        let lines: Vec<&str> = content.lines().collect();
        let offset = input["offset"].as_u64().map(|n| (n as usize).saturating_sub(1)).unwrap_or(0);
        let limit = input["limit"].as_u64().map(|n| n as usize).unwrap_or(usize::MAX);

        let selected: Vec<&str> = lines.iter().skip(offset).take(limit).copied().collect();
        Ok(ToolResult::ok(selected.join("\n")))
    }
}
