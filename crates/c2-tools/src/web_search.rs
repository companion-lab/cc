use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str { "web_search" }
    fn description(&self) -> &str {
        "Search the web for information. Supports searching academic papers on arxiv, general web search, and finding references. Use 'source' parameter to specify search type: 'arxiv' for academic papers, 'web' for general search (default)."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string", "description": "The search query" },
                "source": { 
                    "type": "string", 
                    "enum": ["web", "arxiv", "scholar"],
                    "description": "Search source: 'web' for general web search, 'arxiv' for arxiv papers, 'scholar' for Google Scholar",
                    "default": "web"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 5, max: 10)",
                    "default": 5
                }
            }
        })
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let query = match input["query"].as_str() {
            Some(q) => q.to_string(),
            None => return Ok(ToolResult::err("query is required")),
        };

        let source = input["source"].as_str().unwrap_or("web");
        let max_results = input["max_results"].as_u64().unwrap_or(5).min(10) as usize;

        match source {
            "arxiv" => search_arxiv(&query, max_results).await,
            "scholar" => search_google_scholar(&query, max_results).await,
            _ => search_web(&query, max_results).await,
        }
    }
}

async fn search_arxiv(query: &str, max_results: usize) -> Result<ToolResult> {
    // Use arxiv API to search for papers
    let encoded_query = url_encode(query);
    let url = format!(
        "http://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results={}",
        encoded_query, max_results
    );

    let client = reqwest::Client::builder()
        .user_agent("c2/0.1")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    match client.get(&url).send().await {
        Err(e) => Ok(ToolResult::err(format!("Arxiv search failed: {e}"))),
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            let results = parse_arxiv_response(&text, max_results);
            Ok(ToolResult::ok(results))
        }
    }
}

async fn search_google_scholar(query: &str, max_results: usize) -> Result<ToolResult> {
    // Use SerpAPI or similar for Google Scholar (requires API key)
    // For now, fall back to arxiv search
    search_arxiv(query, max_results).await
}

async fn search_web(query: &str, max_results: usize) -> Result<ToolResult> {
    // Use a search API - for now use DuckDuckGo lite
    let encoded_query = url_encode(query);
    let url = format!("https://lite.duckduckgo.com/lite/?q={}", encoded_query);

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; c2/0.1)")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    match client.get(&url).send().await {
        Err(e) => Ok(ToolResult::err(format!("Web search failed: {e}"))),
        Ok(resp) => {
            let html = resp.text().await.unwrap_or_default();
            let results = parse_duckduckgo_results(&html, max_results);
            Ok(ToolResult::ok(results))
        }
    }
}

fn parse_arxiv_response(xml: &str, max_results: usize) -> String {
    let mut results = Vec::new();
    let mut current_entry = String::new();
    let mut in_entry = false;

    for line in xml.lines() {
        let line = line.trim();

        if line.contains("<entry>") {
            in_entry = true;
            current_entry.clear();
        } else if line.contains("</entry>") {
            in_entry = false;
            if let Some(result) = parse_arxiv_entry(&current_entry) {
                results.push(result);
                if results.len() >= max_results {
                    break;
                }
            }
        } else if in_entry {
            current_entry.push_str(line);
            current_entry.push('\n');
        }
    }

    if results.is_empty() {
        "No arxiv papers found for this query.".to_string()
    } else {
        format!("Found {} arxiv papers:\n\n{}", results.len(), results.join("\n\n---\n\n"))
    }
}

fn parse_arxiv_entry(xml: &str) -> Option<String> {
    let title = extract_xml_tag(xml, "title")?;
    let summary = extract_xml_tag(xml, "summary")?;
    let published = extract_xml_tag(xml, "published").unwrap_or_default();
    let id = extract_xml_tag(xml, "id").unwrap_or_default();

    // Extract authors
    let mut authors = Vec::new();
    let mut remaining = xml;
    while let Some(start) = remaining.find("<author>") {
        if let Some(end) = remaining[start..].find("</author>") {
            let author_xml = &remaining[start..start + end + 9];
            if let Some(name) = extract_xml_tag(author_xml, "name") {
                authors.push(name);
            }
            remaining = &remaining[start + end + 9..];
        } else {
            break;
        }
    }

    let title = title.replace('\n', " ").trim().to_string();
    let summary = summary.replace('\n', " ").trim().to_string();
    let published = if published.len() >= 10 { &published[..10] } else { &published };

    Some(format!(
        "Title: {}\nAuthors: {}\nPublished: {}\nArxiv ID: {}\nURL: {}\nAbstract: {}",
        title,
        authors.join(", "),
        published,
        id.split('/').last().unwrap_or(&id),
        id,
        if summary.len() > 500 { format!("{}...", &summary[..500]) } else { summary }
    ))
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);

    let start = xml.find(&open_tag)? + open_tag.len();
    let end = xml[start..].find(&close_tag)?;

    Some(xml[start..start + end].to_string())
}

fn parse_duckduckgo_results(html: &str, max_results: usize) -> String {
    let mut results = Vec::new();
    let mut lines = html.lines().peekable();

    while let Some(line) = lines.next() {
        // Look for result links
        if line.contains("<a rel=\"nofollow\"") && line.contains("href=\"") {
            if let Some(url_start) = line.find("href=\"") {
                let url_start = url_start + 6;
                if let Some(url_end) = line[url_start..].find('"') {
                    let url = &line[url_start..url_start + url_end];
                    if url.starts_with("http") {
                        // Get the next line for title
                        let title = lines.next()
                            .map(|l| strip_html_tags(l).trim().to_string())
                            .unwrap_or_default();

                        if !title.is_empty() {
                            results.push(format!("• {}\n  {}", title, url));
                            if results.len() >= max_results {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        "No web search results found.".to_string()
    } else {
        format!("Found {} results:\n\n{}", results.len(), results.join("\n\n"))
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
    out
}

fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push(to_hex_digit(byte >> 4));
                result.push(to_hex_digit(byte & 0xf));
            }
        }
    }
    result
}

fn to_hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + nibble - 10) as char,
        _ => '?',
    }
}
