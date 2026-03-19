use c2_storage::{MessageId, SessionId};
use serde::{Deserialize, Serialize};

/// A full message in a session (user or assistant turn).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub session_id: SessionId,
    pub role: Role,
    pub parts: Vec<Part>,
    pub time_created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// A single part within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Part {
    Text { text: String },
    Reasoning { text: String },
    ToolCall { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_call_id: String, content: String, is_error: bool },
}
