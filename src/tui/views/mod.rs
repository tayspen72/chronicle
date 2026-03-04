// Views module - content display handlers

use crate::storage::{JournalStorage, WorkspaceStorage};
use crate::tui::App;
use chrono::{Datelike, Utc};
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render_tree_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
                            app.config.workspace.open_or_create_today_journal()
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
                            .workspace
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
                content_to_show = app.config.workspace.read_md_file(path).unwrap_or_else(|_| {
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

pub fn render_journal_welcome(f: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
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

pub fn render_journal_today(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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

pub fn render_archive_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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

    let title = "Journal History";
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

/// Renders the Backlog view showing all tasks across all programs/projects/milestones.
pub fn render_backlog(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut all_tasks: Vec<(String, String)> = Vec::new(); // (task_name, context)

    // Collect all tasks from all programs/projects/milestones
    if let Ok(programs) = app.config.workspace.list_programs() {
        for program in &programs {
            if let Ok(projects) = app.config.workspace.list_projects(&program.name) {
                for project in &projects {
                    if let Ok(milestones) = app
                        .config
                        .workspace
                        .list_milestones(&program.name, &project.name)
                    {
                        for milestone in &milestones {
                            if let Ok(tasks) = app.config.workspace.list_tasks(
                                &program.name,
                                &project.name,
                                &milestone.name,
                            ) {
                                for task in &tasks {
                                    let context = format!(
                                        "{} > {} > {}",
                                        program.name, project.name, milestone.name
                                    );
                                    all_tasks.push((task.name.clone(), context));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if all_tasks.is_empty() {
        let content = "Backlog\n\nNo tasks found.\n\nCreate tasks in your programs/projects/milestones to see them here.";
        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title("Backlog"),
            );
        f.render_widget(paragraph, area);
        return;
    }

    // Sort tasks alphabetically by name
    all_tasks.sort_by(|a, b| a.0.cmp(&b.0));

    let items: Vec<ListItem> = all_tasks
        .iter()
        .map(|(task_name, context)| {
            let display = format!("{} - ({})", task_name, context);
            ListItem::new(display).style(Style::default().fg(Color::White))
        })
        .collect();

    let title = format!("Backlog ({} tasks)", all_tasks.len());
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

/// Renders the Weekly Planning view showing current week and task statistics.
pub fn render_weekly_planning(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Calculate current week boundaries (Monday to Sunday)
    let now = Utc::now();
    let weekday = now.weekday().num_days_from_monday();
    let monday = now.date_naive() - chrono::Duration::days(weekday as i64);
    let sunday = monday + chrono::Duration::days(6);

    let week_range = format!(
        "Week: {} to {}",
        monday.format("%Y-%m-%d"),
        sunday.format("%Y-%m-%d")
    );

    // Collect task statistics
    let mut total_tasks = 0;
    let mut tasks_by_status: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut tasks_due_this_week = 0;

    if let Ok(programs) = app.config.workspace.list_programs() {
        for program in &programs {
            if let Ok(projects) = app.config.workspace.list_projects(&program.name) {
                for project in &projects {
                    if let Ok(milestones) = app
                        .config
                        .workspace
                        .list_milestones(&program.name, &project.name)
                    {
                        for milestone in &milestones {
                            if let Ok(tasks) = app.config.workspace.list_tasks(
                                &program.name,
                                &project.name,
                                &milestone.name,
                            ) {
                                for task in &tasks {
                                    total_tasks += 1;

                                    // Try to read task file to parse status and due_date
                                    if let Ok(content) =
                                        app.config.workspace.read_md_file(&task.path)
                                    {
                                        // Parse status from frontmatter
                                        if let Some(status) =
                                            parse_frontmatter_field(&content, "status")
                                        {
                                            *tasks_by_status.entry(status).or_insert(0) += 1;
                                        }

                                        // Parse due_date and check if it's this week
                                        if let Some(due_date_str) =
                                            parse_frontmatter_field(&content, "due_date")
                                        {
                                            if let Ok(due_date) = chrono::NaiveDate::parse_from_str(
                                                &due_date_str,
                                                "%Y-%m-%d",
                                            ) {
                                                if due_date >= monday && due_date <= sunday {
                                                    tasks_due_this_week += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Build content
    let mut lines = vec![
        "Weekly Planning".to_string(),
        String::new(),
        week_range,
        String::new(),
        format!("Total Tasks: {}", total_tasks),
        String::new(),
    ];

    if !tasks_by_status.is_empty() {
        lines.push("Tasks by Status:".to_string());
        let mut statuses: Vec<_> = tasks_by_status.iter().collect();
        statuses.sort_by(|a, b| a.0.cmp(b.0));
        for (status, count) in statuses {
            lines.push(format!("  {}: {}", status, count));
        }
        lines.push(String::new());
    }

    if tasks_due_this_week > 0 {
        lines.push(format!("Tasks Due This Week: {}", tasks_due_this_week));
    }

    let content = lines.join("\n");
    let paragraph = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Weekly Planning"),
        );

    f.render_widget(paragraph, area);
}

/// Parses a field from YAML frontmatter in a markdown file.
fn parse_frontmatter_field(content: &str, field: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();

    // Find frontmatter boundaries
    if lines.is_empty() || lines[0] != "---" {
        return None;
    }

    let end_idx = lines.iter().skip(1).position(|line| *line == "---")?;

    // Search for the field in frontmatter
    for line in &lines[1..=end_idx] {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            if key == field {
                let value = value.trim().trim_matches('"').trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

pub fn render_content_viewer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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

    let inner_area = ratatui::layout::Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let paragraph = Paragraph::new(content.as_str()).style(Style::default().fg(Color::White));
    f.render_widget(paragraph, inner_area);
}

pub fn render_input(f: &mut Frame, app: &App, area: ratatui::layout::Rect, prompt: &str) {
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

/// Renders the template field wizard as a single-page form with CONFIRM/CANCEL buttons.
pub fn render_template_fields(f: &mut Frame, app: &App, area: ratatui::layout::Rect, prompt: &str) {
    use crate::tui::WizardFocus;

    let state = match app.template_field_state.as_ref() {
        Some(s) => s,
        None => {
            render_input(f, app, area, prompt);
            return;
        }
    };

    let fields = &state.fields;
    let field_count = fields.len();

    // Calculate available height for fields
    let header_height = 5u16;
    let instructions_height = 2u16;
    let input_height = 3u16;
    let button_height = 3u16;
    let available_height = area
        .height
        .saturating_sub(header_height + instructions_height + input_height + button_height);

    // Calculate how many fields can be visible
    let visible_fields = (available_height / 2) as usize;

    // Calculate scroll offset based on focused field
    let focused_field_idx = match state.focus {
        WizardFocus::Field(idx) => idx,
        WizardFocus::ConfirmButton | WizardFocus::CancelButton => field_count, // Scroll to show buttons
    };
    let scroll_offset = if focused_field_idx >= visible_fields {
        focused_field_idx - visible_fields + 1
    } else {
        0
    };

    // Build content
    let mut lines = vec![
        prompt.to_string(),
        String::new(),
        "↑/↓: Navigate | Enter: Next/Confirm | Esc: Cancel".to_string(),
        String::new(),
    ];

    // Scroll indicator if needed
    if field_count > visible_fields && scroll_offset > 0 {
        lines.push(format!("  ↑ {} more fields above...", scroll_offset));
    }

    // Render visible fields in display_order
    let mut sorted_fields: Vec<_> = fields.iter().enumerate().collect();
    sorted_fields.sort_by_key(|(_, f)| f.display_order);

    for (i, field) in sorted_fields.iter() {
        let idx = *i;
        if idx < scroll_offset || idx >= scroll_offset + visible_fields {
            continue;
        }

        let is_focused = matches!(state.focus, WizardFocus::Field(fi) if fi == idx);
        let prefix = if is_focused { "→ " } else { "  " };

        // Style differently for editable vs prepopulated
        let value_display = if field.value.is_empty() && field.is_editable {
            format!("<{}>", field.placeholder)
        } else if field.is_editable {
            field.value.clone()
        } else {
            // Prepopulated field - show value with indicator
            format!("{} (auto)", field.value)
        };

        let field_line = if field.is_editable {
            format!("{}{}: {}", prefix, field.label, value_display)
        } else {
            // Dim style for prepopulated - we'll add gray styling in display
            format!("{}{}: {} [readonly]", prefix, field.label, value_display)
        };

        lines.push(field_line);
    }

    // Scroll indicator if there are more fields below
    if field_count > visible_fields && scroll_offset + visible_fields < field_count {
        lines.push(format!(
            "  ↓ {} more fields below...",
            field_count - scroll_offset - visible_fields
        ));
    }

    // CONFIRM and CANCEL buttons
    lines.push(String::new());
    let confirm_prefix = if state.focus == WizardFocus::ConfirmButton {
        "→ "
    } else {
        "  "
    };
    let cancel_prefix = if state.focus == WizardFocus::CancelButton {
        "→ "
    } else {
        "  "
    };
    lines.push(format!(
        "{}[CONFIRM]     {}[CANCEL]",
        confirm_prefix, cancel_prefix
    ));
    lines.push(String::new());

    // Input area - only show when editing a field
    if let WizardFocus::Field(idx) = state.focus {
        if let Some(field) = fields.get(idx) {
            if field.is_editable {
                lines.push(format!("Edit {}: {}", field.label, app.input_buffer));
            } else {
                lines.push(format!("Viewing: {} (read-only)", field.label));
            }
        }
    } else {
        lines.push(String::new());
    }

    // Join all lines
    let content = lines.join("\n");

    // Render the widget
    let paragraph = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Template Fields"),
        );
    f.render_widget(paragraph, area);
}

pub fn render_placeholder(f: &mut Frame, area: ratatui::layout::Rect, title: &str, message: &str) {
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

// TODO: These render functions are extracted for future dedicated list views.
// Currently the tree view handles all navigation, but these will be useful
// when implementing separate list views for each tier.
#[allow(dead_code)]
pub fn render_programs_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
pub fn render_projects_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
pub fn render_milestones_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
pub fn render_tasks_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
