use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use globset::{Glob, GlobSetBuilder};
use serde_json::{json, Value};
use walkdir::WalkDir;

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str { "glob" }
    fn description(&self) -> &str {
        "Find files matching a glob pattern. Returns a list of matching paths."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["pattern"],
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern (e.g. **/*.rs)" },
                "path": { "type": "string", "description": "Root directory to search (default: working dir)" }
            }
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let pattern = match input["pattern"].as_str() {
            Some(p) => p.to_string(),
            None => return Ok(ToolResult::err("pattern is required")),
        };
        let root = input["path"].as_str()
            .map(|p| ctx.working_dir.join(p))
            .unwrap_or_else(|| ctx.working_dir.clone());

        let glob = match Glob::new(&pattern) {
            Ok(g) => g,
            Err(e) => return Ok(ToolResult::err(format!("Invalid glob pattern: {e}"))),
        };
        let mut builder = GlobSetBuilder::new();
        builder.add(glob);
        let set = match builder.build() {
            Ok(s) => s,
            Err(e) => return Ok(ToolResult::err(format!("Failed to build glob set: {e}"))),
        };

        let mut matches: Vec<String> = Vec::new();
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let rel = match entry.path().strip_prefix(&root) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if set.is_match(rel) {
                matches.push(rel.display().to_string());
            }
        }

        matches.sort();
        Ok(ToolResult::ok(matches.join("\n")))
    }
}
