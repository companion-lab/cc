use opentui_rust::{
    buffer::OptimizedBuffer,
    style::Style,
};

use crate::app::{Message, Role};
use crate::theme::Theme;
use super::panel::draw_panel;

pub fn draw_messages(
    buffer: &mut OptimizedBuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    messages: &[Message],
    scroll_offset: usize,
    theme: &Theme,
) {
    // Draw panel background
    draw_panel(buffer, x, y, width, height, theme);

    let content_x = x + 2;
    let content_width = width.saturating_sub(4);

    // Calculate visible area
    let mut _total_lines: usize = 0;
    let mut message_line_counts: Vec<usize> = Vec::new();

    // First pass: calculate line counts for all messages
    for message in messages {
        let line_count = calculate_message_lines(&message.content, content_width);
        message_line_counts.push(line_count);
        _total_lines += line_count + 1; // +1 for message separator
    }

    // Second pass: render visible messages
    let mut msg_y = y + 1;
    let mut current_scroll = 0;

    for (idx, message) in messages.iter().enumerate() {
        let line_count = message_line_counts[idx];

        // Skip messages above scroll position
        if current_scroll + line_count + 1 <= scroll_offset {
            current_scroll += line_count + 1;
            continue;
        }

        // Check if we've run out of space
        if msg_y >= y + height - 1 {
            break;
        }

        // Render message
        let (prefix, color) = match message.role {
            Role::User => ("You", theme.accent_secondary),
            Role::Assistant => ("c2", theme.accent_primary),
            Role::System => ("● System", theme.accent_warning),
        };

        let prefix_text = format!("{}: ", prefix);
        buffer.draw_text(content_x, msg_y, &prefix_text,
            Style::builder().fg(color).bg(theme.bg_panel).bold().build());

        msg_y += 1;

        // Render message content with word wrapping
        let mut _lines_drawn = 0;
        for line in message.content.lines() {
            if msg_y >= y + height - 1 {
                break;
            }

            let words: Vec<&str> = line.split_whitespace().collect();
            let mut current_line = String::new();
            let mut current_x = content_x;

            for &word in &words {
                let word_len = word.chars().count();
                let can_add = current_line.is_empty() ||
                    (current_line.len() + word_len + 1) <= content_width as usize;

                if can_add {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                } else {
                    if !current_line.is_empty() && msg_y < y + height - 1 {
                        buffer.draw_text(current_x, msg_y, &current_line,
                            Style::builder().fg(theme.text_primary).bg(theme.bg_panel).build());
                        msg_y += 1;
                        _lines_drawn += 1;
                    }
                    current_line = word.to_string();
                    current_x = content_x;

                    if msg_y >= y + height - 1 {
                        break;
                    }
                }
            }

            if !current_line.is_empty() && msg_y < y + height - 1 {
                buffer.draw_text(current_x, msg_y, &current_line,
                    Style::builder().fg(theme.text_primary).bg(theme.bg_panel).build());
                msg_y += 1;
                _lines_drawn += 1;
            }
        }

        // Add separator between messages
        if msg_y < y + height - 1 {
            msg_y += 1;
        }

        current_scroll += line_count + 1;
    }
}

fn calculate_message_lines(content: &str, width: u32) -> usize {
    let mut total_lines = 0;

    for line in content.lines() {
        if line.is_empty() {
            total_lines += 1;
            continue;
        }

        let words: Vec<&str> = line.split_whitespace().collect();
        let mut current_line_len = 0;

        for &word in &words {
            let word_len = word.len();

            if current_line_len == 0 {
                current_line_len = word_len;
            } else if current_line_len + 1 + word_len <= width as usize {
                current_line_len += 1 + word_len;
            } else {
                total_lines += 1;
                current_line_len = word_len;
            }
        }

        if current_line_len > 0 {
            total_lines += 1;
        }
    }

    total_lines
}
