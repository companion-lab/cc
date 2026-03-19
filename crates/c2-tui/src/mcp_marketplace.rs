use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMarketplaceServer {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub installs: u64,
    pub command: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub enabled: bool,
}

pub struct McpMarketplace {
    servers: Vec<McpMarketplaceServer>,
}

impl McpMarketplace {
    pub fn new() -> Self {
        Self {
            servers: Self::curated_servers(),
        }
    }

    pub fn servers(&self) -> &[McpMarketplaceServer] {
        &self.servers
    }

    pub fn search(&self, query: &str) -> Vec<&McpMarketplaceServer> {
        let query = query.to_lowercase();
        self.servers
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&query)
                    || s.description.to_lowercase().contains(&query)
                    || s.category.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn by_category(&self, category: &str) -> Vec<&McpMarketplaceServer> {
        self.servers
            .iter()
            .filter(|s| s.category.eq_ignore_ascii_case(category))
            .collect()
    }

    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self.servers.iter().map(|s| s.category.clone()).collect();
        cats.sort();
        cats.dedup();
        cats
    }

    fn curated_servers() -> Vec<McpMarketplaceServer> {
        vec![
            // Developer Tools
            McpMarketplaceServer {
                id: "filesystem".to_string(),
                name: "Filesystem".to_string(),
                description: "Secure file operations with configurable access controls. Read, write, and manage files.".to_string(),
                category: "Developer Tools".to_string(),
                installs: 96665,
                command: vec!["npx".to_string(), "-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "github".to_string(),
                name: "GitHub".to_string(),
                description: "Repository management, issues, PRs, and CI/CD workflows via GitHub API.".to_string(),
                category: "Developer Tools".to_string(),
                installs: 49656,
                command: vec!["npx".to_string(), "-y".to_string(), "@modelcontextprotocol/server-github".to_string()],
                env: [("GITHUB_TOKEN".to_string(), "your-token-here".to_string())].into_iter().collect(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "git".to_string(),
                name: "Git".to_string(),
                description: "Tools to read, search, and manipulate Git repositories locally.".to_string(),
                category: "Developer Tools".to_string(),
                installs: 32080,
                command: vec!["npx".to_string(), "-y".to_string(), "@modelcontextprotocol/server-git".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "sqlite".to_string(),
                name: "SQLite".to_string(),
                description: "Query and analyze SQLite databases with read/write access.".to_string(),
                category: "Database Management".to_string(),
                installs: 38786,
                command: vec!["npx".to_string(), "-y".to_string(), "@modelcontextprotocol/server-sqlite".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "postgres".to_string(),
                name: "PostgreSQL".to_string(),
                description: "Read-only database access with schema inspection capabilities.".to_string(),
                category: "Database Management".to_string(),
                installs: 30109,
                command: vec!["npx".to_string(), "-y".to_string(), "@modelcontextprotocol/server-postgres".to_string()],
                env: [("DATABASE_URL".to_string(), "postgresql://localhost/db".to_string())].into_iter().collect(),
                enabled: false,
            },
            // Browser & Web
            McpMarketplaceServer {
                id: "puppeteer".to_string(),
                name: "Puppeteer".to_string(),
                description: "Browser automation for web scraping, testing, and interaction.".to_string(),
                category: "Browser Automation".to_string(),
                installs: 3194,
                command: vec!["npx".to_string(), "-y".to_string(), "@modelcontextprotocol/server-puppeteer".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "playwright".to_string(),
                name: "Playwright".to_string(),
                description: "Cross-browser automation for testing and web scraping.".to_string(),
                category: "Browser Automation".to_string(),
                installs: 2649,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-playwright".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "fetch".to_string(),
                name: "Fetch".to_string(),
                description: "Web content fetching and conversion for efficient LLM usage.".to_string(),
                category: "Web Scraping".to_string(),
                installs: 4195,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-fetch".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            // Knowledge & Docs
            McpMarketplaceServer {
                id: "memory".to_string(),
                name: "Memory".to_string(),
                description: "Knowledge graph-based persistent memory system for Claude.".to_string(),
                category: "Knowledge".to_string(),
                installs: 25986,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-memory".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "context7".to_string(),
                name: "Context7".to_string(),
                description: "Fetches up-to-date documentation and code examples from source.".to_string(),
                category: "Learning".to_string(),
                installs: 49656,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-context7".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            // Communication
            McpMarketplaceServer {
                id: "slack".to_string(),
                name: "Slack".to_string(),
                description: "Slack workspace integration for messaging and channel management.".to_string(),
                category: "Communication".to_string(),
                installs: 11660,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-slack".to_string()],
                env: [("SLACK_TOKEN".to_string(), "xoxb-your-token".to_string())].into_iter().collect(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "discord".to_string(),
                name: "Discord".to_string(),
                description: "Discord bot integration for messaging and server management.".to_string(),
                category: "Communication".to_string(),
                installs: 7959,
                command: vec!["npx".to_string(), "-y".to_string(), "mcp-discord".to_string()],
                env: [("DISCORD_TOKEN".to_string(), "your-token".to_string())].into_iter().collect(),
                enabled: false,
            },
            // Cloud & Infrastructure
            McpMarketplaceServer {
                id: "aws".to_string(),
                name: "AWS".to_string(),
                description: "AWS services integration for EC2, S3, Lambda, and more.".to_string(),
                category: "Cloud Infrastructure".to_string(),
                installs: 1267,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-aws".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "kubernetes".to_string(),
                name: "Kubernetes".to_string(),
                description: "Kubernetes cluster management and pod inspection.".to_string(),
                category: "Cloud Infrastructure".to_string(),
                installs: 96665,
                command: vec!["npx".to_string(), "-y".to_string(), "mcp-server-kubernetes".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            // Productivity
            McpMarketplaceServer {
                id: "notion".to_string(),
                name: "Notion".to_string(),
                description: "Notion workspace integration for pages, databases, and blocks.".to_string(),
                category: "Productivity".to_string(),
                installs: 4483,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-notion".to_string()],
                env: [("NOTION_API_KEY".to_string(), "your-api-key".to_string())].into_iter().collect(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "google-drive".to_string(),
                name: "Google Drive".to_string(),
                description: "Google Drive file access and management.".to_string(),
                category: "Productivity".to_string(),
                installs: 3479,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-gdrive".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "google-maps".to_string(),
                name: "Google Maps".to_string(),
                description: "Maps, places, directions, and geocoding via Google Maps API.".to_string(),
                category: "Productivity".to_string(),
                installs: 2442,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-google-maps".to_string()],
                env: [("GOOGLE_MAPS_API_KEY".to_string(), "your-api-key".to_string())].into_iter().collect(),
                enabled: false,
            },
            // Data & Analytics
            McpMarketplaceServer {
                id: "sentry".to_string(),
                name: "Sentry".to_string(),
                description: "Error tracking and performance monitoring integration.".to_string(),
                category: "Analytics".to_string(),
                installs: 7509,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-sentry".to_string()],
                env: [("SENTRY_AUTH_TOKEN".to_string(), "your-token".to_string())].into_iter().collect(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "brave-search".to_string(),
                name: "Brave Search".to_string(),
                description: "Web search using Brave Search API with privacy focus.".to_string(),
                category: "Web Scraping".to_string(),
                installs: 7959,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-brave-search".to_string()],
                env: [("BRAVE_API_KEY".to_string(), "your-api-key".to_string())].into_iter().collect(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "arxiv".to_string(),
                name: "ArXiv Research".to_string(),
                description: "Search and fetch academic papers from arXiv with full text extraction.".to_string(),
                category: "Research".to_string(),
                installs: 5118,
                command: vec!["npx".to_string(), "-y".to_string(), "mcp-server-arxiv".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "perplexity".to_string(),
                name: "Perplexity".to_string(),
                description: "AI-powered search with real-time web information and citations.".to_string(),
                category: "Research".to_string(),
                installs: 2649,
                command: vec!["npx".to_string(), "-y".to_string(), "mcp-server-perplexity".to_string()],
                env: [("PERPLEXITY_API_KEY".to_string(), "your-api-key".to_string())].into_iter().collect(),
                enabled: false,
            },
            // System
            McpMarketplaceServer {
                id: "sequential-thinking".to_string(),
                name: "Sequential Thinking".to_string(),
                description: "Dynamic problem-solving through thought sequences and reasoning chains.".to_string(),
                category: "AI Tools".to_string(),
                installs: 25986,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-sequential-thinking".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
            McpMarketplaceServer {
                id: "everything".to_string(),
                name: "Everything".to_string(),
                description: "Test/reference server with prompts, resources, and tools.".to_string(),
                category: "Developer Tools".to_string(),
                installs: 33216,
                command: vec!["npx".to_string(), "-y".to_string(), "@anthropic-ai/mcp-server-everything".to_string()],
                env: std::collections::HashMap::new(),
                enabled: false,
            },
        ]
    }
}
