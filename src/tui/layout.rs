use crate::tui::{App, ViewType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Command bar
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    // Command bar
    let command_text = if app.command_palette_open {
        format!("/{}", app.command_input)
    } else {
        "Commands: /".to_string()
    };

    let command_bar = Paragraph::new(command_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(command_bar, chunks[0]);

    // Main content area with sidebar
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Sidebar
            Constraint::Min(0),     // Content
        ])
        .split(chunks[1]);

    // Sidebar (navigation)
    render_sidebar(f, app, main_chunks[0]);

    // Main content area
    render_content(f, app, main_chunks[1]);

    // Command palette overlay
    if app.command_palette_open {
        render_command_palette(f, app);
    }

    // Status bar
    render_status_bar(f, app, chunks[2]);
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = vec![
        ("Programs", ViewType::Programs),
        ("Projects", ViewType::Projects),
        ("Milestones", ViewType::Milestones),
        ("Tasks", ViewType::Tasks),
        ("Journal", ViewType::Journal),
        ("Backlog", ViewType::Backlog),
    ]
    .iter()
    .map(|(label, view)| {
        let style = if app.current_view == *view {
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        ListItem::new(*label).style(style)
    })
    .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Navigation"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    let content = match app.current_view {
        ViewType::Programs => "Programs\n\n(No programs yet)",
        ViewType::Projects => "Projects\n\n(No projects yet)",
        ViewType::Milestones => "Milestones\n\n(No milestones yet)",
        ViewType::Tasks => "Tasks\n\n(No tasks yet)",
        ViewType::Journal => "Journal\n\n(No journal entries yet)",
        ViewType::Backlog => "Backlog\n\n(No backlog items)",
    };

    let paragraph = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(match app.current_view {
                    ViewType::Programs => "Programs",
                    ViewType::Projects => "Projects",
                    ViewType::Milestones => "Milestones",
                    ViewType::Tasks => "Tasks",
                    ViewType::Journal => "Journal",
                    ViewType::Backlog => "Backlog",
                }),
        );

    f.render_widget(paragraph, area);
}

fn render_command_palette(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input
            Constraint::Max(10),   // Results
        ])
        .margin(10)
        .split(area);

    // Command input
    let input = Paragraph::new(format!("/{}", app.command_input))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::LightBlue))
                .title("Command"),
        );

    f.render_widget(input, popup[0]);

    // Command results
    let items: Vec<ListItem> = app
        .command_matches
        .iter()
        .map(|cmd| ListItem::new(cmd.label.as_str()).style(Style::default().fg(Color::White)))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, popup[1]);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = format!("Ready | Data: {}", app.config.data_path.display());

    let paragraph = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(paragraph, area);
}
