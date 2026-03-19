use anyhow::Result;
use cc_core::bus::Bus;
use cc_core::session::Session;
use cc_provider::{LanguageModel, ModelMessage, StreamEvent, StreamOptions};
use cc_storage::Db;
use futures::StreamExt;
use std::sync::Arc;
use tracing::{error, info};

const MAX_TOOL_ROUNDS: usize = 50;
const DOOM_LOOP_THRESHOLD: usize = 3;

pub struct Processor {
    pub model: Arc<dyn LanguageModel>,
    pub db: Arc<Db>,
    pub bus: Arc<Bus>,
}

impl Processor {
    pub fn new(model: Arc<dyn LanguageModel>, db: Arc<Db>, bus: Arc<Bus>) -> Self {
        Self { model, db, bus }
    }

    /// Run the agent loop for a session given a user prompt.
    pub async fn run(
        &self,
        session: &Session,
        prompt: String,
        abort: tokio::sync::watch::Receiver<bool>,
    ) -> Result<()> {
        use cc_core::Event;

        let session_id = session.id.clone();
        self.bus.emit(Event::AgentStarted { session_id: session_id.clone() });
        info!("agent started session={}", session_id);

        // Build initial messages
        let mut messages = vec![ModelMessage::User {
            content: vec![cc_provider::ContentPart::Text { text: prompt }],
        }];

        let opts = StreamOptions {
            system_prompt: Some(system_prompt()),
            max_tokens: Some(8096),
            temperature: None,
            tools: vec![], // TODO: inject tools from ToolRegistry
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
            let mut had_tool_calls = false;

            while let Some(event) = stream.next().await {
                if *abort.borrow() {
                    break;
                }

                match event? {
                    StreamEvent::TextDelta(delta) => {
                        text_buf.push_str(&delta);
                        self.bus.emit(Event::TextDelta {
                            session_id: session_id.clone(),
                            message_id: cc_storage::MessageId::new(),
                            delta,
                        });
                    }
                    StreamEvent::ToolCallStart { id, name } => {
                        had_tool_calls = true;
                        info!("tool call start: {} ({})", name, id);
                        self.bus.emit(Event::ToolCallStart {
                            session_id: session_id.clone(),
                            message_id: cc_storage::MessageId::new(),
                            tool_call_id: id,
                            tool_name: name,
                        });
                    }
                    StreamEvent::Done => break,
                    _ => {}
                }
            }

            // If no tool calls were made, the agent is done
            if !had_tool_calls {
                break;
            }

            // TODO: dispatch tool calls and append results to messages
            // For now, break after one round
            break;
        }

        self.bus.emit(Event::AgentDone { session_id: session_id.clone() });
        info!("agent done session={}", session_id);
        Ok(())
    }
}

fn system_prompt() -> String {
    include_str!("system_prompt.txt").to_string()
}
