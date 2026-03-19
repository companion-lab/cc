use anyhow::Result;
use c2_core::bus::Bus;
use c2_core::session::Session;
use c2_provider::{LanguageModel, ModelMessage, StreamEvent, StreamOptions, ContentPart};
use c2_storage::Db;
use c2_tools::{ToolRegistry, ToolContext};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

const MAX_TOOL_ROUNDS: usize = 50;

struct PendingToolCall {
    id: String,
    name: String,
    json_args: String,
}

pub struct Processor {
    pub model: Arc<dyn LanguageModel>,
    pub db: Arc<Db>,
    pub bus: Arc<Bus>,
    pub registry: Arc<ToolRegistry>,
}

impl Processor {
    pub fn new(model: Arc<dyn LanguageModel>, db: Arc<Db>, bus: Arc<Bus>) -> Self {
        Self { 
            model, 
            db, 
            bus,
            registry: Arc::new(ToolRegistry::new()),
        }
    }

    /// Run the agent loop for a session given a user prompt.
    pub async fn run(
        &self,
        session: &Session,
        prompt: String,
        abort: tokio::sync::watch::Receiver<bool>,
    ) -> Result<()> {
        use c2_core::Event;

        let session_id = session.id.clone();
        self.bus.emit(Event::AgentStarted { session_id: session_id.clone() });
        info!("agent started session={}", session_id);

        let mut messages = vec![ModelMessage::User {
            content: vec![ContentPart::Text { text: prompt }],
        }];

        let opts = StreamOptions {
            system_prompt: Some(system_prompt()),
            max_tokens: Some(8096),
            temperature: None,
            tools: self.registry.definitions(),
        };

        let mut rounds = 0;
        loop {
            if rounds >= MAX_TOOL_ROUNDS {
                error!("agent hit max tool rounds ({})", MAX_TOOL_ROUNDS);
                break;
            }
            if *abort.borrow() {
                info!("agent aborted");
                break;
            }

            rounds += 1;

            let mut stream = self.model.stream(messages.clone(), opts.clone()).await?;
            let mut text_buf = String::new();
            
            // Track tool calls in this stream
            let mut pending_tools: HashMap<String, PendingToolCall> = HashMap::new();

            while let Some(event) = stream.next().await {
                if *abort.borrow() {
                    break;
                }

                match event? {
                    StreamEvent::TextDelta(delta) => {
                        text_buf.push_str(&delta);
                        self.bus.emit(Event::TextDelta {
                            session_id: session_id.clone(),
                            message_id: c2_storage::MessageId::new(),
                            delta,
                        });
                    }
                    StreamEvent::ReasoningDelta(delta) => {
                        self.bus.emit(Event::ReasoningDelta {
                            session_id: session_id.clone(),
                            message_id: c2_storage::MessageId::new(),
                            delta,
                        });
                    }
                    StreamEvent::ToolCallStart { id, name } => {
                        pending_tools.insert(id.clone(), PendingToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            json_args: String::new(),
                        });
                        self.bus.emit(Event::ToolCallStart {
                            session_id: session_id.clone(),
                            message_id: c2_storage::MessageId::new(),
                            tool_call_id: id,
                            tool_name: name,
                        });
                    }
                    StreamEvent::ToolCallDelta { id, json_delta } => {
                        if let Some(tool) = pending_tools.get_mut(&id) {
                            tool.json_args.push_str(&json_delta);
                        }
                    }
                    StreamEvent::ToolCallEnd { id } => {
                        self.bus.emit(Event::ToolCallEnd {
                            session_id: session_id.clone(),
                            message_id: c2_storage::MessageId::new(),
                            tool_call_id: id,
                        });
                    }
                    StreamEvent::Done => break,
                    _ => {}
                }
            }

            // Append assistant message
            let mut assistant_content = vec![];
            if !text_buf.is_empty() {
                assistant_content.push(ContentPart::Text { text: text_buf });
            }

            for tool in pending_tools.values() {
                let input: serde_json::Value = serde_json::from_str(&tool.json_args)
                    .unwrap_or_else(|_| serde_json::json!({}));
                assistant_content.push(ContentPart::ToolUse {
                    id: tool.id.clone(),
                    name: tool.name.clone(),
                    input,
                });
            }

            if !assistant_content.is_empty() {
                messages.push(ModelMessage::Assistant { content: assistant_content });
            }

            // If no tool calls were made, the agent is done
            if pending_tools.is_empty() {
                break;
            }

            // Dispatch tool calls
            for pending in pending_tools.into_values() {
                let tool_name = pending.name.clone();
                let tool_id = pending.id.clone();
                let input: serde_json::Value = serde_json::from_str(&pending.json_args)
                    .unwrap_or_else(|_| serde_json::json!({}));
                
                info!("executing tool {} ({})", tool_name, tool_id);
                
                let tool_result = if let Some(tool) = self.registry.get(&tool_name) {
                    let ctx = ToolContext {
                        session_id: session_id.clone(),
                        working_dir: session.directory.clone().into(),
                        bus: self.bus.clone(),
                        permissions: Arc::new(c2_permissions::PermissionGate::allow_all()), // TODO: inject from AppState
                    };
                    
                    match tool.call(input, &ctx).await {
                        Ok(res) => {
                            let text = res.content.clone();
                            self.bus.emit(Event::ToolResult {
                                session_id: session_id.clone(),
                                message_id: c2_storage::MessageId::new(),
                                tool_call_id: tool_id.clone(),
                                content: text.clone(),
                                is_error: res.is_error,
                            });
                            text
                        }
                        Err(e) => {
                            let text = format!("Error: {}", e);
                            self.bus.emit(Event::ToolResult {
                                session_id: session_id.clone(),
                                message_id: c2_storage::MessageId::new(),
                                tool_call_id: tool_id.clone(),
                                content: text.clone(),
                                is_error: true,
                            });
                            text
                        }
                    }
                } else {
                    let text = format!("Error: Tool '{}' not found", tool_name);
                    self.bus.emit(Event::ToolResult {
                        session_id: session_id.clone(),
                        message_id: c2_storage::MessageId::new(),
                        tool_call_id: tool_id.clone(),
                        content: text.clone(),
                        is_error: true,
                    });
                    text
                };

                messages.push(ModelMessage::Tool {
                    tool_use_id: tool_id,
                    content: tool_result,
                });
            }
        }

        self.bus.emit(Event::AgentDone { session_id: session_id.clone() });
        info!("agent done session={}", session_id);
        Ok(())
    }
}

fn system_prompt() -> String {
    include_str!("system_prompt.txt").to_string()
}
