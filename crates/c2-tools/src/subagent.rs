use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct SubagentTool;

#[async_trait]
impl Tool for SubagentTool {
    fn name(&self) -> &str { "subagent" }
    fn description(&self) -> &str {
        "Spawn a subagent to perform a task. The subagent runs independently and returns its results. Use this for complex tasks that benefit from a focused agent, like research, analysis, or multi-step operations."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["task"],
            "properties": {
                "task": { 
                    "type": "string", 
                    "description": "The task for the subagent to perform. Be specific and detailed." 
                },
                "agent": {
                    "type": "string",
                    "enum": ["general", "research", "explore"],
                    "description": "Which agent to use: 'general' for coding tasks, 'research' for academic research with citations, 'explore' for codebase exploration",
                    "default": "general"
                },
                "context": {
                    "type": "string",
                    "description": "Additional context to provide to the subagent"
                }
            }
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let task = match input["task"].as_str() {
            Some(t) => t.to_string(),
            None => return Ok(ToolResult::err("task is required")),
        };

        let agent_type = input["agent"].as_str().unwrap_or("general");
        let context = input["context"].as_str().unwrap_or("");

        // Build the subagent prompt based on agent type
        let system_prompt = match agent_type {
            "research" => RESEARCH_SYSTEM_PROMPT,
            "explore" => EXPLORE_SYSTEM_PROMPT,
            _ => GENERAL_SUBAGENT_PROMPT,
        };

        let full_task = if context.is_empty() {
            task
        } else {
            format!("Context:\n{}\n\nTask:\n{}", context, task)
        };

        // Emit a bus event to indicate subagent is running
        ctx.bus.emit(c2_core::Event::AgentStarted {
            session_id: ctx.session_id.clone(),
        });

        // For now, return a structured response indicating the subagent task
        // In a full implementation, this would spawn a new Processor
        let result = format!(
            "Subagent [{}] queued for task:\n{}\n\nNote: Full subagent execution requires processor integration.",
            agent_type, full_task
        );

        Ok(ToolResult::ok(result))
    }
}

const GENERAL_SUBAGENT_PROMPT: &str = r#"You are a focused coding subagent. Your task is to complete the given task efficiently and return clear results.

Guidelines:
- Be concise and direct
- Use tools to read/write files as needed
- Return a summary of what you did and any relevant output
- If you encounter errors, explain them clearly"#;

const RESEARCH_SYSTEM_PROMPT: &str = r#"You are a research subagent specializing in finding and citing academic papers and authoritative sources.

Your task is to research the given topic thoroughly and provide accurate, well-cited information.

Guidelines:
1. Search for relevant academic papers on arxiv using web_search with source="arxiv"
2. Use web_fetch to read full paper abstracts and content when needed
3. Always provide proper citations with:
   - Paper title
   - Authors
   - Publication date
   - Arxiv ID or DOI when available
   - URL to the paper
4. Distinguish between established facts and emerging research
5. Include confidence levels for claims (high/medium/low)
6. If findings are contradictory, present multiple viewpoints with sources

Output format:
## Summary
[2-3 sentence overview of findings]

## Key Findings
1. [Finding with citation]
2. [Finding with citation]
...

## References
1. [Full citation with URL]
2. [Full citation with URL]
...

## Confidence Assessment
- [Topic aspect]: [confidence level] based on [evidence]"#;

const EXPLORE_SYSTEM_PROMPT: &str = r#"You are a codebase exploration subagent. Your task is to quickly understand and map out code structures.

Guidelines:
- Use glob to find relevant files
- Use grep to search for patterns
- Use read to examine key files
- Return a clear summary of what you found
- Include file paths and line numbers for important findings"#;
