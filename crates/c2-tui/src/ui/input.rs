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
    let prompt = if is_waiting { "◌ " } else { "❯ " };
    let prompt_color = if is_waiting { theme.accent_warning } else { theme.accent_primary };
    let prompt_width = 2u32;

    buffer.draw_text(x + 1, y + 1, prompt,
        Style::builder().fg(prompt_color).bg(theme.bg_panel).build());

    // Draw input text with proper truncation
    let max_text_len = width.saturating_sub(prompt_width + 3) as usize;
    let display_text = if input_text.len() > max_text_len {
        format!("…{}", &input_text[input_text.len() - max_text_len + 1..])
    } else {
        input_text.to_string()
    };

    let text_x = x + 1 + prompt_width;
    buffer.draw_text(text_x, y + 1, &display_text,
        Style::builder().fg(theme.text_primary).bg(theme.bg_panel).build());

    // Draw cursor (blinking block) - positioned right after the text
    if !is_waiting {
        let cursor_x = text_x + display_text.len() as u32;
        if cursor_x < x + width - 1 {
            // Use a block character for the cursor
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
