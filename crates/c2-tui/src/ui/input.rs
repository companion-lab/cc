use opentui_rust::{
    buffer::OptimizedBuffer,
    cell::Cell,
    style::Style,
};

use crate::theme::Theme;
use super::panel::draw_panel;

pub fn draw_input(
    buffer: &mut OptimizedBuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    input_text: &str,
    is_waiting: bool,
    theme: &Theme,
) {
    // Draw panel background
    draw_panel(buffer, x, y, width, height, theme);

    // Draw input prompt
    let prompt = if is_waiting { "◌ " } else { "● " };
    let prompt_color = if is_waiting { theme.accent_warning } else { theme.accent_primary };

    buffer.draw_text(x + 1, y + 1, prompt,
        Style::builder().fg(prompt_color).bg(theme.bg_panel).build());

    // Draw input text
    let max_text_len = width.saturating_sub(4) as usize;
    let display_text = if input_text.len() > max_text_len {
        format!("{}…", &input_text[input_text.len() - max_text_len + 1..])
    } else {
        input_text.to_string()
    };

    buffer.draw_text(x + 3, y + 1, &display_text,
        Style::builder().fg(theme.text_primary).bg(theme.bg_panel).build());

    // Draw cursor
    if !is_waiting {
        let cursor_x = x + 3 + display_text.len() as u32;
        if cursor_x < x + width - 1 {
            buffer.set(cursor_x, y + 1, Cell::new('█',
                Style::builder().fg(theme.accent_primary).bg(theme.bg_panel).build()));
        }
    }
}

pub fn draw_input_help(
    buffer: &mut OptimizedBuffer,
    x: u32,
    y: u32,
    width: u32,
    theme: &Theme,
) {
    // Clear help area
    for col in x..x + width {
        buffer.set(col, y, Cell::new(' ', Style::builder().bg(theme.bg_dark).build()));
    }

    // Draw help text
    let help = " Enter: Send │ Esc: Cancel │ Ctrl+C: Exit ";
    let help_x = x + width / 2 - help.len() as u32 / 2;

    if help_x >= x {
        buffer.draw_text(help_x, y, help,
            Style::builder().fg(theme.text_muted).bg(theme.bg_dark).build());
    }
}
