use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode, Role};

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main layout: Horizontal split for Sidebar and Main Content
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints([Constraint::Length(20), Constraint::Min(0)].as_ref())
        .split(size);

    let sidebar_area = chunks[0];
    let main_area = chunks[1];

    draw_sidebar(f, sidebar_area, app);
    draw_main(f, main_area, app);
}

fn draw_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == app.selected_session {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Line::from(vec![Span::styled(s.clone(), style)])).style(Style::default().bg(Color::Reset))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Sessions ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );

    f.render_widget(list, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &App) {
    // Vertical split for Chat History and Input box
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3), Constraint::Length(1)])
        .split(area);

    let messages_area = chunks[0];
    let input_area = chunks[1];
    let status_area = chunks[2];

    draw_messages(f, messages_area, app);
    draw_input(f, input_area, app);
    draw_status_bar(f, status_area, app);
}

fn draw_messages(f: &mut Frame, area: Rect, app: &App) {
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .flat_map(|msg| {
            let (prefix, style) = match msg.role {
                Role::User => ("You", Style::default().fg(Color::Green)),
                Role::Assistant => ("c2", Style::default().fg(Color::Cyan)),
                Role::System => ("System", Style::default().fg(Color::Yellow)),
            };
            
            let lines: Vec<Line> = msg
                .content
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    // Wrap long lines
                    let wrapped_lines = textwrap::wrap(line, (area.width - 4) as usize);
                    wrapped_lines.into_iter().enumerate().map(|(wrap_idx, wrapped_line)| {
                        if i == 0 && wrap_idx == 0 {
                            Line::from(vec![
                                Span::styled(format!("{}: ", prefix), Style::default().add_modifier(Modifier::BOLD)),
                                Span::styled(wrapped_line.to_string(), style),
                            ])
                        } else {
                            Line::from(vec![Span::styled(wrapped_line.to_string(), style)])
                        }
                    }).collect::<Vec<_>>()
                })
                .flatten()
                .collect();
            
            lines.into_iter().map(ListItem::new).collect::<Vec<_>>()
        })
        .collect();

    let messages_list = List::new(messages)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .padding(ratatui::widgets::Padding::new(1, 1, 0, 0)),
        );
    
    f.render_widget(messages_list, area);
}

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let input_style = match app.mode {
        AppMode::Input => Style::default().fg(Color::White),
        AppMode::Waiting => Style::default().fg(Color::Gray),
    };

    let title = match app.mode {
        AppMode::Input => " Input (Enter to send, Esc to exit) ",
        AppMode::Waiting => " Waiting for response... (Ctrl+C to cancel) ",
    };

    let input_widget = Paragraph::new(app.input.as_str())
        .style(input_style)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title)
                .title_style(Style::default().fg(Color::Cyan)),
        );

    f.render_widget(input_widget, area);

    if app.mode == AppMode::Input {
        f.set_cursor_position((
            area.x + app.input.len() as u16 + 1,
            area.y + 1,
        ));
    }
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let status_style = match app.mode {
        AppMode::Input => Style::default().fg(Color::Green),
        AppMode::Waiting => Style::default().fg(Color::Yellow),
    };

    let status_text = format!("● {} | Messages: {}", app.status, app.messages.len());
    let status_bar = Paragraph::new(status_text)
        .style(status_style)
        .block(Block::default());

    f.render_widget(status_bar, area);
}
