use super::views;
use crate::tui::{App, Mode, ViewType};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
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
        app.command_palette.display_text()
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
    let max_len = app
        .navigation_state
        .sidebar_items
        .iter()
        .map(|item| {
            let prefix_len = if item.is_header || item.indent == 0 {
                0
            } else {
                4
            };
            item.name.len() + (item.indent * 4) + prefix_len
        })
        .max()
        .unwrap_or(0)
        .max("Navigator".len());

    ((max_len + 6) as u16).clamp(15, 60)
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let idx = app.navigation_state.selected_entry_index;
    let in_selection_mode = app.mode == Mode::TaskSelection;

    let items = &app.navigation_state.sidebar_items;

    // Pre-compute which items need vertical continuation pipes at each indent level.
    // Pipe at level d means: "there are items at depth d after this item"
    // Note: Level 0 (root) is excluded - it's handled by the root's tree connector,
    // not by continuation pipes for its children.
    let max_indent = items.iter().map(|i| i.indent).max().unwrap_or(0);
    let continuation_levels: Vec<Vec<bool>> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            (1..=max_indent) // Start from level 1, exclude level 0
                .map(|d| {
                    if item.indent > d {
                        true
                    } else {
                        items[i + 1..].iter().any(|x| x.indent == d)
                    }
                })
                .collect()
        })
        .collect();

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == idx;

            // Determine checkbox prefix for TaskSelection mode
            let checkbox_prefix =
                (in_selection_mode && !item.is_header && item.indent >= 3).then(|| {
                    let is_selected = item.path.as_ref().is_some_and(|p| {
                        app.planning_session.is_path_selected(&p.to_string_lossy())
                    });
                    if is_selected { "[x] " } else { "[ ] " }
                });

            let prefix = if item.is_header || item.indent == 0 {
                item.name.clone()
            } else {
                // Build vertical pipes for ancestor levels.
                // Level 0 (root) has no pipe, so pipes start from level 1.
                // Indent 1 items have no pipes (direct children of root).
                // Indent 2 items have 1 pipe (to level 1).
                // Indent 3 items have 2 pipes (to levels 1 and 2).
                let pipe_start = 1;
                let pipes: String = (pipe_start..item.indent)
                    .map(|d| {
                        let has_pipe = continuation_levels
                            .get(i)
                            .and_then(|l| l.get(d))
                            .copied()
                            .unwrap_or(false);
                        if has_pipe { "│   " } else { "    " }
                    })
                    .collect();

                // Check if there are more siblings at the same indent
                let is_last = items[i + 1..].iter().all(|p| p.indent != item.indent);

                let tree_prefix = if is_last { "└── " } else { "├── " };
                format!("{}{}{}", pipes, tree_prefix, item.name)
            };

            let full_label = if let Some(cb) = checkbox_prefix {
                // Prefix includes tree chars; strip them and prepend checkbox column
                let tree_chars_len = if prefix.starts_with("│   ") { 12 } else { 4 };
                format!(
                    "{}{}{}",
                    "    ".repeat(item.indent),
                    cb,
                    &prefix[tree_chars_len.min(prefix.len())..]
                )
            } else {
                prefix.clone()
            };

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

    let list = List::new(list_items)
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
            if let Some(ref state) = app.wizard_state.template {
                let prompt = format!("Fill in fields for: {}", state.template_name);
                views::render_template_fields(f, app, area, &prompt);
            } else {
                views::render_input(f, app, area, "Enter value:");
            }
        }
        ViewType::InputPlanningSessionDates => {
            views::render_planning_dates_wizard(f, app, area);
        }
        ViewType::PlanningTaskPicker => {
            views::render_planning_task_picker(f, app, area);
        }
        ViewType::HierarchicalTaskPicker => {
            views::render_hierarchical_task_picker(f, app, area);
        }
        ViewType::PlanningPreview => {
            views::render_planning_preview(f, app, area);
        }
        ViewType::TaskDetailWizard | ViewType::InputTaskDetailField => {
            views::render_task_detail_wizard(f, app, area);
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
    let input = Paragraph::new(app.command_palette.display_text())
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
        .command_palette
        .matches
        .iter()
        .enumerate()
        .map(|(idx, cmd)| {
            let style = if idx == app.command_palette.selection_index {
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

    if let Some(program) = &app.navigation_state.current_program {
        breadcrumb_parts.push(program.clone());
    }
    if let Some(project) = &app.navigation_state.current_project {
        breadcrumb_parts.push(project.clone());
    }
    if let Some(milestone) = &app.navigation_state.current_milestone {
        breadcrumb_parts.push(milestone.clone());
    }
    if let Some(task) = &app.navigation_state.current_task {
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
        Mode::TaskSelection => ("SELECT", Color::Magenta),
        Mode::ReviewSession => ("REVIEW", Color::LightMagenta),
        Mode::HierarchicalSelection => ("ADD TASKS", Color::LightCyan),
        Mode::PlanningPreview => ("PREVIEW", Color::LightBlue),
        Mode::TaskDetailWizard => ("EDIT TASK", Color::LightYellow),
        Mode::InputTaskDetailField => ("INPUT", Color::Cyan),
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
