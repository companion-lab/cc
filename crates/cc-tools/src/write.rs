use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct WriteTool;

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str { "write" }
    fn description(&self) -> &str {
        "Write content to a file. Creates the file and any parent directories if they don't exist. \
        Prefer edit for targeted changes; use write for new files or full rewrites."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path", "content"],
            "properties": {
                "path": { "type": "string", "description": "Path to write to" },
                "content": { "type": "string", "description": "Full file content" }
            }
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path_str = match input["path"].as_str() {
            Some(p) => p,
            None => return Ok(ToolResult::err("path is required")),
        };
        let content = match input["content"].as_str() {
            Some(c) => c,
            None => return Ok(ToolResult::err("content is required")),
        };

        if !ctx.permissions.check("write", &input).await {
            return Ok(ToolResult::err("Permission denied: write not allowed"));
        }

        let path = ctx.working_dir.join(path_str);
        if let Some(parent) = path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return Ok(ToolResult::err(format!("Failed to create directories: {e}")));
            }
        }

        // Atomic write: write to temp file, then rename
        let tmp = path.with_extension("cc.tmp");
        if let Err(e) = tokio::fs::write(&tmp, content).await {
            return Ok(ToolResult::err(format!("Failed to write temp file: {e}")));
        }
        if let Err(e) = tokio::fs::rename(&tmp, &path).await {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Ok(ToolResult::err(format!("Failed to finalize write: {e}")));
        }

        let lines = content.lines().count();
        Ok(ToolResult::ok(format!("Wrote {lines} lines to {path_str}")))
    }
}
