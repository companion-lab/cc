use opentui_rust::{
    buffer::OptimizedBuffer,
    cell::Cell,
    color::Rgba,
    style::Style,
};

use crate::theme::Theme;

pub fn draw_status_bar(
    buffer: &mut OptimizedBuffer,
    x: u32,
    y: u32,
    width: u32,
    status: &str,
    is_waiting: bool,
    message_count: usize,
    theme: &Theme,
) {
    // Draw status bar background
    let bg = theme.accent_primary;
    for col in x..x + width {
        buffer.set(col, y, Cell::new(' ', Style::builder().bg(bg).build()));
    }

    // Draw status icon
    let status_icon = if is_waiting { "◌" } else { "●" };
    let status_color = if is_waiting {
        Rgba::from_hex("#000000").unwrap()
    } else {
        theme.bg_dark
    };

    buffer.draw_text(x + 1, y, status_icon,
        Style::builder().fg(status_color).bg(bg).build());

    // Draw status text
    let status_text = format!(" {} │ Messages: {}", status, message_count);
    buffer.draw_text(x + 3, y, &status_text,
        Style::builder().fg(theme.bg_dark).bg(bg).build());

    // Draw version/branding on the right
    let brand = "c2 v0.1.0 ";
    let brand_x = x + width - brand.len() as u32;
    if brand_x > x + status_text.len() as u32 + 5 {
        buffer.draw_text(brand_x, y, brand,
            Style::builder().fg(theme.bg_dark).bg(bg).build());
    }
}

