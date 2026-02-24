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
        ("Programs", ViewType::ProgramsList),
        ("Projects", ViewType::ProjectsList),
        ("Milestones", ViewType::MilestonesList),
        ("Tasks", ViewType::TasksList),
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
    match app.current_view {
        ViewType::ProgramsList => render_programs_list(f, app, area),
        ViewType::ProjectsList => render_projects_list(f, app, area),
        ViewType::MilestonesList => render_milestones_list(f, app, area),
        ViewType::TasksList => render_tasks_list(f, app, area),
        ViewType::Journal => render_journal_welcome(f, app, area),
        ViewType::JournalArchiveList => render_archive_list(f, app, area),
        ViewType::Backlog => render_placeholder(f, area, "Backlog", "No backlog items"),
        ViewType::SelectProgram => render_programs_list(f, app, area),
        ViewType::SelectProject => render_projects_list(f, app, area),
        ViewType::SelectMilestone => render_milestones_list(f, app, area),
        ViewType::InputProgram => render_input(f, app, area, "Enter program name:"),
        ViewType::InputProject => render_input(f, app, area, "Enter project name:"),
        ViewType::InputMilestone => render_input(f, app, area, "Enter milestone name:"),
        ViewType::InputTask => render_input(f, app, area, "Enter task name:"),
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

fn render_journal_welcome(f: &mut Frame, _app: &App, area: Rect) {
    let content = "Journal\n\n\
        Welcome to your journal!\n\n\
        Type /journal to access:\n\
          - Open Today's Journal\n\
          - Read Archived Journal Entries\n\n\
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

fn render_programs_list(f: &mut Frame, app: &App, area: Rect) {
    let title = match app.current_view {
        ViewType::SelectProgram => "Select Program (for new project)",
        _ => "Programs",
    };

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

fn render_projects_list(f: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(ref program) = app.current_program {
        format!("Projects - {}", program)
    } else {
        "Projects".to_string()
    };

    let title = match app.current_view {
        ViewType::SelectProject => "Select Project (for new milestone)".to_string(),
        _ => title,
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

fn render_milestones_list(f: &mut Frame, app: &App, area: Rect) {
    let title = if let (Some(ref program), Some(ref project)) =
        (&app.current_program, &app.current_project)
    {
        format!("Milestones - {}/{}", program, project)
    } else {
        "Milestones".to_string()
    };

    let title = match app.current_view {
        ViewType::SelectMilestone => "Select Milestone (for new task)".to_string(),
        _ => title,
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
        let paragraph = Paragraph::new("No archived journal entries found.")
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title("Archived Journals"),
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
                .title("Archived Journals (newest first)"),
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
        .enumerate()
        .map(|(idx, cmd)| {
            let style = if idx == app.command_selection_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
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
        .style(Style::default().fg(Color::White));

    f.render_widget(list, popup[1]);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let context = if let (Some(p), Some(pr), Some(m)) = (
        &app.current_program,
        &app.current_project,
        &app.current_milestone,
    ) {
        format!("{}/{}/{}", p, pr, m)
    } else if let (Some(p), Some(pr)) = (&app.current_program, &app.current_project) {
        format!("{}/{}", p, pr)
    } else if let Some(p) = &app.current_program {
        p.clone()
    } else {
        "No selection".to_string()
    };

    let status = format!(
        "Context: {} | Data: {} | Editor: {}",
        context,
        app.config.data_path.display(),
        app.config.editor
    );

    let paragraph = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(paragraph, area);
}
