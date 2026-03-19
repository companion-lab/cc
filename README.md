# c2

A Rust-based AI coding assistant with a modern terminal UI.

Inspired by [opencode](https://github.com/anomalyco/opencode) — TUI patterns, theming, and agent architecture adapted for Rust.

## Features

- **Modern TUI** — Dark theme, modular components, responsive layout
- **Model Selection** — Browse models from [models.dev](https://models.dev), including OpenRouter free models
- **Agent Modes** — Switch between build, plan, and research agents
- **MCP Marketplace** — Browse and install MCP servers from [mcpmarket.com](https://mcpmarket.com)
- **Web Tools** — `web_search` for arxiv papers, `web_fetch` for URL content
- **Subagents** — Spawn focused agents for research, exploration, or coding tasks

## Quick Start

```bash
# Configure
cp .c2/config.json.example .c2/config.json
# Edit with your API key

# Run
c2
```

## Shortcuts

| Key | Action |
|-----|--------|
| `/` | Command palette |
| `Ctrl+M` | Switch model |
| `Tab` / `Shift+Tab` | Cycle agents |
| `Ctrl+T` | MCP servers |
| `Space` | Toggle in marketplace |
| `Enter` | Send / Select |
| `Esc` | Close dialog |

## Commands

| Command | Description |
|---------|-------------|
| `/model` | Switch model |
| `/agent` | Switch agent |
| `/research` | Enter research mode |
| `/mcp` | Manage MCP servers |
| `/marketplace` | Browse MCP marketplace |
| `/clear` | Clear conversation |
| `/help` | Show help |

## Configuration

Config: `~/.config/c2/config.json`

```json
{
  "provider": {
    "id": "openai-compatible",
    "base_url": "https://openrouter.ai/api/v1/chat/completions",
    "api_key": "sk-..."
  },
  "model": "anthropic/claude-sonnet-4"
}
```

## Credits

TUI architecture and patterns from [opencode](https://github.com/anomalyco/opencode).
