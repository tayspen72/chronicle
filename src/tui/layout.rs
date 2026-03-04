use super::views;
use crate::tui::{App, Mode, ViewType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
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
    let command_text = if matches!(app.mode, Mode::CommandPalette) {
        format!("/{}", app.command_input)
    } else {
        "Commands: /".to_string()
    };

    let command_bar = Paragraph::new(command_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(command_bar, chunks[0]);

    // Calculate dynamic sidebar width based on content
    let sidebar_width = calculate_sidebar_width(app);

    // Main content area with sidebar
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(sidebar_width), // Sidebar - dynamic width
            Constraint::Min(0),                // Content
        ])
        .split(chunks[1]);

    // Sidebar (navigation)
    render_sidebar(f, app, main_chunks[0]);

    // Main content area
    render_content(f, app, main_chunks[1]);

    // Command palette overlay
    if matches!(app.mode, Mode::CommandPalette) {
        render_command_palette(f, app);
    }

    // Status bar
    render_status_bar(f, app, chunks[2]);
}

fn calculate_sidebar_width(app: &App) -> u16 {
    let mut max_len = 0usize;

    max_len = max_len.max("Navigator".len());

    for item in &app.sidebar_items {
        let len = item.name.len() + (item.indent * 4);
        max_len = max_len.max(len);
    }

    (max_len + 4).clamp(15, 60) as u16
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let idx = app.selected_entry_index;

    let items: Vec<ListItem> = app
        .sidebar_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == idx;
            let indent_str = "    ".repeat(item.indent);

            let prefix = if item.is_header || item.indent == 0 {
                item.name.clone()
            } else {
                let is_last = app
                    .sidebar_items
                    .iter()
                    .skip(i + 1)
                    .take_while(|p| p.indent == item.indent)
                    .next()
                    .is_none();
                if is_last {
                    format!("└── {}", item.name)
                } else {
                    format!("├── {}", item.name)
                }
            };

            let full_label = format!("{}{}", indent_str, prefix);

            let style = if item.is_header {
                Style::default().fg(Color::DarkGray)
            } else if item.is_create_action {
                // Style create action items with dimmed cyan to indicate it's an action
                if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(ratatui::style::Modifier::ITALIC)
                } else {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(ratatui::style::Modifier::ITALIC)
                }
            } else if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(full_label).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Navigator"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    match app.current_view {
        ViewType::TreeView => views::render_tree_view(f, app, area),
        ViewType::Journal => views::render_journal_welcome(f, app, area),
        ViewType::JournalArchiveList => views::render_archive_list(f, app, area),
        ViewType::JournalToday => views::render_journal_today(f, app, area),
        ViewType::Backlog => views::render_backlog(f, app, area),
        ViewType::WeeklyPlanning => views::render_weekly_planning(f, app, area),
        ViewType::ViewingContent => views::render_content_viewer(f, app, area),
        ViewType::InputProgram => views::render_input(f, app, area, "Enter program name:"),
        ViewType::InputProject => views::render_input(f, app, area, "Enter project name:"),
        ViewType::InputMilestone => views::render_input(f, app, area, "Enter milestone name:"),
        ViewType::InputTask => views::render_input(f, app, area, "Enter task name:"),
        ViewType::InputTemplateField => {
            if let Some(ref state) = app.template_field_state {
                let prompt = format!("Fill in fields for: {}", state.template_name);
                views::render_template_fields(f, app, area, &prompt);
            } else {
                views::render_input(f, app, area, "Enter value:");
            }
        }
    }
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

    // Clear the background behind the popup
    f.render_widget(Clear, area);

    // Command input
    let input = Paragraph::new(format!("/{}", app.command_input))
        .style(Style::default().fg(Color::White).bg(Color::Black))
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
        .enumerate()
        .map(|(idx, cmd)| {
            let style = if idx == app.command_selection_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).bg(Color::Black)
            };
            ListItem::new(cmd.label.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black));

    f.render_widget(list, popup[1]);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    // Build breadcrumb from current selection
    let mut breadcrumb_parts = Vec::new();

    if let Some(program) = &app.current_program {
        breadcrumb_parts.push(program.clone());
    }
    if let Some(project) = &app.current_project {
        breadcrumb_parts.push(project.clone());
    }
    if let Some(milestone) = &app.current_milestone {
        breadcrumb_parts.push(milestone.clone());
    }
    if let Some(task) = &app.current_task {
        breadcrumb_parts.push(task.clone());
    }

    let breadcrumb = if breadcrumb_parts.is_empty() {
        "No selection".to_string()
    } else {
        breadcrumb_parts.join(" > ")
    };

    // Determine mode text and color
    let (mode_text, mode_color) = match app.mode {
        Mode::Normal => ("NORMAL", Color::Green),
        Mode::CommandPalette => ("COMMAND", Color::Yellow),
        Mode::Input => ("INPUT", Color::Cyan),
    };

    // Split the status bar into left (breadcrumb) and right (mode) sections
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),     // Breadcrumb - takes remaining space
            Constraint::Length(10), // Mode indicator
        ])
        .split(area);

    // Render breadcrumb (left side)
    let breadcrumb_widget = Paragraph::new(breadcrumb)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(breadcrumb_widget, chunks[0]);

    // Render mode indicator (right side)
    let mode_widget = Paragraph::new(mode_text)
        .style(Style::default().fg(mode_color))
        .block(Block::default().borders(Borders::NONE))
        .alignment(ratatui::layout::Alignment::Right);
    f.render_widget(mode_widget, chunks[1]);
}
