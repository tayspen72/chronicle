use crate::storage::{JournalStorage, WorkspaceStorage};
use crate::tui::{App, DateInputPart, Mode, ViewType};
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
        ViewType::TreeView => render_tree_view(f, app, area),
        ViewType::Journal => render_journal_welcome(f, app, area),
        ViewType::JournalArchiveList => render_archive_list(f, app, area),
        ViewType::JournalToday => render_journal_today(f, app, area),
        ViewType::Backlog => render_placeholder(f, area, "Backlog", "No backlog items"),
        ViewType::WeeklyPlanning => render_placeholder(f, area, "Weekly Planning", "Coming soon"),
        ViewType::ViewingContent => render_content_viewer(f, app, area),
        ViewType::InputProgram => render_input(f, app, area, "Enter program name:"),
        ViewType::InputProject => render_input(f, app, area, "Enter project name:"),
        ViewType::InputMilestone => render_input(f, app, area, "Enter milestone name:"),
        ViewType::InputTask => render_input(f, app, area, "Enter task name:"),
        ViewType::InputTemplateField => {
            let prompt = if let Some(ref state) = app.template_field_state {
                if let Some((ref label, _, _)) = state.fields.get(state.current_index) {
                    let date_part = state.date_part.as_ref();
                    match date_part {
                        Some(DateInputPart::Year) => format!("{} - Year (YYYY):", label),
                        Some(DateInputPart::Month) => format!("{} - Month (MM):", label),
                        Some(DateInputPart::Day) => format!("{} - Day (DD):", label),
                        None => format!("{}:", label),
                    }
                } else {
                    "Enter value:".to_string()
                }
            } else {
                "Enter value:".to_string()
            };
            render_input(f, app, area, &prompt);
        }
    }
}

fn render_placeholder(f: &mut Frame, area: Rect, title: &str, message: &str) {
    let content = format!("{}\n\n({})", title, message);
    let paragraph = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        );
    f.render_widget(paragraph, area);
}

fn render_tree_view(f: &mut Frame, app: &App, area: Rect) {
    let idx = app.selected_entry_index;
    let mut content_to_show = "No item selected".to_string();
    let mut title = "Empty".to_string();

    if idx < app.sidebar_items.len() {
        let item = &app.sidebar_items[idx];

        if !item.is_header && !item.name.is_empty() {
            if let Some(journal_action) = &item.is_journal_item {
                match journal_action.as_str() {
                    "Today" => {
                        title = "Today".to_string();
                        if let Ok((_, content)) =
                            app.config.data_path.open_or_create_today_journal()
                        {
                            content_to_show = content;
                        } else {
                            content_to_show = "No journal entry for today".to_string();
                        }
                    }
                    "History" => {
                        title = "Journal History".to_string();
                        let entries = app
                            .config
                            .data_path
                            .list_journal_entries()
                            .unwrap_or_default();
                        if entries.is_empty() {
                            content_to_show = "No journal entries found".to_string();
                        } else {
                            content_to_show = entries
                                .iter()
                                .map(|e| e.filename.trim_end_matches(".md").to_string())
                                .collect::<Vec<_>>()
                                .join("\n");
                        }
                    }
                    _ => {}
                }
            } else if let Some(path) = &item.path {
                title = item.name.clone();
                content_to_show = app.config.data_path.read_md_file(path).unwrap_or_else(|_| {
                    format!("# {}\n\n(No content or file not found)", item.name)
                });
            }
        }
    }

    let paragraph = Paragraph::new(content_to_show)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        );

    f.render_widget(paragraph, area);
}

fn render_content_viewer(f: &mut Frame, app: &App, area: Rect) {
    let title = app
        .selected_content
        .as_ref()
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "Content".to_string());

    let content = app
        .current_content_text
        .clone()
        .unwrap_or_else(|| "No content".to_string());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title);

    f.render_widget(block, area);

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let paragraph = Paragraph::new(content.as_str()).style(Style::default().fg(Color::White));
    f.render_widget(paragraph, inner_area);
}

fn render_journal_welcome(f: &mut Frame, _app: &App, area: Rect) {
    let content = "Journal\n\n\
        Welcome to your journal!\n\n\
        Type /journal to access:\n\
          - Open Today's Journal\n\
          - Journal History\n\n\
        Press / to open command palette";

    let paragraph = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Journal"),
        );
    f.render_widget(paragraph, area);
}

fn render_journal_today(f: &mut Frame, app: &App, area: Rect) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let content = format!("Today's Journal\n\n{}", app.input_buffer);

    let paragraph = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!("Journal - {}", today)),
        );
    f.render_widget(paragraph, area);
}

// TODO: These render functions are extracted for future dedicated list views.
// Currently the tree view handles all navigation, but these will be useful
// when implementing separate list views for each tier.
#[allow(dead_code)]
fn render_programs_list(f: &mut Frame, app: &App, area: Rect) {
    let title = "Programs";

    if app.programs.is_empty() {
        render_input(f, app, area, "No programs yet. Type to create one:");
        return;
    }

    let items: Vec<ListItem> = app
        .programs
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == app.selected_entry_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(entry.name.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

#[allow(dead_code)]
fn render_projects_list(f: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(ref program) = app.current_program {
        format!("Projects - {}", program)
    } else {
        "Projects".to_string()
    };

    if app.projects.is_empty() {
        let content = format!(
            "{}\n\n(No projects yet. Use /new project to create one.)",
            title
        );
        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(title),
            );
        f.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == app.selected_entry_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(entry.name.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

#[allow(dead_code)]
fn render_milestones_list(f: &mut Frame, app: &App, area: Rect) {
    let title = if let (Some(ref program), Some(ref project)) =
        (&app.current_program, &app.current_project)
    {
        format!("Milestones - {}/{}", program, project)
    } else {
        "Milestones".to_string()
    };

    if app.milestones.is_empty() {
        let content = format!(
            "{}\n\n(No milestones yet. Use /new milestone to create one.)",
            title
        );
        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(title),
            );
        f.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .milestones
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == app.selected_entry_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(entry.name.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

#[allow(dead_code)]
fn render_tasks_list(f: &mut Frame, app: &App, area: Rect) {
    let title = if let (Some(ref program), Some(ref project), Some(ref milestone)) = (
        &app.current_program,
        &app.current_project,
        &app.current_milestone,
    ) {
        format!("Tasks - {}/{}/{}", program, project, milestone)
    } else {
        "Tasks".to_string()
    };

    if app.tasks.is_empty() {
        let content = format!("{}\n\n(No tasks yet. Use /new task to create one.)", title);
        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(title),
            );
        f.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == app.selected_entry_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(entry.name.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_input(f: &mut Frame, app: &App, area: Rect, prompt: &str) {
    let content = format!(
        "{}\n\n> {}\n\nPress Enter to confirm, Esc to cancel",
        prompt, app.input_buffer
    );
    let paragraph = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Input"),
        );
    f.render_widget(paragraph, area);
}

fn render_archive_list(f: &mut Frame, app: &App, area: Rect) {
    if app.journal_entries.is_empty() {
        let paragraph = Paragraph::new("No journal entries found.")
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title("Journal History"),
            );
        f.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .journal_entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == app.selected_entry_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let label = entry.filename.trim_end_matches(".md");
            ListItem::new(label.to_string()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Journal History"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
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
    let depth = app.tree_state.path.len();
    let idx = app.selected_entry_index;

    let selected_path = if depth == 0 {
        app.programs.get(idx).map(|e| e.path.clone())
    } else if depth == 1 {
        let prog_count = app.programs.len();
        if idx < prog_count {
            app.programs.get(idx).map(|e| e.path.clone())
        } else {
            app.projects.get(idx - prog_count).map(|e| e.path.clone())
        }
    } else if depth == 2 {
        let prog_count = app.programs.len();
        let proj_count = app.projects.len();
        if idx < prog_count {
            app.programs.get(idx).map(|e| e.path.clone())
        } else if idx < prog_count + proj_count {
            app.projects.get(idx - prog_count).map(|e| e.path.clone())
        } else {
            app.milestones
                .get(idx - prog_count - proj_count)
                .map(|e| e.path.clone())
        }
    } else {
        let prog_count = app.programs.len();
        let proj_count = app.projects.len();
        let mile_count = app.milestones.len();
        if idx < prog_count {
            app.programs.get(idx).map(|e| e.path.clone())
        } else if idx < prog_count + proj_count {
            app.projects.get(idx - prog_count).map(|e| e.path.clone())
        } else if idx < prog_count + proj_count + mile_count {
            app.milestones
                .get(idx - prog_count - proj_count)
                .map(|e| e.path.clone())
        } else {
            app.tasks
                .get(idx - prog_count - proj_count - mile_count)
                .map(|e| e.path.clone())
        }
    };

    let path_str = selected_path
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "No selection".to_string());

    let paragraph = Paragraph::new(path_str)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(paragraph, area);
}
