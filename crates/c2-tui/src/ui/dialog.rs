use opentui_rust::{
    buffer::OptimizedBuffer,
    cell::Cell,
    style::Style,
};

use crate::app::{AppState, DialogMode, McpStatus};
use crate::theme::Theme;

pub fn draw_dialog(buffer: &mut OptimizedBuffer, app: &AppState, theme: &Theme) {
    let width = buffer.width();
    let height = buffer.height();

    // Calculate dialog dimensions
    let dialog_width = std::cmp::min(60, width.saturating_sub(4));
    let dialog_height = std::cmp::min(20, height.saturating_sub(4));
    let dialog_x = (width - dialog_width) / 2;
    let dialog_y = (height - dialog_height) / 2;

    // Draw dialog background
    for row in dialog_y..dialog_y + dialog_height {
        for col in dialog_x..dialog_x + dialog_width {
            buffer.set(col, row, Cell::new(' ', Style::builder().bg(theme.bg_panel).build()));
        }
    }

    // Draw dialog border
    draw_dialog_border(buffer, dialog_x, dialog_y, dialog_width, dialog_height, theme);

    // Draw dialog title
    let title = match app.dialog_mode {
        DialogMode::ModelSelect => " Select Model ",
        DialogMode::AgentSelect => " Select Agent ",
        DialogMode::McpManager => " MCP Servers ",
        DialogMode::None => "",
    };

    let title_x = dialog_x + (dialog_width - title.len() as u32) / 2;
    buffer.draw_text(title_x, dialog_y, title,
        Style::builder().fg(theme.accent_primary).bg(theme.bg_panel).bold().build());

    // Draw filter input
    let filter_y = dialog_y + 2;
    let filter_label = "Search: ";
    buffer.draw_text(dialog_x + 2, filter_y, filter_label,
        Style::builder().fg(theme.text_secondary).bg(theme.bg_panel).build());

    let filter_x = dialog_x + 2 + filter_label.len() as u32;
    let filter_display = if app.dialog_filter.is_empty() {
        "Type to filter...".to_string()
    } else {
        app.dialog_filter.clone()
    };
    buffer.draw_text(filter_x, filter_y, &filter_display,
        Style::builder()
            .fg(if app.dialog_filter.is_empty() { theme.text_muted } else { theme.text_primary })
            .bg(theme.bg_panel)
            .build());

    // Draw cursor
    if !app.dialog_filter.is_empty() {
        let cursor_x = filter_x + app.dialog_filter.len() as u32;
        if cursor_x < dialog_x + dialog_width - 2 {
            buffer.set(cursor_x, filter_y, Cell::new('█',
                Style::builder().fg(theme.accent_primary).bg(theme.bg_panel).build()));
        }
    }

    // Draw items
    let items_y = dialog_y + 4;
    let max_items = (dialog_height.saturating_sub(6)) as usize;

    match app.dialog_mode {
        DialogMode::ModelSelect => {
            draw_model_list(buffer, app, dialog_x, items_y, dialog_width, max_items, theme);
        }
        DialogMode::AgentSelect => {
            draw_agent_list(buffer, app, dialog_x, items_y, dialog_width, max_items, theme);
        }
        DialogMode::McpManager => {
            draw_mcp_list(buffer, app, dialog_x, items_y, dialog_width, max_items, theme);
        }
        DialogMode::None => {}
    }

    // Draw help text at bottom
    let help_y = dialog_y + dialog_height - 2;
    let help = "↑/↓: Navigate │ Enter: Select │ Esc: Close";
    let help_x = dialog_x + (dialog_width - help.len() as u32) / 2;
    if help_x >= dialog_x && help_x + help.len() as u32 <= dialog_x + dialog_width {
        buffer.draw_text(help_x, help_y, help,
            Style::builder().fg(theme.text_muted).bg(theme.bg_panel).build());
    }
}

fn draw_dialog_border(
    buffer: &mut OptimizedBuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    theme: &Theme,
) {
    // Top border
    buffer.set(x, y, Cell::new('┌', Style::builder().fg(theme.border_focus).bg(theme.bg_panel).build()));
    for col in x + 1..x + width - 1 {
        buffer.set(col, y, Cell::new('─', Style::builder().fg(theme.border_focus).bg(theme.bg_panel).build()));
    }
    buffer.set(x + width - 1, y, Cell::new('┐', Style::builder().fg(theme.border_focus).bg(theme.bg_panel).build()));

    // Bottom border
    buffer.set(x, y + height - 1, Cell::new('└', Style::builder().fg(theme.border_focus).bg(theme.bg_panel).build()));
    for col in x + 1..x + width - 1 {
        buffer.set(col, y + height - 1, Cell::new('─', Style::builder().fg(theme.border_focus).bg(theme.bg_panel).build()));
    }
    buffer.set(x + width - 1, y + height - 1, Cell::new('┘', Style::builder().fg(theme.border_focus).bg(theme.bg_panel).build()));

    // Side borders
    for row in y + 1..y + height - 1 {
        buffer.set(x, row, Cell::new('│', Style::builder().fg(theme.border_focus).bg(theme.bg_panel).build()));
        buffer.set(x + width - 1, row, Cell::new('│', Style::builder().fg(theme.border_focus).bg(theme.bg_panel).build()));
    }
}

fn draw_model_list(
    buffer: &mut OptimizedBuffer,
    app: &AppState,
    x: u32,
    y: u32,
    width: u32,
    max_items: usize,
    theme: &Theme,
) {
    let models = app.get_filtered_models();

    if models.is_empty() {
        buffer.draw_text(x + 2, y, "No models available",
            Style::builder().fg(theme.text_muted).bg(theme.bg_panel).build());
        return;
    }

    for (idx, model) in models.iter().enumerate().take(max_items) {
        let item_y = y + idx as u32;
        let is_selected = idx == app.model_dialog_selection;
        let is_current = model.model_id == app.current_model.model_id && model.provider_id == app.current_model.provider_id;

        let bg = if is_selected { theme.bg_highlight } else { theme.bg_panel };
        let fg = if is_current { theme.accent_primary } else if is_selected { theme.text_primary } else { theme.text_secondary };

        // Clear row
        for col in x + 1..x + width - 1 {
            buffer.set(col, item_y, Cell::new(' ', Style::builder().bg(bg).build()));
        }

        // Draw prefix
        let prefix = if is_selected { "▸ " } else { "  " };
        buffer.draw_text(x + 2, item_y, prefix,
            Style::builder().fg(fg).bg(bg).build());

        // Draw model name
        let name = format!("{}{}", if is_current { "● " } else { "  " }, model.name);
        buffer.draw_text(x + 4, item_y, &name,
            Style::builder().fg(fg).bg(bg).build());

        // Draw provider on right
        let provider_text = format!("{} ", model.provider_id);
        let provider_x = x + width - provider_text.len() as u32 - 2;
        if provider_x > x + 4 + name.len() as u32 + 2 {
            buffer.draw_text(provider_x, item_y, &provider_text,
                Style::builder().fg(theme.text_muted).bg(bg).build());
        }
    }
}

fn draw_agent_list(
    buffer: &mut OptimizedBuffer,
    app: &AppState,
    x: u32,
    y: u32,
    width: u32,
    max_items: usize,
    theme: &Theme,
) {
    let agents = app.get_filtered_agents();

    if agents.is_empty() {
        buffer.draw_text(x + 2, y, "No agents available",
            Style::builder().fg(theme.text_muted).bg(theme.bg_panel).build());
        return;
    }

    for (idx, agent) in agents.iter().enumerate().take(max_items) {
        let item_y = y + idx as u32;
        let is_selected = idx == app.agent_dialog_selection;
        let is_current = agent.name == app.current_agent.name;

        let bg = if is_selected { theme.bg_highlight } else { theme.bg_panel };
        let fg = if is_current { theme.accent_primary } else if is_selected { theme.text_primary } else { theme.text_secondary };

        // Clear row
        for col in x + 1..x + width - 1 {
            buffer.set(col, item_y, Cell::new(' ', Style::builder().bg(bg).build()));
        }

        // Draw prefix
        let prefix = if is_selected { "▸ " } else { "  " };
        buffer.draw_text(x + 2, item_y, prefix,
            Style::builder().fg(fg).bg(bg).build());

        // Draw agent name
        let name = format!("{}{}", if is_current { "● " } else { "  " }, agent.name);
        buffer.draw_text(x + 4, item_y, &name,
            Style::builder().fg(fg).bg(bg).build());

        // Draw description truncated
        let max_desc = width.saturating_sub(name.len() as u32 + 8) as usize;
        let desc = if agent.description.len() > max_desc {
            format!("{}…", &agent.description[..max_desc.saturating_sub(1)])
        } else {
            agent.description.clone()
        };
        let desc_x = x + 4 + name.len() as u32 + 2;
        if desc_x < x + width - 2 {
            buffer.draw_text(desc_x, item_y, &desc,
                Style::builder().fg(theme.text_muted).bg(bg).build());
        }
    }
}

fn draw_mcp_list(
    buffer: &mut OptimizedBuffer,
    app: &AppState,
    x: u32,
    y: u32,
    width: u32,
    max_items: usize,
    theme: &Theme,
) {
    let servers = app.get_filtered_mcp_servers();

    if servers.is_empty() {
        buffer.draw_text(x + 2, y, "No MCP servers configured",
            Style::builder().fg(theme.text_muted).bg(theme.bg_panel).build());
        return;
    }

    for (idx, server_name) in servers.iter().enumerate().take(max_items) {
        let item_y = y + idx as u32;
        let is_selected = idx == app.mcp_dialog_selection;

        let bg = if is_selected { theme.bg_highlight } else { theme.bg_panel };
        let fg = if is_selected { theme.text_primary } else { theme.text_secondary };

        // Clear row
        for col in x + 1..x + width - 1 {
            buffer.set(col, item_y, Cell::new(' ', Style::builder().bg(bg).build()));
        }

        // Draw prefix
        let prefix = if is_selected { "▸ " } else { "  " };
        buffer.draw_text(x + 2, item_y, prefix,
            Style::builder().fg(fg).bg(bg).build());

        // Draw server name
        buffer.draw_text(x + 4, item_y, server_name,
            Style::builder().fg(fg).bg(bg).build());

        // Draw status on right
        if let Some(server) = app.mcp_servers.get(server_name) {
            let (status_text, status_color) = match server.status {
                McpStatus::Connected => ("● Connected", theme.accent_secondary),
                McpStatus::Disconnected => ("○ Disconnected", theme.text_muted),
                McpStatus::Failed => ("✕ Failed", theme.accent_warning),
                McpStatus::Loading => ("◌ Loading...", theme.text_muted),
            };

            let status_len = status_text.len() as u32;
            let status_x = x + width - status_len - 3;
            if status_x > x + 4 + server_name.len() as u32 + 2 {
                buffer.draw_text(status_x, item_y, status_text,
                    Style::builder().fg(status_color).bg(bg).build());
            }
        }
    }

    // Draw toggle hint
    if !servers.is_empty() {
        let hint_y = y + std::cmp::min(servers.len(), max_items) as u32 + 1;
        if hint_y < y + max_items as u32 + 2 {
            buffer.draw_text(x + 2, hint_y, "Space: Toggle │ Enter: Toggle",
                Style::builder().fg(theme.text_muted).bg(theme.bg_panel).build());
        }
    }
}
