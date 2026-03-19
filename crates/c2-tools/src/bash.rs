use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::process::Command;

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str { "bash" }

    fn description(&self) -> &str {
        "Execute a shell command in the current working directory. \
        Use for running tests, builds, git operations, and other terminal tasks. \
        Commands run with a 2-minute timeout by default."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Optional timeout in milliseconds (default: 120000)"
                }
            }
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let command = match input["command"].as_str() {
            Some(c) if !c.is_empty() => c.to_string(),
            _ => return Ok(ToolResult::err("command is required")),
        };
        let timeout_ms = input["timeout_ms"].as_u64().unwrap_or(120_000);

        // Permission check
        if !ctx.permissions.check("bash", &input).await {
            return Ok(ToolResult::err(format!(
                "Permission denied: bash is not allowed to run: {}",
                &command[..command.len().min(80)]
            )));
        }

        let result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            Command::new("sh")
                .arg("-c")
                .arg(&command)
                .current_dir(&ctx.working_dir)
                .output(),
        )
        .await;

        match result {
            Err(_) => Ok(ToolResult::err(format!(
                "Command timed out after {}ms: {}",
                timeout_ms,
                &command[..command.len().min(80)]
            ))),
            Ok(Err(e)) => Ok(ToolResult::err(format!("Failed to spawn command: {e}"))),
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                let mut content = String::new();
                if !stdout.is_empty() {
                    content.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(&format!("[stderr]\n{stderr}"));
                }
                if content.is_empty() {
                    content = format!("[exit code: {exit_code}]");
                }

                Ok(if output.status.success() {
                    ToolResult::ok(content)
                } else {
                    ToolResult::err(format!("[exit code: {exit_code}]\n{content}"))
                })
            }
        }
    }
}
