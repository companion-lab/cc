use opentui_rust::{
    buffer::OptimizedBuffer,
    style::Style,
};

use crate::theme::Theme;

pub fn draw_header(
    buffer: &mut OptimizedBuffer,
    x: u32,
    y: u32,
    width: u32,
    title: &str,
    mode: &str,
    theme: &Theme,
) {
    // Clear header area
    for col in x..x + width {
        buffer.set(col, y, opentui_rust::cell::Cell::new(' ', Style::builder().bg(theme.bg_panel).build()));
    }

    // Draw title
    let display_title = if title.len() > (width as usize).saturating_sub(20) {
        format!("{}…", &title[..(width as usize).saturating_sub(21)])
    } else {
        title.to_string()
    };

    buffer.draw_text(x + 1, y, &display_title,
        Style::builder().fg(theme.text_primary).bg(theme.bg_panel).bold().build());

    // Draw mode indicator on the right
    let mode_text = format!(" {} ", mode);
    let mode_x = x + width - mode_text.len() as u32 - 1;
    if mode_x > x + display_title.len() as u32 + 3 {
        buffer.draw_text(mode_x, y, &mode_text,
            Style::builder().fg(theme.accent_warning).bg(theme.bg_panel).build());
    }
}
