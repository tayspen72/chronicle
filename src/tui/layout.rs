use crate::storage::WorkspaceStorage;
use crate::tui::{App, ViewType};
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
    let command_text = if app.command_palette_open {
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
    if app.command_palette_open {
        render_command_palette(f, app);
    }

    // Status bar
    render_status_bar(f, app, chunks[2]);
}

fn calculate_sidebar_width(app: &App) -> u16 {
    let depth = app.tree_state.path.len();
    let mut max_len = 0usize;

    // Add current level name
    let level_name = match depth {
        0 => "Programs",
        1 => "Projects",
        2 => "Milestones",
        3 => "Tasks",
        _ => "Items",
    };
    max_len = max_len.max(level_name.len());

    // Add all visible item names
    for prog in &app.programs {
        max_len = max_len.max(prog.name.len());
    }
    if depth >= 1 {
        for proj in &app.projects {
            max_len = max_len.max(proj.name.len() + 4); // +4 for tree indentation
        }
    }
    if depth >= 2 {
        for mile in &app.milestones {
            max_len = max_len.max(mile.name.len() + 8);
        }
    }
    if depth >= 3 {
        for task in &app.tasks {
            max_len = max_len.max(task.name.len() + 12);
        }
    }

    // Add padding for borders
    (max_len + 4).min(60).max(15) as u16
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let depth = app.tree_state.path.len();
    let idx = app.selected_entry_index;

    let prog_count = app.programs.len();
    let proj_count = app.projects.len();
    let mile_count = app.milestones.len();
    let task_count = app.tasks.len();

    let mut all_items: Vec<(usize, String, bool, bool)> = Vec::new();

    let add_tree_line =
        |all_items: &mut Vec<_>, indent: usize, name: &str, is_last: bool, is_selected: bool| {
            let prefix = if indent == 0 {
                name.to_string()
            } else if is_last {
                format!("└── {}", name)
            } else {
                format!("├── {}", name)
            };
            all_items.push((indent, prefix, is_selected, is_last));
        };

    if depth == 0 {
        for (i, entry) in app.programs.iter().enumerate() {
            let is_last = i == prog_count.saturating_sub(1);
            add_tree_line(&mut all_items, 0, &entry.name, is_last, i == idx);
        }
    } else if depth == 1 {
        let selected_program = &app.tree_state.path[0];
        for (i, entry) in app.programs.iter().enumerate() {
            let is_current = entry.name == *selected_program;
            let is_last_prog = i == prog_count.saturating_sub(1);
            add_tree_line(&mut all_items, 0, &entry.name, is_last_prog, false);
            if is_current {
                for (j, proj) in app.projects.iter().enumerate() {
                    let is_last = j == proj_count.saturating_sub(1);
                    let selected_proj_idx = idx.saturating_sub(prog_count);
                    add_tree_line(
                        &mut all_items,
                        1,
                        &proj.name,
                        is_last,
                        j == selected_proj_idx,
                    );
                }
            }
        }
    } else if depth == 2 {
        let selected_program = &app.tree_state.path[0];
        let selected_project = &app.tree_state.path[1];

        for (i, entry) in app.programs.iter().enumerate() {
            let is_current_prog = entry.name == *selected_program;
            let is_last_prog = i == prog_count.saturating_sub(1);
            add_tree_line(&mut all_items, 0, &entry.name, is_last_prog, false);

            if is_current_prog {
                for (j, proj) in app.projects.iter().enumerate() {
                    let is_current_proj = proj.name == *selected_project;
                    let is_last_proj = j == proj_count.saturating_sub(1);
                    add_tree_line(&mut all_items, 1, &proj.name, is_last_proj, false);

                    if is_current_proj {
                        for (k, mile) in app.milestones.iter().enumerate() {
                            let is_last = k == mile_count.saturating_sub(1);
                            let selected_mile_idx = idx.saturating_sub(prog_count + proj_count);
                            add_tree_line(
                                &mut all_items,
                                2,
                                &mile.name,
                                is_last,
                                k == selected_mile_idx,
                            );
                        }
                    }
                }
            }
        }
    } else if depth >= 3 {
        let selected_program = &app.tree_state.path[0];
        let selected_project = &app.tree_state.path[1];
        let selected_milestone = &app.tree_state.path[2];

        for (i, entry) in app.programs.iter().enumerate() {
            let is_current_prog = entry.name == *selected_program;
            let is_last_prog = i == prog_count.saturating_sub(1);
            add_tree_line(&mut all_items, 0, &entry.name, is_last_prog, i == idx);

            if is_current_prog {
                for (j, proj) in app.projects.iter().enumerate() {
                    let is_current_proj = proj.name == *selected_project;
                    let is_last_proj = j == proj_count.saturating_sub(1);
                    let proj_offset = prog_count;
                    add_tree_line(
                        &mut all_items,
                        1,
                        &proj.name,
                        is_last_proj,
                        j + proj_offset == idx,
                    );

                    if is_current_proj {
                        for (k, mile) in app.milestones.iter().enumerate() {
                            let is_current_mile = mile.name == *selected_milestone;
                            let is_last_mile = k == mile_count.saturating_sub(1);
                            let mile_offset = prog_count + proj_count;
                            add_tree_line(
                                &mut all_items,
                                2,
                                &mile.name,
                                is_last_mile,
                                k + mile_offset == idx,
                            );

                            if is_current_mile {
                                for (t, task) in app.tasks.iter().enumerate() {
                                    let is_last_task = t == task_count.saturating_sub(1);
                                    let task_offset = prog_count + proj_count + mile_count;
                                    add_tree_line(
                                        &mut all_items,
                                        3,
                                        &task.name,
                                        is_last_task,
                                        t + task_offset == idx,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let items: Vec<ListItem> = all_items
        .iter()
        .map(|(indent, prefix, is_selected, _is_last)| {
            let indent_str = "    ".repeat(*indent);
            let full_label = format!("{}{}", indent_str, prefix);
            let style = if *is_selected {
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

    let tier_name = match app.tree_state.path.len() {
        0 => "Programs",
        1 => "Projects",
        2 => "Milestones",
        3 => "Tasks",
        _ => "Items",
    };
    let title = tier_name.to_string();

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

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    match app.current_view {
        ViewType::TreeView => render_tree_view(f, app, area),
        ViewType::Journal => render_journal_welcome(f, app, area),
        ViewType::JournalArchiveList => render_archive_list(f, app, area),
        ViewType::Backlog => render_placeholder(f, area, "Backlog", "No backlog items"),
        ViewType::ViewingContent => render_content_viewer(f, app, area),
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

fn render_tree_view(f: &mut Frame, app: &App, area: Rect) {
    let depth = app.tree_state.path.len();
    let idx = app.selected_entry_index;

    let selected_entry: Option<&crate::storage::DirectoryEntry>;
    let content_to_show: String;

    if depth == 0 {
        selected_entry = app.programs.get(idx);
    } else if depth == 1 {
        let prog_count = app.programs.len();
        if idx < prog_count {
            selected_entry = app.programs.get(idx);
        } else {
            selected_entry = app.projects.get(idx - prog_count);
        }
    } else if depth == 2 {
        let prog_count = app.programs.len();
        let proj_count = app.projects.len();
        if idx < prog_count {
            selected_entry = app.programs.get(idx);
        } else if idx < prog_count + proj_count {
            selected_entry = app.projects.get(idx - prog_count);
        } else {
            selected_entry = app.milestones.get(idx - prog_count - proj_count);
        }
    } else {
        let prog_count = app.programs.len();
        let proj_count = app.projects.len();
        let mile_count = app.milestones.len();
        if idx < prog_count {
            selected_entry = app.programs.get(idx);
        } else if idx < prog_count + proj_count {
            selected_entry = app.projects.get(idx - prog_count);
        } else if idx < prog_count + proj_count + mile_count {
            selected_entry = app.milestones.get(idx - prog_count - proj_count);
        } else {
            selected_entry = app.tasks.get(idx - prog_count - proj_count - mile_count);
        }
    }

    if let Some(entry) = selected_entry {
        content_to_show = app
            .config
            .data_path
            .read_md_file(&entry.path)
            .unwrap_or_else(|_| format!("# {}\n\n(No content or file not found)", entry.name));
    } else {
        content_to_show = "No item selected".to_string();
    }

    let level_name = match depth {
        0 => "Programs",
        1 => "Projects",
        2 => "Milestones",
        3 => "Tasks",
        _ => "Items",
    };

    let title = if let Some(entry) = selected_entry {
        entry.name.clone()
    } else {
        "Empty".to_string()
    };

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
        .as_ref()
        .map(|t| t.clone())
        .unwrap_or_else(|| "No content".to_string());

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
