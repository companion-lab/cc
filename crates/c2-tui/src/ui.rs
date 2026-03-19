use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main layout: Horizontal split for Sidebar and Main Content
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
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
        .map(|s| ListItem::new(Line::from(vec![Span::raw(s)])))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Sessions"))
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_widget(list, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &App) {
    // Vertical split for Chat History and Input box
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(area);

    let messages_area = chunks[0];
    let input_area = chunks[1];

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|m| {
            let content = Line::from(Span::raw(m));
            ListItem::new(content)
        })
        .collect();

    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Chat"));
    f.render_widget(messages_list, messages_area);

    let input_widget = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input (Type message, Enter to send, Esc to exit)"));
    f.render_widget(input_widget, input_area);

    // Make the cursor visible
    f.set_cursor_position((
        input_area.x + app.input.len() as u16 + 1,
        input_area.y + 1,
    ));
}
