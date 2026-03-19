use c2_storage::{MessageId, PartId, SessionId};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// All events that flow through the system.
/// Subscribers (TUI, SSE clients, tests) receive these via broadcast channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    // Session lifecycle
    SessionCreated { session_id: SessionId },
    SessionUpdated { session_id: SessionId },
    SessionDeleted { session_id: SessionId },

    // Message lifecycle
    MessageAdded { session_id: SessionId, message_id: MessageId },

    // Streaming
    TextDelta { session_id: SessionId, message_id: MessageId, delta: String },
    ReasoningDelta { session_id: SessionId, message_id: MessageId, delta: String },
    ToolCallStart { session_id: SessionId, message_id: MessageId, tool_call_id: String, tool_name: String },
    ToolCallDelta { session_id: SessionId, message_id: MessageId, tool_call_id: String, json_delta: String },
    ToolCallEnd { session_id: SessionId, message_id: MessageId, tool_call_id: String },
    ToolResult { session_id: SessionId, message_id: MessageId, tool_call_id: String, content: String, is_error: bool },

    // Agent status
    AgentStarted { session_id: SessionId },
    AgentDone { session_id: SessionId },
    AgentError { session_id: SessionId, error: String },

    // PTY
    PtyOutput { pty_id: String, data: Vec<u8> },
    PtyExit { pty_id: String, exit_code: i32 },

    // MCP
    McpToolsChanged { server: String },
    McpServerError { server: String, error: String },

    // Permission
    PermissionRequired { session_id: SessionId, tool: String, input: serde_json::Value },
    PermissionGranted { session_id: SessionId, tool: String },
    PermissionDenied { session_id: SessionId, tool: String },
}

/// Global event bus backed by a tokio broadcast channel.
#[derive(Clone)]
pub struct Bus {
    tx: broadcast::Sender<Event>,
}

impl Bus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(4096);
        Self { tx }
    }

    pub fn emit(&self, event: Event) {
        // Ignore send errors — no subscribers is fine
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}
