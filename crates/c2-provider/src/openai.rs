use crate::{
    ContentPart, LanguageModel, ModelMessage, ProviderStream, StreamEvent, StreamOptions,
    ToolDefinition, UsageInfo,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_MODEL: &str = "gpt-4o";

pub struct OpenAIProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: Client,
}

impl OpenAIProvider {
    pub fn new(api_key: impl Into<String>, model: Option<String>, base_url: Option<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            base_url: base_url.unwrap_or_else(|| OPENAI_API_URL.to_string()),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LanguageModel for OpenAIProvider {
    fn id(&self) -> &str { &self.model }
    fn provider_id(&self) -> &str { "openai" }
    fn context_length(&self) -> u32 { 128_000 }
    fn supports_tools(&self) -> bool { true }

    async fn stream(
        &self,
        messages: Vec<ModelMessage>,
        options: StreamOptions,
    ) -> Result<ProviderStream> {
        let msgs: Vec<Value> = messages.iter().map(msg_to_json).collect();
        let mut body = json!({
            "model": self.model,
            "messages": msgs,
            "stream": true,
            "stream_options": { "include_usage": true },
        });

        if let Some(sys) = &options.system_prompt {
            // Prepend system message
            let mut all_msgs = vec![json!({"role": "system", "content": sys})];
            all_msgs.extend(msgs);
            body["messages"] = json!(all_msgs);
        }

        if !options.tools.is_empty() {
            let tools: Vec<Value> = options.tools.iter().map(tool_to_json).collect();
            body["tools"] = json!(tools);
        }

        let response = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("openai request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("openai error {status}: {body}");
        }

        let byte_stream = response.bytes_stream();
        Ok(Box::pin(parse_sse(byte_stream)))
    }
}

fn msg_to_json(msg: &ModelMessage) -> Value {
    match msg {
        ModelMessage::User { content } => {
            let text = content.iter().filter_map(|p| {
                if let ContentPart::Text { text } = p { Some(text.as_str()) } else { None }
            }).collect::<Vec<_>>().join("\n");
            json!({"role": "user", "content": text})
        }
        ModelMessage::Assistant { content } => {
            let text = content.iter().filter_map(|p| {
                if let ContentPart::Text { text } = p { Some(text.as_str()) } else { None }
            }).collect::<Vec<_>>().join("\n");
            json!({"role": "assistant", "content": text})
        }
        ModelMessage::Tool { tool_use_id, content } => {
            json!({"role": "tool", "tool_call_id": tool_use_id, "content": content})
        }
    }
}

fn tool_to_json(tool: &ToolDefinition) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.schema,
        }
    })
}

fn parse_sse(
    byte_stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
) -> impl Stream<Item = Result<StreamEvent>> + Send {
    async_stream::stream! {
        let mut buffer = String::new();
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

                    let data = message.lines()
                        .find_map(|l| l.strip_prefix("data: "))
                        .unwrap_or("")
                        .to_string();

                    if data == "[DONE]" {
                        yield Ok(StreamEvent::Done);
                        return;
                    }
                    if data.is_empty() { continue; }

                    let v: Value = match serde_json::from_str(&data) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    // Usage
                    if let Some(usage) = v.get("usage") {
                        yield Ok(StreamEvent::Usage(UsageInfo {
                            input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                            output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                            ..Default::default()
                        }));
                    }

                    for choice in v["choices"].as_array().unwrap_or(&vec![]) {
                        let delta = &choice["delta"];

                        // Thinking/Reasoning (for models like Mimo, DeepSeek R1, etc.)
                        if let Some(thinking) = delta["reasoning_content"].as_str() {
                            if !thinking.is_empty() {
                                yield Ok(StreamEvent::ReasoningDelta(thinking.to_string()));
                            }
                        }

                        // Also check for "thinking" field (some models use this)
                        if let Some(thinking) = delta["thinking"].as_str() {
                            if !thinking.is_empty() {
                                yield Ok(StreamEvent::ReasoningDelta(thinking.to_string()));
                            }
                        }

                        // Text
                        if let Some(text) = delta["content"].as_str() {
                            if !text.is_empty() {
                                yield Ok(StreamEvent::TextDelta(text.to_string()));
                            }
                        }

                        // Tool calls
                        if let Some(tool_calls) = delta["tool_calls"].as_array() {
                            for tc in tool_calls {
                                let idx = tc["index"].as_u64().unwrap_or(0);
                                if let Some(func) = tc.get("function") {
                                    if let Some(name) = func["name"].as_str() {
                                        let id = tc["id"].as_str().unwrap_or("").to_string();
                                        yield Ok(StreamEvent::ToolCallStart { id: id.clone(), name: name.to_string() });
                                    }
                                    if let Some(json_delta) = func["arguments"].as_str() {
                                        let id = format!("tool_{idx}");
                                        yield Ok(StreamEvent::ToolCallDelta { id, json_delta: json_delta.to_string() });
                                    }
                                }
                            }
                        }

                        if let Some("stop") = choice["finish_reason"].as_str() {
                            yield Ok(StreamEvent::Done);
                            return;
                        }
                    }
                } else { break; }
            }
        }
    }
}
