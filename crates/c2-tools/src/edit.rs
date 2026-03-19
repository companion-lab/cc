/// edit tool stub — full unified-diff apply logic in Phase 2
use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str { "edit" }
    fn description(&self) -> &str {
        "Edit a file by replacing an exact string with a new string. \
        old_string must match exactly (including whitespace) and must be unique in the file."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path", "old_string", "new_string"],
            "properties": {
                "path": { "type": "string" },
                "old_string": { "type": "string", "description": "Exact content to replace" },
                "new_string": { "type": "string", "description": "Content to replace with" }
            }
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path_str = match input["path"].as_str() { Some(p) => p, None => return Ok(ToolResult::err("path required")) };
        let old_str = match input["old_string"].as_str() { Some(s) => s, None => return Ok(ToolResult::err("old_string required")) };
        let new_str = match input["new_string"].as_str() { Some(s) => s, None => return Ok(ToolResult::err("new_string required")) };

        if !ctx.permissions.check("edit", &input).await {
            return Ok(ToolResult::err("Permission denied: edit not allowed"));
        }

        let path = ctx.working_dir.join(path_str);
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => return Ok(ToolResult::err(format!("Cannot read {path_str}: {e}"))),
        };

        let count = content.matches(old_str).count();
        if count == 0 {
            return Ok(ToolResult::err(format!("old_string not found in {path_str}")));
        }
        if count > 1 {
            return Ok(ToolResult::err(format!(
                "old_string matches {count} times in {path_str} — must be unique"
            )));
        }

        let new_content = content.replacen(old_str, new_str, 1);
        tokio::fs::write(&path, &new_content).await
            .map_err(|e| anyhow::anyhow!("Cannot write {path_str}: {e}"))?;

        Ok(ToolResult::ok(format!("Edited {path_str}")))
    }
}
