use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str { "web_fetch" }
    fn description(&self) -> &str {
        "Fetch content from a URL and return it as plain text or markdown."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": { "type": "string", "description": "The URL to fetch" }
            }
        })
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let url = match input["url"].as_str() {
            Some(u) => u.to_string(),
            None => return Ok(ToolResult::err("url is required")),
        };

        let client = reqwest::Client::builder()
            .user_agent("cc/0.1")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        match client.get(&url).send().await {
            Err(e) => Ok(ToolResult::err(format!("Request failed: {e}"))),
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    // Very basic HTML stripping — replace with htmd crate in Phase 2
                    let clean = strip_html_tags(&text);
                    Ok(ToolResult::ok(clean))
                } else {
                    Ok(ToolResult::err(format!("HTTP {status}: {}", &text[..text.len().min(500)])))
                }
            }
        }
    }
}

fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    // Collapse excess whitespace
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
