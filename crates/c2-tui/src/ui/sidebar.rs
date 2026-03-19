use opentui_rust::{
    buffer::OptimizedBuffer,
    cell::Cell,
    style::Style,
};

use crate::theme::Theme;
use super::panel::draw_panel;

pub fn draw_sidebar(
    buffer: &mut OptimizedBuffer,
    width: u32,
    height: u32,
    sessions: &[String],
    selected_session: usize,
    theme: &Theme,
) {
    // Draw panel background
    draw_panel(buffer, 0, 0, width, height, theme);

    // Header with accent color
    let header = " Sessions ";
    let header_x = 2;
    buffer.draw_text(header_x, 1, header,
        Style::builder().fg(theme.accent_primary).bg(theme.bg_panel).bold().build());

    // Draw corner connectors
    buffer.set(width - 1, 0, Cell::new('┐', Style::builder().bg(theme.bg_panel).fg(theme.border).build()));
    buffer.set(width - 1, height - 1, Cell::new('┘', Style::builder().bg(theme.bg_panel).fg(theme.border).build()));

    // Draw session list
    let max_visible = (height.saturating_sub(3)) as usize;
    let start_idx = if sessions.len() > max_visible {
        sessions.len().saturating_sub(max_visible)
    } else {
        0
    };

    for (idx, session) in sessions.iter().skip(start_idx).enumerate() {
        let y = 3 + idx as u32;
        if y >= height - 1 {
            break;
        }

        let is_selected = (start_idx + idx) == selected_session;
        let (text_color, bg_color, prefix) = if is_selected {
            (theme.text_primary, theme.bg_highlight, "▸ ")
        } else {
            (theme.text_secondary, theme.bg_panel, "  ")
        };

        let max_text_len = width.saturating_sub(5) as usize;
        let truncated = if session.len() > max_text_len {
            format!("{}…", &session[..max_text_len.saturating_sub(1)])
        } else {
            session.clone()
        };

        let label = format!("{}{}", prefix, truncated);
        buffer.draw_text(1, y, &label,
            Style::builder().fg(text_color).bg(bg_color).build());
    }
}
