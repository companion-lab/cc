use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: String,
    pub priority: String,
}

pub struct TodoWriteTool;
pub struct TodoReadTool;

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str { "todo_write" }
    fn description(&self) -> &str {
        "Create and manage the task list for the current session. \
        Use to track multi-step tasks and communicate progress to the user."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["todos"],
            "properties": {
                "todos": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["content", "status", "priority"],
                        "properties": {
                            "content": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "completed", "cancelled"] },
                            "priority": { "type": "string", "enum": ["high", "medium", "low"] }
                        }
                    }
                }
            }
        })
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let todos: Vec<TodoItem> = match serde_json::from_value(input["todos"].clone()) {
            Ok(t) => t,
            Err(e) => return Ok(ToolResult::err(format!("Invalid todos: {e}"))),
        };

        let summary: Vec<String> = todos.iter().map(|t| {
            let icon = match t.status.as_str() {
                "completed" => "✓",
                "in_progress" => "→",
                "cancelled" => "✗",
                _ => "○",
            };
            format!("{} [{}] {}", icon, t.priority, t.content)
        }).collect();

        Ok(ToolResult::ok(format!("Todos updated:\n{}", summary.join("\n"))))
    }
}

#[async_trait]
impl Tool for TodoReadTool {
    fn name(&self) -> &str { "todo_read" }
    fn description(&self) -> &str { "Read the current todo list for the active session." }
    fn schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        // TODO: read from cc-storage todos table
        Ok(ToolResult::ok("No todos yet."))
    }
}
