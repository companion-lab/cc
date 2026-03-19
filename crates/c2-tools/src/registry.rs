use crate::tool::Tool;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new() };
        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        self.register(Arc::new(crate::bash::BashTool));
        self.register(Arc::new(crate::read::ReadTool));
        self.register(Arc::new(crate::write::WriteTool));
        self.register(Arc::new(crate::edit::EditTool));
        self.register(Arc::new(crate::glob::GlobTool));
        self.register(Arc::new(crate::grep::GrepTool));
        self.register(Arc::new(crate::ls::LsTool));
        self.register(Arc::new(crate::web_fetch::WebFetchTool));
        self.register(Arc::new(crate::web_search::WebSearchTool));
        self.register(Arc::new(crate::subagent::SubagentTool));
        self.register(Arc::new(crate::todo::TodoWriteTool));
        self.register(Arc::new(crate::todo::TodoReadTool));
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn list(&self) -> Vec<&Arc<dyn Tool>> {
        self.tools.values().collect()
    }

    pub fn definitions(&self) -> Vec<c2_provider::ToolDefinition> {
        self.tools
            .values()
            .map(|t| c2_provider::ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                schema: t.schema(),
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
