use crate::{
    ContentPart, LanguageModel, ModelMessage, ProviderStream, StreamEvent, StreamOptions,
    ToolDefinition, UsageInfo,
};
use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::pin::Pin;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_MODEL: &str = "claude-3-7-sonnet-20250219";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>, model: Option<String>, base_url: Option<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            base_url: base_url.unwrap_or_else(|| ANTHROPIC_API_URL.to_string()),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LanguageModel for AnthropicProvider {
    fn id(&self) -> &str {
        &self.model
    }

    fn provider_id(&self) -> &str {
        "anthropic"
    }

    fn context_length(&self) -> u32 {
        200_000
    }

    fn supports_tools(&self) -> bool {
        true
    }

    async fn stream(
        &self,
        messages: Vec<ModelMessage>,
        options: StreamOptions,
    ) -> Result<ProviderStream> {
        let body = build_request_body(&self.model, messages, &options);

        let response = self
            .client
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .context("anthropic request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("anthropic error {status}: {body}");
        }

        let byte_stream = response.bytes_stream();
        let event_stream = parse_sse(byte_stream);
        Ok(Box::pin(event_stream))
    }
}

// ── Request builder ────────────────────────────────────────────────────────────

fn build_request_body(model: &str, messages: Vec<ModelMessage>, opts: &StreamOptions) -> Value {
    let msgs: Vec<Value> = messages.iter().map(message_to_json).collect();
    let tools: Vec<Value> = opts.tools.iter().map(tool_to_json).collect();

    let mut body = json!({
        "model": model,
        "max_tokens": opts.max_tokens.unwrap_or(8096),
        "messages": msgs,
        "stream": true,
    });

    if let Some(system) = &opts.system_prompt {
        body["system"] = json!(system);
    }

    if !tools.is_empty() {
        body["tools"] = json!(tools);
    }

    body
}

fn message_to_json(msg: &ModelMessage) -> Value {
    match msg {
        ModelMessage::User { content } => json!({
            "role": "user",
            "content": content.iter().map(part_to_json).collect::<Vec<_>>(),
        }),
        ModelMessage::Assistant { content } => json!({
            "role": "assistant",
            "content": content.iter().map(part_to_json).collect::<Vec<_>>(),
        }),
        ModelMessage::Tool { tool_use_id, content } => json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": content,
            }],
        }),
    }
}

fn part_to_json(part: &ContentPart) -> Value {
    match part {
        ContentPart::Text { text } => json!({"type": "text", "text": text}),
        ContentPart::ToolUse { id, name, input } => json!({
            "type": "tool_use",
            "id": id,
            "name": name,
            "input": input,
        }),
        ContentPart::ToolResult { tool_use_id, content } => json!({
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": content,
        }),
    }
}

fn tool_to_json(tool: &ToolDefinition) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description,
        "input_schema": tool.schema,
    })
}

// ── SSE parser ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicEvent {
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: Delta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: MessageDeltaInfo,
        usage: Option<AnthropicUsage>,
    },
    MessageStop,
    MessageStart {
        message: AnthropicMessageStart,
    },
    Ping,
    Error {
        error: AnthropicError,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String },
    Thinking { thinking: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Delta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
}

#[derive(Debug, Deserialize)]
struct MessageDeltaInfo {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageStart {
    usage: Option<AnthropicMessageUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageUsage {
    input_tokens: Option<u32>,
    cache_creation_input_tokens: Option<u32>,
    cache_read_input_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AnthropicError {
    message: String,
}

struct IndexedBlock {
    tool_id: Option<String>,
    tool_name: Option<String>,
    kind: BlockKind,
}

enum BlockKind {
    Text,
    ToolUse,
    Thinking,
}

fn parse_sse(
    byte_stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
) -> impl Stream<Item = Result<StreamEvent>> + Send {
    async_stream::stream! {
        let mut buffer = String::new();
        let mut blocks: std::collections::HashMap<usize, IndexedBlock> = std::collections::HashMap::new();

        futures::pin_mut!(byte_stream);

        while let Some(chunk) = byte_stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(e) => { yield Err(anyhow::anyhow!(e)); break; }
            };

            buffer.push_str(&String::from_utf8_lossy(&bytes));

            loop {
                if let Some(pos) = buffer.find("\n\n") {
                    let message = buffer[..pos].to_string();
                    buffer.drain(..pos + 2);

                    let mut event_type = String::new();
                    let mut data = String::new();

                    for line in message.lines() {
                        if let Some(t) = line.strip_prefix("event: ") {
                            event_type = t.to_string();
                        } else if let Some(d) = line.strip_prefix("data: ") {
                            data = d.to_string();
                        }
                    }

                    if data == "[DONE]" || event_type == "message_stop" {
                        yield Ok(StreamEvent::Done);
                        return;
                    }

                    if data.is_empty() { continue; }

                    let ev: AnthropicEvent = match serde_json::from_str(&data) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    match ev {
                        AnthropicEvent::MessageStart { message } => {
                            if let Some(usage) = message.usage {
                                yield Ok(StreamEvent::Usage(UsageInfo {
                                    input_tokens: usage.input_tokens.unwrap_or(0),
                                    cache_read_tokens: usage.cache_read_input_tokens.unwrap_or(0),
                                    cache_write_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
                                    ..Default::default()
                                }));
                            }
                        }
                        AnthropicEvent::ContentBlockStart { index, content_block } => {
                            match content_block {
                                ContentBlock::Text { .. } => {
                                    blocks.insert(index, IndexedBlock { tool_id: None, tool_name: None, kind: BlockKind::Text });
                                }
                                ContentBlock::ToolUse { id, name } => {
                                    blocks.insert(index, IndexedBlock { tool_id: Some(id.clone()), tool_name: Some(name.clone()), kind: BlockKind::ToolUse });
                                    yield Ok(StreamEvent::ToolCallStart { id, name });
                                }
                                ContentBlock::Thinking { .. } => {
                                    blocks.insert(index, IndexedBlock { tool_id: None, tool_name: None, kind: BlockKind::Thinking });
                                }
                            }
                        }
                        AnthropicEvent::ContentBlockDelta { index, delta } => {
                            match delta {
                                Delta::TextDelta { text } => {
                                    yield Ok(StreamEvent::TextDelta(text));
                                }
                                Delta::InputJsonDelta { partial_json } => {
                                    if let Some(block) = blocks.get(&index) {
                                        if let Some(id) = &block.tool_id {
                                            yield Ok(StreamEvent::ToolCallDelta { id: id.clone(), json_delta: partial_json });
                                        }
                                    }
                                }
                                Delta::ThinkingDelta { thinking } => {
                                    yield Ok(StreamEvent::ReasoningDelta(thinking));
                                }
                            }
                        }
                        AnthropicEvent::ContentBlockStop { index } => {
                            if let Some(block) = blocks.remove(&index) {
                                if let Some(id) = block.tool_id {
                                    yield Ok(StreamEvent::ToolCallEnd { id });
                                }
                            }
                        }
                        AnthropicEvent::MessageDelta { usage, .. } => {
                            if let Some(usage) = usage {
                                yield Ok(StreamEvent::Usage(UsageInfo {
                                    output_tokens: usage.output_tokens.unwrap_or(0),
                                    ..Default::default()
                                }));
                            }
                        }
                        AnthropicEvent::MessageStop => {
                            yield Ok(StreamEvent::Done);
                            return;
                        }
                        AnthropicEvent::Error { error } => {
                            yield Err(anyhow::anyhow!("anthropic error: {}", error.message));
                            return;
                        }
                        AnthropicEvent::Ping => {}
                    }
                } else {
                    break;
                }
            }
        }
    }
}
