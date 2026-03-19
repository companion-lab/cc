use crate::tool::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "grep" }
    fn description(&self) -> &str {
        "Search for a regex pattern in files. Returns matching lines with file:line context."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["pattern", "path"],
            "properties": {
                "pattern": { "type": "string", "description": "Regex pattern to search for" },
                "path": { "type": "string", "description": "Directory or file to search" },
                "include": { "type": "string", "description": "Glob pattern to filter files (e.g. *.rs)" }
            }
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let pattern_str = match input["pattern"].as_str() {
            Some(p) => p,
            None => return Ok(ToolResult::err("pattern is required")),
        };
        let path_str = match input["path"].as_str() {
            Some(p) => p,
            None => return Ok(ToolResult::err("path is required")),
        };

        let re = match Regex::new(pattern_str) {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::err(format!("Invalid regex: {e}"))),
        };

        let search_root = ctx.working_dir.join(path_str);
        let include_glob = input["include"].as_str().and_then(|g| {
            globset::Glob::new(g).ok().and_then(|g| {
                let mut b = globset::GlobSetBuilder::new();
                b.add(g);
                b.build().ok()
            })
        });

        let mut results: Vec<String> = Vec::new();
        const MAX_RESULTS: usize = 500;

        if search_root.is_file() {
            grep_file(&search_root, &re, &mut results, MAX_RESULTS);
        } else {
            for entry in WalkDir::new(&search_root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                if results.len() >= MAX_RESULTS { break; }

                if let Some(ref gs) = include_glob {
                    let rel = entry.path().strip_prefix(&search_root).unwrap_or(entry.path());
                    if !gs.is_match(rel) { continue; }
                }

                grep_file(entry.path(), &re, &mut results, MAX_RESULTS);
            }
        }

        if results.is_empty() {
            Ok(ToolResult::ok("No matches found"))
        } else {
            Ok(ToolResult::ok(results.join("\n")))
        }
    }
}

fn grep_file(path: &std::path::Path, re: &Regex, results: &mut Vec<String>, max: usize) {
    let Ok(content) = std::fs::read_to_string(path) else { return };
    for (lineno, line) in content.lines().enumerate() {
        if results.len() >= max { break; }
        if re.is_match(line) {
            results.push(format!("{}:{}: {}", path.display(), lineno + 1, line));
        }
    }
}
