use opentui_rust::{
    buffer::OptimizedBuffer,
    cell::Cell,
    color::Rgba,
    style::Style,
};

use crate::theme::Theme;

pub fn draw_panel(
    buffer: &mut OptimizedBuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    theme: &Theme,
) {
    if width < 2 || height < 2 {
        return;
    }

    // Top border
    buffer.set(x, y, Cell::new('┌', Style::builder().fg(theme.border).bg(theme.bg_panel).build()));
    for col in x + 1..x + width - 1 {
        buffer.set(col, y, Cell::new('─', Style::builder().fg(theme.border).bg(theme.bg_panel).build()));
    }
    buffer.set(x + width - 1, y, Cell::new('┐', Style::builder().fg(theme.border).bg(theme.bg_panel).build()));

    // Bottom border
    buffer.set(x, y + height - 1, Cell::new('└', Style::builder().fg(theme.border).bg(theme.bg_panel).build()));
    for col in x + 1..x + width - 1 {
        buffer.set(col, y + height - 1, Cell::new('─', Style::builder().fg(theme.border).bg(theme.bg_panel).build()));
    }
    buffer.set(x + width - 1, y + height - 1, Cell::new('┘', Style::builder().fg(theme.border).bg(theme.bg_panel).build()));

    // Side borders and fill
    for row in y + 1..y + height - 1 {
        buffer.set(x, row, Cell::new('│', Style::builder().fg(theme.border).bg(theme.bg_panel).build()));
        buffer.set(x + width - 1, row, Cell::new('│', Style::builder().fg(theme.border).bg(theme.bg_panel).build()));

        for col in x + 1..x + width - 1 {
            buffer.set(col, row, Cell::new(' ', Style::builder().bg(theme.bg_panel).build()));
        }
    }
}

pub fn draw_box_with_bg(
    buffer: &mut OptimizedBuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    bg: Rgba,
) {
    for row in y..y + height {
        for col in x..x + width {
            buffer.set(col, row, Cell::new(' ', Style::builder().bg(bg).build()));
        }
    }
}
