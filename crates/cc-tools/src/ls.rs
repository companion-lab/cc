use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &str { "ls" }
    fn description(&self) -> &str { "List directory contents with type and size." }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory to list (default: working dir)" }
            }
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let dir = input["path"].as_str()
            .map(|p| ctx.working_dir.join(p))
            .unwrap_or_else(|| ctx.working_dir.clone());

        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(e) => return Ok(ToolResult::err(format!("Failed to read directory: {e}"))),
        };

        let mut lines: Vec<(bool, String, u64)> = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(meta) = entry.metadata().await else { continue };
            let is_dir = meta.is_dir();
            let size = meta.len();
            lines.push((is_dir, name, size));
        }

        // Dirs first, then files; each alphabetically
        lines.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

        let output: Vec<String> = lines.iter().map(|(is_dir, name, size)| {
            if *is_dir {
                format!("{name}/")
            } else {
                format!("{name}  ({size}B)")
            }
        }).collect();

        Ok(ToolResult::ok(output.join("\n")))
    }
}
