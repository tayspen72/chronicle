// Views module - content display handlers

use crate::storage::{JournalStorage, WorkspaceStorage};
use crate::tui::cache::build_journal_tree;
use crate::tui::{App, ElementReport, Mode, PlanningReportKind, SelectedElementView, navigation};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap},
};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;

pub fn render_element_report(
    f: &mut Frame,
    app: &App,
    report: &ElementReport,
    area: ratatui::layout::Rect,
) {
    if report.rows.is_empty() {
        let message = format!("No {} detected.", report.child_plural);
        let paragraph = Paragraph::new(message)
            .style(app.text_primary())
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(app.border_style())
                    .title(report.child_plural),
            );
        f.render_widget(paragraph, area);
        return;
    }

    let rows: Vec<Row> = report
        .rows
        .iter()
        .map(|row| {
            let grandchild_text = if row.grandchild_count == 0 {
                format!("0 {} (none detected)", report.grandchild_plural)
            } else if row.grandchild_count == 1 {
                format!("1 {}", report.grandchild_singular)
            } else {
                format!("{} {}", row.grandchild_count, report.grandchild_plural)
            };
            Row::new(vec![
                Cell::from(row.name.clone()),
                Cell::from(row.status.clone()),
                Cell::from(grandchild_text),
            ])
        })
        .collect();

    let column_widths = [
        Constraint::Percentage(50),
        Constraint::Length(14),
        Constraint::Length(26),
    ];
    let table = Table::new(rows, column_widths)
        .header(
            Row::new(vec![
                Cell::from("Name"),
                Cell::from("Status"),
                Cell::from(report.grandchild_plural),
            ])
            .style(app.content_table_header_style())
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.content_table_border_style())
                .title(report.child_plural),
        );

    f.render_widget(table, area);
}

fn render_selected_element_view(
    f: &mut Frame,
    app: &App,
    selected: &SelectedElementView,
    area: ratatui::layout::Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(app.border_style())
        .title(selected.title.clone());
    f.render_widget(block, area);

    let inner = ratatui::layout::Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let yaml_rows = parse_yaml_frontmatter_rows(&selected.content);
    let details_height = (yaml_rows.len() as u16 + 3).clamp(4, 10);
    let report_height = if selected.report.rows.is_empty() {
        4
    } else {
        (selected.report.rows.len() as u16 + 3).clamp(5, 12)
    };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(details_height),
            Constraint::Min(4),
            Constraint::Length(report_height),
        ])
        .split(inner);

    render_yaml_details(f, app, &yaml_rows, &selected.status, sections[0]);

    let markdown_body = strip_yaml_frontmatter(&selected.content);
    let markdown = Paragraph::new(markdown_to_text(markdown_body))
        .style(app.text_primary())
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style())
                .title("Content"),
        );
    f.render_widget(markdown, sections[1]);

    render_element_report(f, app, &selected.report, sections[2]);
}

fn render_yaml_details(
    f: &mut Frame,
    app: &App,
    rows: &[(String, String)],
    status: &str,
    area: ratatui::layout::Rect,
) {
    if rows.is_empty() {
        let message = format!("No YAML frontmatter detected.\nStatus: {}.", status);
        let paragraph = Paragraph::new(message)
            .style(app.text_primary())
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(app.border_style())
                    .title("Details"),
            );
        f.render_widget(paragraph, area);
        return;
    }

    let table_rows: Vec<Row> = rows
        .iter()
        .map(|(key, value)| Row::new(vec![Cell::from(key.clone()), Cell::from(value.clone())]))
        .collect();
    let widths = [Constraint::Length(24), Constraint::Min(20)];
    let table = Table::new(table_rows, widths)
        .header(
            Row::new(vec![Cell::from("Field"), Cell::from("Value")])
                .style(app.content_table_header_style())
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style())
                .title("Details"),
        );
    f.render_widget(table, area);
}

fn parse_yaml_frontmatter_rows(content: &str) -> Vec<(String, String)> {
    let (frontmatter, _) = split_yaml_frontmatter(content);
    let Some(frontmatter) = frontmatter else {
        return Vec::new();
    };

    let parsed: Value = match serde_yaml::from_str(frontmatter) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };

    match parsed {
        Value::Mapping(map) => mapping_to_rows(&map),
        other => vec![("value".to_string(), yaml_value_to_string(&other))],
    }
}

fn mapping_to_rows(map: &Mapping) -> Vec<(String, String)> {
    map.iter()
        .map(|(key, value)| {
            let key_text = match key {
                Value::String(text) => text.clone(),
                _ => yaml_value_to_string(key),
            };
            (key_text, yaml_value_to_string(value))
        })
        .collect()
}

fn yaml_value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => v.clone(),
        Value::Sequence(items) => items
            .iter()
            .map(yaml_value_to_string)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Mapping(_) => match serde_yaml::to_string(value) {
            Ok(serialized) => serialized.replace('\n', " ").trim().to_string(),
            Err(_) => String::new(),
        },
        Value::Tagged(tagged) => yaml_value_to_string(&tagged.value),
    }
}

pub fn render_tree_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    if let Some(selected) = app.selected_element_view() {
        render_selected_element_view(f, app, &selected, area);
        return;
    }
    let idx = app.navigation_state.selected_entry_index;
    let mut content_to_show = "No item selected".to_string();
    let mut title = "Empty".to_string();

    if idx < app.navigation_state.sidebar_items.len() {
        let item = &app.navigation_state.sidebar_items[idx];

        if !item.is_header && !item.name.is_empty() {
            // Handle journal tree navigation items (years, months, entries)
            if let Some(ref jpath) = item.journal_path {
                if item.is_journal_header {
                    // This is a journal header (year or month) - show contents at this level
                    if jpath.is_empty() {
                        // History level - show years list
                        title = "Journal History".to_string();
                        let years = app.journal_tree_state.years();
                        if years.is_empty() {
                            content_to_show = "No journal entries found.\n\nUse /journal to create today's entry.".to_string();
                        } else {
                            content_to_show = years.join("\n");
                        }
                    } else if jpath.len() == 1 {
                        // Year level - show months list
                        let year = &jpath[0];
                        title = format!("Journal - {}", year);
                        let months = app.journal_tree_state.months_for_year(year);
                        if months.is_empty() {
                            content_to_show = format!("No entries for {}", year);
                        } else {
                            content_to_show = months.join("\n");
                        }
                    } else if jpath.len() == 2 {
                        // Month level - show entries list
                        let year = &jpath[0];
                        let month = &jpath[1];
                        title = format!("Journal - {} {}", month, year);
                        let entries = app.journal_tree_state.entries_for_month(year, month);
                        if entries.is_empty() {
                            content_to_show = format!("No entries for {} {}", month, year);
                        } else {
                            content_to_show = entries
                                .iter()
                                .map(|e| e.filename.trim_end_matches(".md").to_string())
                                .collect::<Vec<_>>()
                                .join("\n");
                        }
                    }
                } else if let Some(entry) = app.journal_entries.iter().find(|e| {
                    let label = navigation::journal_entry_label(jpath).unwrap_or(&item.name);
                    *e.filename.trim_end_matches(".md") == *label
                }) {
                    // Entry level - show the entry content
                    title = entry.filename.trim_end_matches(".md").to_string();
                    content_to_show = app
                        .config
                        .workspace
                        .read_journal_entry(&entry.path)
                        .unwrap_or_else(|_| "Failed to load entry".to_string());
                }
            } else if let Some(journal_action) = &item.is_journal_item {
                // Handle journal action items (Today, History)
                match journal_action.as_str() {
                    "Today" => {
                        title = "Today".to_string();
                        let today_path = app.config.workspace.today_journal_path();
                        if today_path.exists() {
                            content_to_show = app
                                .config
                                .workspace
                                .read_md_file(&today_path)
                                .unwrap_or_else(|_| "Failed to load today's journal".to_string());
                        } else {
                            content_to_show =
                                "No journal entry found for today.\n\nPress Enter to create entry."
                                    .to_string();
                        }
                    }
                    "History" => {
                        title = "Journal History".to_string();
                        if app.journal_entries.is_empty() {
                            content_to_show = "No journal entries found.\n\nUse /journal to create today's entry.".to_string();
                        } else {
                            // Always show years list (tier is always visible)
                            let years = app.journal_tree_state.years();
                            content_to_show = years.join("\n");
                        }
                    }
                    _ => {}
                }
            } else if let Some(plan_type) = &item.is_planning_item {
                match plan_type.as_str() {
                    "WeeklyPlanning" => {
                        render_weekly_planning(f, app, area);
                        return;
                    }
                    "MyTasks" => {
                        render_my_tasks(f, app, area);
                        return;
                    }
                    "Backlog" => {
                        render_backlog(f, app, area);
                        return;
                    }
                    _ => {}
                }
            } else if item.section == crate::tui::navigation::SidebarSection::Notes {
                use crate::tui::cache::NoteNode;
                if let crate::tui::navigation::SidebarNodeData::Note(node) = &item.node_data {
                    match node {
                        NoteNode::Category { name } => {
                            title = name.clone();
                            let direct = app.notes_tree_state.entries_in_category(name);
                            let folders = app.notes_tree_state.folders_for_category(name);
                            let mut lines = Vec::new();
                            for f_name in &folders {
                                lines.push(format!("  {}/", f_name));
                                for entry in app.notes_tree_state.entries_in_folder(name, f_name) {
                                    lines.push(format!(
                                        "    {}",
                                        entry.filename.trim_end_matches(".md")
                                    ));
                                }
                            }
                            for entry in &direct {
                                lines.push(format!("  {}", entry.filename.trim_end_matches(".md")));
                            }
                            content_to_show = if lines.is_empty() {
                                format!(
                                    "No notes in {}.\n\nUse /New Note or /New Note Folder.",
                                    name
                                )
                            } else {
                                lines.join("\n")
                            };
                        }
                        NoteNode::Folder { category, name } => {
                            title = format!("{}/{}", category, name);
                            let entries = app.notes_tree_state.entries_in_folder(category, name);
                            let lines: Vec<String> = entries
                                .iter()
                                .map(|e| format!("  {}", e.filename.trim_end_matches(".md")))
                                .collect();
                            content_to_show = if lines.is_empty() {
                                format!(
                                    "No notes in {}/{}.\n\nUse /New Note or /New Note Folder.",
                                    category, name
                                )
                            } else {
                                lines.join("\n")
                            };
                        }
                        NoteNode::Entry { path, .. } => {
                            title = item.name.clone();
                            content_to_show = app
                                .config
                                .workspace
                                .read_md_file(path)
                                .unwrap_or_else(|_| "Failed to load note.".to_string());
                        }
                    }
                }
            } else if let Some(path) = &item.path {
                title = item.name.clone();
                content_to_show = app.config.workspace.read_md_file(path).unwrap_or_else(|_| {
                    format!("# {}\n\n(No content or file not found)", item.name)
                });
            }
        }
    }

    let paragraph = Paragraph::new(markdown_to_text(&content_to_show))
        .style(app.text_primary())
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style())
                .title(title),
        );

    f.render_widget(paragraph, area);
}

pub fn render_journal_welcome(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let content = "Journal\n\n\
        Welcome to your journal!\n\n\
        Type /journal to access:\n\
          - Open Today's Journal\n\
          - Journal History\n\n\
        Press / to open command palette";

    let paragraph = Paragraph::new(content).style(app.text_primary()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.border_style())
            .title("Journal"),
    );
    f.render_widget(paragraph, area);
}

pub fn render_journal_today(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let content = format!("Today's Journal\n\n{}", app.input_buffer);

    let paragraph = Paragraph::new(content).style(app.text_primary()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.border_style())
            .title(format!("Journal - {}", today)),
    );
    f.render_widget(paragraph, area);
}

pub fn render_archive_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    if app.journal_entries.is_empty() {
        let paragraph = Paragraph::new("No journal entries found.")
            .style(app.text_primary())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(app.border_style())
                    .title("Journal History"),
            );
        f.render_widget(paragraph, area);
        return;
    }

    let tree_items = build_journal_tree(&app.journal_entries);
    let selected_idx = app.navigation_state.selected_entry_index;

    let items: Vec<ListItem> = tree_items
        .iter()
        .enumerate()
        .map(|(i, (indent, label, is_header))| {
            let is_selected = i == selected_idx;
            let indent_str = "    ".repeat(*indent);

            let prefix = if *is_header {
                String::new()
            } else {
                let is_last = tree_items
                    .iter()
                    .skip(i + 1)
                    .take_while(|(nindent, _, _)| *nindent == *indent)
                    .next()
                    .is_none();
                if is_last { "└── " } else { "├── " }.to_string()
            };

            let full_label = format!("{}{}{}", indent_str, prefix, label);

            let style = if *is_header {
                if *indent == 0 {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
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
                .border_style(app.border_style())
                .title("Journal History"),
        )
        .style(app.text_primary());

    f.render_widget(list, area);
}

/// Renders the Backlog view showing all tasks across all programs/projects/milestones.
pub fn render_backlog(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let tasks = app.backlog_report_tasks();
    let is_interactive = app.mode == Mode::CurrentPlanNavigation
        && app.active_planning_report() == Some(PlanningReportKind::Backlog);
    let selected_idx = app.review_state.selection_index;

    render_plan_task_table(
        f,
        app,
        area,
        "Backlog",
        &tasks,
        is_interactive,
        selected_idx,
        "No unassigned tasks in current plan.",
    );

    let hints = if is_interactive {
        "j/k: move | a: assign to me | Esc/q: back"
    } else {
        "Enter: navigate backlog tasks"
    };
    render_hint_line(f, area, hints);
}

pub fn render_my_tasks(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let assigned_tasks = app.my_assigned_plan_tasks();
    let todos = app.today_todo_items();
    let is_interactive = app.mode == Mode::CurrentPlanNavigation
        && app.active_planning_report() == Some(PlanningReportKind::MyTasks);
    let selected_idx = app.review_state.selection_index;
    let task_count = assigned_tasks.len();

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((task_count as u16 + 4).clamp(6, 12)),
            Constraint::Min(6),
        ])
        .split(area);

    render_plan_task_table(
        f,
        app,
        sections[0],
        "My Tasks",
        &assigned_tasks,
        is_interactive,
        selected_idx,
        "No current-plan tasks assigned to you.",
    );

    if todos.is_empty() {
        let paragraph = Paragraph::new("No To Do items found in today's journal.")
            .style(app.text_primary())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(app.border_style())
                    .title("Today's To Do"),
            );
        f.render_widget(paragraph, sections[1]);
    } else {
        let rows = todos
            .iter()
            .enumerate()
            .map(|(idx, todo)| {
                let list_idx = task_count + idx;
                let is_selected = is_interactive && selected_idx == list_idx;
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::LightYellow)
                } else {
                    Style::default().fg(Color::White)
                };
                let checkbox = if todo.checked { "[x]" } else { "[ ]" };
                Row::new(vec![
                    Cell::from(checkbox).style(style),
                    Cell::from(todo.text.clone()).style(style),
                ])
            })
            .collect::<Vec<_>>();

        let table = Table::new(rows, [Constraint::Length(10), Constraint::Min(20)])
            .header(
                Row::new(vec![Cell::from("Done"), Cell::from("Task")])
                    .style(Style::default().fg(Color::LightBlue))
                    .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(app.border_style())
                    .title("Today's To Do"),
            );
        f.render_widget(table, sections[1]);
    }

    let hints = if is_interactive {
        "j/k: move | s: cycle task status | x: toggle To Do checkbox | Esc/q: back"
    } else {
        "Enter: navigate my tasks + today's To Do"
    };
    render_hint_line(f, area, hints);
}

fn render_plan_task_table(
    f: &mut Frame,
    app: &App,
    area: ratatui::layout::Rect,
    title: &str,
    tasks: &[crate::model::SelectedTask],
    is_interactive: bool,
    selected_idx: usize,
    empty_message: &str,
) {
    if tasks.is_empty() {
        let paragraph = Paragraph::new(empty_message)
            .style(app.text_primary())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(app.border_style())
                    .title(title),
            );
        f.render_widget(paragraph, area);
        return;
    }

    let rows = tasks
        .iter()
        .enumerate()
        .map(|(idx, task)| {
            let is_selected = is_interactive && idx == selected_idx;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::LightYellow)
            } else {
                Style::default().fg(Color::White)
            };
            Row::new(vec![
                Cell::from(task.task_name.clone()).style(style),
                Cell::from(task.due_date.clone().unwrap_or_else(|| "-".to_string())).style(style),
                Cell::from(task.assigned_to.clone().unwrap_or_else(|| "-".to_string()))
                    .style(style),
            ])
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(50),
            Constraint::Length(12),
            Constraint::Length(18),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from("Title"),
            Cell::from("Due Date"),
            Cell::from("Assigned To"),
        ])
        .style(Style::default().fg(Color::LightBlue))
        .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.border_style())
            .title(title),
    );
    f.render_widget(table, area);
}

fn render_hint_line(f: &mut Frame, area: ratatui::layout::Rect, text: &str) {
    let hints = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    let hint_area =
        ratatui::layout::Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
    f.render_widget(hints, hint_area);
}

/// Renders the Current Plan view showing tasks organized by hierarchy with workflow columns.
pub fn render_weekly_planning(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    if !app.planning_session.active {
        let empty_state = Paragraph::new(
            "No current planning session found.\n\nPress 'Enter' or '/Start Planning Session' to create a new plan.",
        )
        .style(app.text_primary())
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style())
                .title("Current Plan"),
        );
        f.render_widget(empty_state, area);
        return;
    }

    let mut workflow_columns = app.config.workflow.clone();
    if workflow_columns.is_empty() {
        workflow_columns.push("Status".to_string());
    }

    let matrix_rows = build_plan_matrix_rows(app);
    let total_count = app.planning_session.task_count();
    let selected_idx = app.review_state.selection_index;
    let is_interactive = app.mode == Mode::CurrentPlanNavigation;

    let mut status_counts: BTreeMap<String, usize> = BTreeMap::new();
    for task in &app.planning_session.tasks {
        *status_counts
            .entry(task.status.to_lowercase())
            .or_insert(0usize) += 1;
    }

    let mut header_cells = vec![Cell::from("Hierarchy")];
    header_cells.extend(workflow_columns.iter().map(|status| {
        let count = status_counts
            .get(&status.to_lowercase())
            .copied()
            .unwrap_or_default();
        Cell::from(format!("{} ({})", status, count))
    }));

    let header = Row::new(header_cells)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray))
        .height(1);

    let mut rows: Vec<Row<'static>> = Vec::new();
    if matrix_rows.is_empty() {
        let mut cells = vec![Cell::from("No tasks in current plan")];
        cells.extend(std::iter::repeat_n(Cell::from("-"), workflow_columns.len()));
        rows.push(Row::new(cells));
    } else {
        for row_data in matrix_rows {
            let is_selected = is_interactive
                && row_data
                    .task_index
                    .is_some_and(|task_idx| task_idx == selected_idx);
            let base_style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::LightYellow)
            } else {
                Style::default().fg(Color::White)
            };

            let hierarchy_style = match row_data.kind {
                MatrixRowKind::Program => base_style.add_modifier(ratatui::style::Modifier::BOLD),
                MatrixRowKind::Project => base_style.add_modifier(ratatui::style::Modifier::ITALIC),
                MatrixRowKind::Milestone => base_style.fg(Color::LightCyan),
                MatrixRowKind::Task => base_style,
            };

            let mut cells = vec![Cell::from(row_data.hierarchy).style(hierarchy_style)];
            match row_data.kind {
                MatrixRowKind::Task => {
                    let status = row_data.status.unwrap_or_default();
                    let task_name = row_data.task_name.unwrap_or_else(|| "-".to_string());
                    for workflow_status in &workflow_columns {
                        if workflow_status.eq_ignore_ascii_case(&status) {
                            cells.push(Cell::from(task_name.clone()).style(base_style));
                        } else {
                            cells.push(Cell::from("-").style(Style::default().fg(Color::DarkGray)));
                        }
                    }
                }
                MatrixRowKind::Program | MatrixRowKind::Project | MatrixRowKind::Milestone => {
                    cells.extend(std::iter::repeat_n(
                        Cell::from("-").style(Style::default().fg(Color::DarkGray)),
                        workflow_columns.len(),
                    ));
                }
            }
            rows.push(Row::new(cells).height(1));
        }
    }

    let hierarchy_width = if workflow_columns.len() > 5 { 30 } else { 36 };
    let status_width =
        ((100 - hierarchy_width as u16) / workflow_columns.len().max(1) as u16).max(8);
    let mut constraints = vec![Constraint::Percentage(hierarchy_width as u16)];
    constraints.extend(std::iter::repeat_n(
        Constraint::Percentage(status_width),
        workflow_columns.len(),
    ));

    let title = if app.planning_session.active {
        let (start, end) = app.planning_session.date_range();
        format!(
            "Current Plan [{} tasks | {} -> {}]",
            total_count, start, end
        )
    } else {
        "Current Plan".to_string()
    };

    let table = Table::new(rows, &constraints)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style())
                .title(title),
        )
        .column_spacing(1);

    f.render_widget(table, area);

    if app.mode == Mode::CurrentPlanNavigation {
        let hints = Paragraph::new(
            "j/k: move tasks | Enter: open task | s: toggle status | x: remove from plan | Esc/q: back",
        )
        .style(Style::default().fg(Color::DarkGray));
        let hint_area =
            ratatui::layout::Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
        f.render_widget(hints, hint_area);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatrixRowKind {
    Program,
    Project,
    Milestone,
    Task,
}

#[derive(Debug, Clone)]
struct MatrixRowData {
    kind: MatrixRowKind,
    hierarchy: String,
    task_index: Option<usize>,
    task_name: Option<String>,
    status: Option<String>,
}

fn build_plan_matrix_rows(app: &App) -> Vec<MatrixRowData> {
    let mut indexed_tasks: Vec<(usize, &crate::model::SelectedTask)> =
        app.planning_session.tasks.iter().enumerate().collect();
    indexed_tasks.sort_by(|(_, a), (_, b)| {
        (
            a.program.to_lowercase(),
            a.project.to_lowercase(),
            a.milestone.to_lowercase(),
            a.task_name.to_lowercase(),
        )
            .cmp(&(
                b.program.to_lowercase(),
                b.project.to_lowercase(),
                b.milestone.to_lowercase(),
                b.task_name.to_lowercase(),
            ))
    });

    let mut rows = Vec::new();
    let mut last_program = String::new();
    let mut last_project = String::new();
    let mut last_milestone = String::new();

    for (task_index, task) in indexed_tasks {
        let program = if task.program.trim().is_empty() {
            "Unscoped Program".to_string()
        } else {
            task.program.clone()
        };
        let project = if task.project.trim().is_empty() {
            "Unscoped Project".to_string()
        } else {
            task.project.clone()
        };
        let milestone = if task.milestone.trim().is_empty() {
            "Unscoped Milestone".to_string()
        } else {
            task.milestone.clone()
        };

        if program != last_program {
            rows.push(MatrixRowData {
                kind: MatrixRowKind::Program,
                hierarchy: program.clone(),
                task_index: None,
                task_name: None,
                status: None,
            });
            last_program = program.clone();
            last_project.clear();
            last_milestone.clear();
        }
        if project != last_project {
            rows.push(MatrixRowData {
                kind: MatrixRowKind::Project,
                hierarchy: project.clone(),
                task_index: None,
                task_name: None,
                status: None,
            });
            last_project = project.clone();
            last_milestone.clear();
        }
        if milestone != last_milestone {
            rows.push(MatrixRowData {
                kind: MatrixRowKind::Milestone,
                hierarchy: milestone.clone(),
                task_index: None,
                task_name: None,
                status: None,
            });
            last_milestone = milestone;
        }

        rows.push(MatrixRowData {
            kind: MatrixRowKind::Task,
            hierarchy: "-".to_string(),
            task_index: Some(task_index),
            task_name: Some(task.task_name.clone()),
            status: Some(task.status.clone()),
        });
    }
    rows
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
        .border_style(app.border_style())
        .title(title);

    f.render_widget(block, area);

    let inner_area = ratatui::layout::Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let paragraph = Paragraph::new(markdown_to_text(&content))
        .style(app.text_primary())
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner_area);
}

fn markdown_to_text(content: &str) -> Text<'static> {
    let body = strip_yaml_frontmatter(content);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;

    for raw_line in body.lines() {
        let trimmed = raw_line.trim_end();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                trimmed.to_string(),
                Style::default().fg(Color::Yellow),
            )));
            continue;
        }

        if trimmed.is_empty() {
            lines.push(Line::from(""));
            continue;
        }

        if let Some((level, text)) = parse_heading(trimmed) {
            let heading_color = match level {
                1 => Color::Cyan,
                2 => Color::LightBlue,
                _ => Color::White,
            };
            lines.push(Line::from(Span::styled(
                text.to_string(),
                Style::default()
                    .fg(heading_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )));
            continue;
        }

        if let Some(item) = parse_list_item(trimmed) {
            let mut spans = vec![Span::styled(
                "• ".to_string(),
                Style::default().fg(Color::LightBlue),
            )];
            spans.extend(render_inline_markdown(
                item,
                Style::default().fg(Color::White),
            ));
            lines.push(Line::from(spans));
            continue;
        }

        if let Some(quote) = trimmed.strip_prefix("> ") {
            lines.push(Line::from(Span::styled(
                quote.to_string(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(ratatui::style::Modifier::ITALIC),
            )));
            continue;
        }

        lines.push(Line::from(render_inline_markdown(
            trimmed,
            Style::default().fg(Color::White),
        )));
    }

    Text::from(lines)
}

fn strip_yaml_frontmatter(content: &str) -> &str {
    let (_, body) = split_yaml_frontmatter(content);
    body
}

fn split_yaml_frontmatter(content: &str) -> (Option<&str>, &str) {
    let Some(first_line) = content.lines().next() else {
        return (None, content);
    };
    if first_line.trim_end() != "---" {
        return (None, content);
    }

    let mut cursor = first_line.len();
    if content.as_bytes().get(cursor) == Some(&b'\n') {
        cursor += 1;
    }
    let frontmatter_start = cursor;

    for line in content[cursor..].lines() {
        let line_start = cursor;
        cursor += line.len();
        if content.as_bytes().get(cursor) == Some(&b'\n') {
            cursor += 1;
        }

        if line.trim_end() == "---" {
            let frontmatter = &content[frontmatter_start..line_start];
            let body = &content[cursor..];
            return (Some(frontmatter), body);
        }
    }

    (None, content)
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let text = line[hashes..].trim_start();
    if text.is_empty() {
        return None;
    }
    Some((hashes, text))
}

fn parse_list_item(line: &str) -> Option<&str> {
    for prefix in ["- ", "* ", "+ "] {
        if let Some(item) = line.strip_prefix(prefix) {
            return Some(item);
        }
    }

    let bytes = line.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx > 0 && idx + 1 < bytes.len() && bytes[idx] == b'.' && bytes[idx + 1] == b' ' {
        return Some(&line[idx + 2..]);
    }
    None
}

fn render_inline_markdown(line: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut bold = false;
    let mut italic = false;
    let mut code = false;
    let bytes = line.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            flush_span(
                &mut spans,
                &mut buffer,
                style_from_flags(base_style, bold, italic, code),
            );
            bold = !bold;
            i += 2;
            continue;
        }

        if bytes[i] == b'*' {
            flush_span(
                &mut spans,
                &mut buffer,
                style_from_flags(base_style, bold, italic, code),
            );
            italic = !italic;
            i += 1;
            continue;
        }

        if bytes[i] == b'`' {
            flush_span(
                &mut spans,
                &mut buffer,
                style_from_flags(base_style, bold, italic, code),
            );
            code = !code;
            i += 1;
            continue;
        }

        buffer.push(bytes[i] as char);
        i += 1;
    }

    flush_span(
        &mut spans,
        &mut buffer,
        style_from_flags(base_style, bold, italic, code),
    );
    spans
}

fn style_from_flags(base: Style, bold: bool, italic: bool, code: bool) -> Style {
    let mut style = base;
    if bold {
        style = style.add_modifier(ratatui::style::Modifier::BOLD);
    }
    if italic {
        style = style.add_modifier(ratatui::style::Modifier::ITALIC);
    }
    if code {
        style = style.fg(Color::Yellow).bg(Color::Rgb(40, 40, 40));
    }
    style
}

fn flush_span(spans: &mut Vec<Span<'static>>, buffer: &mut String, style: Style) {
    if !buffer.is_empty() {
        spans.push(Span::styled(buffer.clone(), style));
        buffer.clear();
    }
}

pub fn render_input(f: &mut Frame, app: &App, area: ratatui::layout::Rect, prompt: &str) {
    let content = format!(
        "{}\n\n> {}\n\nPress Enter to confirm, Esc to cancel",
        prompt, app.input_buffer
    );
    let paragraph = Paragraph::new(content).style(app.text_primary()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.border_style())
            .title("Input"),
    );
    f.render_widget(paragraph, area);
}

/// Renders the template field wizard with inline editing - field names and values on same line.
pub fn render_template_fields(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use crate::tui::{WizardFocus, wizard::FieldKind};
    use ratatui::layout::{Constraint, Layout};
    use ratatui::text::{Line, Span};

    let Some(state) = app.wizard_state.template.as_ref() else {
        render_input(f, app, area, "No template wizard state");
        return;
    };

    let chunks = Layout::default()
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            state.path_hint.as_str(),
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )]),
        Line::from(Span::styled(
            "↑/↓: Navigate | ←/→: Options | Enter: Confirm | Esc: Cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ]);
    f.render_widget(header, chunks[0]);

    let body_chunk = Layout::default()
        .constraints([Constraint::Min(1)])
        .split(chunks[1])[0];

    let mut lines_vec: Vec<Line> = Vec::new();
    for (idx, field) in state.fields.iter().enumerate() {
        let is_focused = matches!(state.focus, WizardFocus::Field(fi) if fi == idx);
        let display_value = if field.value.is_empty() && field.is_editable {
            "empty".to_string()
        } else {
            field.value.clone()
        };

        let value_color = match field.kind {
            FieldKind::Fixed => Color::White,
            FieldKind::AutoFilled => Color::DarkGray,
            _ => {
                if field.was_edited {
                    Color::White
                } else {
                    Color::DarkGray
                }
            }
        };

        let mut line_spans = Vec::new();
        line_spans.push(Span::styled("  ", Style::default().fg(Color::White)));
        let label_style = if is_focused && field.is_editable {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD)
        };
        line_spans.push(Span::styled(format!("{}: ", field.label), label_style));
        let value_style = if is_focused && field.is_editable {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(value_color)
        };
        line_spans.push(Span::styled(display_value, value_style));

        if is_focused && !field.choices.is_empty() {
            line_spans.push(Span::raw("    "));
            line_spans.push(Span::styled("[ ", Style::default().fg(Color::DarkGray)));
            for option in &field.choices {
                let is_selected = option.eq_ignore_ascii_case(&field.value);
                let option_style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightBlue)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                line_spans.push(Span::styled(option.as_str(), option_style));
                line_spans.push(Span::styled(" ", Style::default().fg(Color::DarkGray)));
            }
            line_spans.push(Span::styled("]", Style::default().fg(Color::DarkGray)));
        }

        lines_vec.push(Line::from(line_spans));
    }

    f.render_widget(Paragraph::new(lines_vec), body_chunk);

    let confirm_style = if state.focus == WizardFocus::ConfirmButton {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let cancel_style = if state.focus == WizardFocus::CancelButton {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("CONFIRM", confirm_style),
            Span::styled("     ", Style::default().fg(Color::DarkGray)),
            Span::styled("CANCEL", cancel_style),
        ])),
        chunks[2],
    );
}

pub fn render_placeholder(
    f: &mut Frame,
    app: &App,
    area: ratatui::layout::Rect,
    title: &str,
    message: &str,
) {
    let content = format!("{}\n\n({})", title, message);
    let paragraph = Paragraph::new(content).style(app.text_primary()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.border_style())
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

    if app.tree_data.programs.is_empty() {
        render_input(f, app, area, "No programs yet. Type to create one:");
        return;
    }

    let items: Vec<ListItem> = app
        .tree_data
        .programs
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == app.navigation_state.selected_entry_index {
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
                .border_style(app.border_style())
                .title(title),
        )
        .style(app.text_primary());

    f.render_widget(list, area);
}

#[allow(dead_code)]
pub fn render_projects_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = if let Some(ref program) = app.navigation_state.current_program {
        format!("Projects - {}", program)
    } else {
        "Projects".to_string()
    };

    if app.tree_data.projects.is_empty() {
        let content = format!(
            "{}\n\n(No projects yet. Use /new project to create one.)",
            title
        );
        let paragraph = Paragraph::new(content).style(app.text_primary()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style())
                .title(title),
        );
        f.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tree_data
        .projects
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == app.navigation_state.selected_entry_index {
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
                .border_style(app.border_style())
                .title(title),
        )
        .style(app.text_primary());

    f.render_widget(list, area);
}

#[allow(dead_code)]
pub fn render_milestones_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = if let (Some(program), Some(project)) = (
        &app.navigation_state.current_program,
        &app.navigation_state.current_project,
    ) {
        format!("Milestones - {}/{}", program, project)
    } else {
        "Milestones".to_string()
    };

    if app.tree_data.milestones.is_empty() {
        let content = format!(
            "{}\n\n(No milestones yet. Use /new milestone to create one.)",
            title
        );
        let paragraph = Paragraph::new(content).style(app.text_primary()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style())
                .title(title),
        );
        f.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tree_data
        .milestones
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == app.navigation_state.selected_entry_index {
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
                .border_style(app.border_style())
                .title(title),
        )
        .style(app.text_primary());

    f.render_widget(list, area);
}

#[allow(dead_code)]
pub fn render_tasks_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = if let (Some(program), Some(project), Some(milestone)) = (
        &app.navigation_state.current_program,
        &app.navigation_state.current_project,
        &app.navigation_state.current_milestone,
    ) {
        format!("Tasks - {}/{}/{}", program, project, milestone)
    } else {
        "Tasks".to_string()
    };

    if app.tree_data.tasks.is_empty() {
        let content = format!("{}\n\n(No tasks yet. Use /new task to create one.)", title);
        let paragraph = Paragraph::new(content).style(app.text_primary()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style())
                .title(title),
        );
        f.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tree_data
        .tasks
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == app.navigation_state.selected_entry_index {
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
                .border_style(app.border_style())
                .title(title),
        )
        .style(app.text_primary());

    f.render_widget(list, area);
}

pub fn render_planning_dates_wizard(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    // Get wizard state or return early if not present
    let Some(ref wizard) = app.planning_wizard else {
        let error =
            Paragraph::new("No planning wizard state").style(Style::default().fg(Color::Red));
        f.render_widget(error, area);
        return;
    };

    let start_date_input = &wizard.start_date;
    let duration = &wizard.duration;
    let focus = wizard.focus.index();

    // When editing (focused), show input_buffer; otherwise show stored value
    let start_date_display = if focus == 0 {
        wizard.input_buffer.clone()
    } else if start_date_input.is_empty() {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    } else {
        start_date_input.clone()
    };

    let days = match duration.as_str() {
        "biweekly" => 14,
        "6weekly" => 42,
        _ => 7,
    };

    let end_date = chrono::NaiveDate::parse_from_str(&start_date_display, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.checked_add_days(chrono::Days::new(days)))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| start_date_display.clone());

    let chunks = Layout::default()
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "Start Planning Session",
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )]),
        Line::from(Span::styled(
            "↑/↓: Navigate | ←/→: Duration | Enter: Confirm | Esc: Cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ]);
    f.render_widget(header, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();

    let start_label_style = if focus == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD)
    };
    let start_value_style = if focus == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else if wizard.start_date_edited {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::styled("  Start date: ", start_label_style),
        Span::styled(&start_date_display, start_value_style),
    ]));

    let duration_label_style = if focus == 1 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD)
    };
    let duration_value_style = if focus == 1 || wizard.duration_edited {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let mut duration_spans = vec![
        Span::styled("  Duration: ", duration_label_style),
        Span::styled(duration, duration_value_style),
    ];
    if focus == 1 {
        let duration_choices = ["weekly", "biweekly", "6weekly"];
        duration_spans.push(Span::raw("    "));
        duration_spans.push(Span::styled("[ ", Style::default().fg(Color::DarkGray)));
        for option in duration_choices {
            let is_selected = option.eq_ignore_ascii_case(duration);
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            duration_spans.push(Span::styled(option, style));
            duration_spans.push(Span::styled(" ", Style::default().fg(Color::DarkGray)));
        }
        duration_spans.push(Span::styled("]", Style::default().fg(Color::DarkGray)));
    }
    lines.push(Line::from(duration_spans));

    // End date field - editable, allows overriding auto-calculated date
    // When focused (editing), show input_buffer; otherwise show stored or auto-calculated value
    let end_date_display = if focus == 2 {
        wizard.input_buffer.clone()
    } else if wizard.end_date.is_empty() {
        end_date.clone()
    } else {
        wizard.end_date.clone()
    };

    let end_label_style = if focus == 2 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD)
    };
    let end_value_style = if focus == 2 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else if wizard.end_date_edited {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::styled("  End date: ", end_label_style),
        Span::styled(&end_date_display, end_value_style),
    ]));

    if let Some(ref error) = wizard.date_error {
        lines.push(Line::from(Span::styled(
            format!("  ⚠ {}", error),
            Style::default().fg(Color::Red),
        )));
    }

    let content = Paragraph::new(lines);
    f.render_widget(content, chunks[1]);

    // Buttons - match template wizard style (no brackets)
    let confirm_style = if focus == 3 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let cancel_style = if focus == 4 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let buttons = Paragraph::new(Line::from(vec![
        Span::styled("CONFIRM", confirm_style),
        Span::styled("     ", Style::default().fg(Color::DarkGray)),
        Span::styled("CANCEL", cancel_style),
    ]));
    f.render_widget(buttons, chunks[2]);
}

pub fn render_planning_task_picker(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{List, ListItem, Paragraph};

    let chunks = Layout::default()
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

    let prompt_line = Line::from(vec![Span::styled(
        "Select Tasks for Planning Session",
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )]);
    let date_info = Line::from(Span::styled(
        app.planning_session.format_date_range(),
        Style::default().fg(Color::DarkGray),
    ));
    let header = Paragraph::new(vec![prompt_line, date_info]);
    f.render_widget(header, chunks[0]);

    // Filter input with cursor - only show if planning wizard is active
    if let Some(ref wizard) = app.planning_wizard {
        let filter_prompt = Line::from(vec![
            Span::styled("Filter: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                wizard.task_filter.as_str(),
                Style::default().fg(Color::White),
            ),
            Span::styled("_", Style::default().fg(Color::LightBlue)),
        ]);
        let filter_para = Paragraph::new(filter_prompt)
            .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::BOTTOM));
        f.render_widget(filter_para, chunks[1]);

        let selected_set: std::collections::HashSet<_> =
            wizard.selected_tasks.iter().cloned().collect();

        let filtered_tasks = app.get_filtered_tasks();
        let total_count = wizard.tasks.len();
        let selected_count = wizard.selected_tasks.len();

        let items: Vec<ListItem> = filtered_tasks
            .iter()
            .enumerate()
            .map(|(idx, task)| {
                let is_selected = selected_set.contains(&task.uuid);
                let is_focused = idx == wizard.task_index;
                let check = if is_selected { "[x]" } else { "[ ]" };
                let name_style = if is_focused {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightBlue)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                // Show full hierarchy path for context
                let hierarchy = format!("{} > {} > {}", task.program, task.project, task.milestone);
                ListItem::new(Line::from(vec![
                    Span::styled(check, Style::default().fg(Color::Cyan)),
                    Span::raw(" "),
                    Span::styled(&task.task_name, name_style),
                    Span::styled(
                        format!(" ({})", hierarchy),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::NONE)
                .title(format!(
                    "Tasks ({}/{} selected)",
                    selected_count, total_count
                )),
        );
        f.render_widget(list, chunks[2]);

        // Buttons - match template wizard style
        let confirm_style = if !wizard.selected_tasks.is_empty() {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let buttons = Paragraph::new(Line::from(vec![
            Span::styled("CONFIRM", confirm_style),
            Span::styled("     ", Style::default().fg(Color::DarkGray)),
            Span::styled("CANCEL", Style::default().fg(Color::White)),
        ]));
        f.render_widget(buttons, chunks[3]);
    } else {
        // No wizard active - show placeholder
        let placeholder = Paragraph::new("No planning wizard active");
        f.render_widget(placeholder, chunks[1]);
    }
}

pub fn render_hierarchical_task_picker(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use crate::tui::hierarchical_picker::PickerLevel;
    use ratatui::layout::{Constraint, Layout};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{List, ListItem, Paragraph};

    let picker = &app.hierarchical_picker;
    let chunks = Layout::default()
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

    let title_line = Line::from(vec![Span::styled(
        "Add Tasks To Plan",
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )]);
    let header = Paragraph::new(vec![title_line]);
    f.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = picker
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_cursor = idx == picker.cursor_index;
            let is_task_level = picker.level == PickerLevel::Tasks;

            // For tasks level, show checkbox if selected
            let check_prefix = if is_task_level {
                let path_str = item.path.to_string_lossy().to_string();
                let is_selected = picker.selected_tasks.contains(&path_str);
                if is_selected { "[x] " } else { "[ ] " }
            } else {
                ""
            };

            let display_text = format!("{}{}", check_prefix, item.name);

            let style = if is_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(display_text).style(style)
        })
        .collect();

    let level_title = format!("{} ({})", picker.level_title(), picker.items.len());
    let list = List::new(items).block(
        ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::NONE)
            .title(level_title),
    );
    f.render_widget(list, chunks[1]);

    let hint_text = match picker.level {
        PickerLevel::Programs => "↑/↓: Navigate | Enter: Select | ←/→: Back/Forward | Esc: Cancel",
        PickerLevel::Projects => "↑/↓: Navigate | Enter: Select | ←/→: Back/Forward | Esc: Back",
        PickerLevel::Milestones => "↑/↓: Navigate | Enter: Select | ←/→: Back/Forward | Esc: Back",
        PickerLevel::Tasks => "↑/↓: Navigate | Enter: Add Task | Esc: Back",
    };
    let session_task_count = app.planning_session.task_count();
    let count_text = if session_task_count > 0 && picker.is_wizard_mode {
        format!("{} | f: Finish | Tasks: {}", hint_text, session_task_count)
    } else {
        hint_text.to_string()
    };
    let buttons = Paragraph::new(Line::from(Span::styled(
        count_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(buttons, chunks[2]);
}

pub fn render_planning_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph};

    let chunks = Layout::default()
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Line::from(vec![Span::styled(
        "Preview Plan",
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )]);
    let date_range = if let (Some(start), Some(end)) = (
        &app.planning_session.start_date,
        &app.planning_session.due_date,
    ) {
        format!("{} → {}", start, end)
    } else {
        "No dates set".to_string()
    };
    let subtitle = Line::from(Span::styled(
        date_range,
        Style::default().fg(Color::DarkGray),
    ));
    let instructions = Line::from(Span::styled(
        "↑/↓: Navigate | ←/→: Duration | Enter: Confirm | Esc: Cancel",
        Style::default().fg(Color::DarkGray),
    ));
    let header = Paragraph::new(vec![title, subtitle, instructions]);
    f.render_widget(header, chunks[0]);

    let tasks = &app.planning_session.tasks;
    let task_count = tasks.len();
    let task_lines: Vec<Line> = if task_count == 0 {
        vec![Line::from(Span::styled(
            "No tasks selected",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        let mut sorted_tasks: Vec<_> = tasks.iter().collect();
        sorted_tasks.sort_by(|a, b| {
            a.program
                .cmp(&b.program)
                .then_with(|| a.project.cmp(&b.project))
                .then_with(|| a.milestone.cmp(&b.milestone))
                .then_with(|| a.status.cmp(&b.status))
                .then_with(|| a.task_name.cmp(&b.task_name))
        });
        sorted_tasks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                Line::from(vec![Span::styled(
                    format!(
                        "{}. {} ({}) - {}/{}/{}",
                        i + 1,
                        t.task_name,
                        t.status,
                        t.program,
                        t.project,
                        t.milestone
                    ),
                    Style::default().fg(Color::White),
                )])
            })
            .collect()
    };
    let tasks_para = Paragraph::new(task_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.border_style())
            .title(format!("Tasks ({})", task_count)),
    );
    f.render_widget(tasks_para, chunks[1]);

    let buttons = ["ADD TASKS TO PLAN", "CONFIRM", "CANCEL"];
    let mut spans = Vec::new();
    for (i, label) in buttons.iter().enumerate() {
        let style = if i == app.review_state.preview_focus {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        spans.push(Span::styled(format!(" {} ", label), style));
        if i < buttons.len() - 1 {
            spans.push(Span::raw("  "));
        }
    }
    let buttons_para = Paragraph::new(Line::from(spans));
    f.render_widget(buttons_para, chunks[2]);

    // Show confirmation message if session was just confirmed
}

pub fn render_task_detail_wizard(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph};

    let Some(ref task) = app.task_wizard.as_ref().unwrap().task else {
        let error = Paragraph::new("No task selected").style(Style::default().fg(Color::Red));
        f.render_widget(error, area);
        return;
    };

    // Inline editing mode - cursor shows on focused field
    let is_editing = app.mode == Mode::TaskDetailWizard;
    let focused_field = app.task_wizard.as_ref().unwrap().field_index;

    let fields: Vec<(&str, String)> = vec![
        ("Task", task.task_name.clone()),
        ("Status", task.status.clone()),
        ("Assigned to", task.assigned_to.clone().unwrap_or_default()),
        ("Start date", task.start_date.clone().unwrap_or_default()),
        ("Due date", task.due_date.clone().unwrap_or_default()),
        ("Importance", task.importance.clone().unwrap_or_default()),
        ("Description", task.description.clone().unwrap_or_default()),
    ];

    let chunks = Layout::default()
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let title_text = if app.task_wizard_tree_edit_mode {
        "Edit Subtask Details"
    } else {
        "Edit Task Details"
    };
    let hint_text = if app.task_wizard_tree_edit_mode {
        "↑/↓: Navigate | ←/→: Status/Importance | Enter: Save | Esc: Cancel"
    } else {
        "↑/↓: Navigate | ←/→: Status/Importance | Enter: Confirm | Esc: Cancel"
    };

    let title = Line::from(vec![Span::styled(
        title_text,
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )]);
    let hint = Line::from(Span::styled(
        hint_text,
        Style::default().fg(Color::DarkGray),
    ));
    let header = Paragraph::new(vec![title, hint]);
    f.render_widget(header, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();
    for (i, (label, value)) in fields.iter().enumerate() {
        let is_focused = i == focused_field;
        let is_editable_field = matches!(i, 2..=4 | 6); // assigned_to, start_date, due_date, description
        let is_description = i == 6;
        let focused_style = if is_focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let label_style = if is_focused {
            focused_style
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD)
        };
        let value_style = if is_focused {
            focused_style
        } else if value.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        // Show cursor when focused on editable fields
        let display_value = if is_focused && is_editing && is_editable_field {
            format!("{}_", value)
        } else {
            value.clone()
        };

        if is_description {
            // Description field - wrap to multiple lines
            let label_line = Line::from(vec![Span::styled(format!("  {}: ", label), label_style)]);
            lines.push(label_line);

            if value.is_empty() && !display_value.ends_with('_') {
                lines.push(Line::from(vec![Span::styled(
                    "    empty",
                    Style::default().fg(Color::DarkGray),
                )]));
            } else {
                // Wrap description text based on available width
                let wrap_width = (area.width.saturating_sub(8)) as usize;
                let wrapped_text = if display_value.len() > wrap_width && wrap_width > 10 {
                    let mut wrapped = String::new();
                    for (idx, c) in display_value.chars().enumerate() {
                        if idx > 0 && idx % 50 == 0 {
                            wrapped.push('\n');
                        }
                        wrapped.push(c);
                    }
                    wrapped
                } else {
                    display_value.clone()
                };

                for line_text in wrapped_text.lines() {
                    lines.push(Line::from(vec![Span::styled(
                        format!("    {}", line_text),
                        value_style,
                    )]));
                }
            }
        } else {
            let mut spans = vec![
                Span::styled(format!("  {}: ", label), label_style),
                Span::styled(display_value.clone(), value_style),
            ];
            if i == focused_field && matches!(i, 1 | 5) {
                spans.push(Span::raw("    "));
                spans.push(Span::styled("[ ", Style::default().fg(Color::DarkGray)));
                let choices = if i == 1 {
                    app.config.workflow.iter()
                } else {
                    app.config.importance.iter()
                };
                for option in choices {
                    let selected_value = if i == 1 {
                        &task.status
                    } else {
                        task.importance.as_deref().unwrap_or_default()
                    };
                    let is_selected = option.eq_ignore_ascii_case(selected_value);
                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::LightBlue)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    spans.push(Span::styled(option.clone(), style));
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::styled("]", Style::default().fg(Color::DarkGray)));
            }
            let line = if value.is_empty() && !display_value.ends_with('_') {
                Line::from(vec![
                    Span::styled(format!("  {}: ", label), label_style),
                    Span::styled("empty", Style::default().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(spans)
            };
            lines.push(line);
        }
    }

    let fields_para =
        Paragraph::new(lines).block(Block::default().borders(Borders::NONE).title(Span::styled(
            task.task_name.as_str(),
            Style::default().fg(Color::DarkGray),
        )));
    f.render_widget(fields_para, chunks[1]);

    let buttons = if app.task_wizard_tree_edit_mode {
        ["SAVE", "CANCEL"]
    } else {
        ["ADD TO PLAN", "CANCEL"]
    };
    let mut spans: Vec<Span> = Vec::new();
    let confirm_offset = fields.len();
    for (i, label) in buttons.iter().enumerate() {
        let is_focused = app.task_wizard.as_ref().unwrap().field_index == confirm_offset + i;
        let style = if is_focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        spans.push(Span::styled(format!(" {} ", label), style));
        if i < buttons.len() - 1 {
            spans.push(Span::raw("  "));
        }
    }
    let buttons_para = Paragraph::new(Line::from(spans));
    f.render_widget(buttons_para, chunks[2]);
}

/// Renders the two-step note creation wizard (category picker → folder selection/input).
pub fn render_note_wizard(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use crate::tui::{NoteCreationMode, NoteWizardStep};
    use ratatui::layout::{Constraint, Layout};
    use ratatui::text::{Line, Span};

    let Some(ref state) = app.note_creation_state else {
        return;
    };

    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    match state.step {
        NoteWizardStep::Category => {
            let title = if state.mode == NoteCreationMode::FolderOnly {
                "Notes → Select Category (New Folder)"
            } else {
                "Notes → Select Category"
            };
            let header = Paragraph::new(vec![
                Line::from(vec![Span::styled(
                    title,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )]),
                Line::from(Span::styled(
                    "↑/↓: Navigate  Enter: Confirm  Esc: Cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ]);
            f.render_widget(header, chunks[0]);

            let items: Vec<ListItem> = state
                .categories
                .iter()
                .enumerate()
                .map(|(i, cat)| {
                    let style = if i == state.selected_category_index {
                        Style::default().fg(Color::Black).bg(Color::LightBlue)
                    } else {
                        app.text_primary()
                    };
                    ListItem::new(format!("  {}", cat)).style(style)
                })
                .collect();
            let list = List::new(items);
            f.render_widget(list, chunks[1]);
        }
        NoteWizardStep::Folder => {
            let category = state
                .categories
                .get(state.selected_category_index)
                .cloned()
                .unwrap_or_default();
            let folder_title = match state.mode {
                NoteCreationMode::FolderOnly => format!("Notes → {} → New Folder", category),
                NoteCreationMode::Note => format!("Notes → {} → Destination", category),
            };
            let help_text = match state.mode {
                NoteCreationMode::FolderOnly => "Type a folder name  Enter: Create  Esc: Cancel",
                NoteCreationMode::Note => "↑/↓: Select destination  Enter: Confirm  Esc: Cancel",
            };
            let header = Paragraph::new(vec![
                Line::from(vec![Span::styled(
                    folder_title,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )]),
                Line::from(Span::styled(
                    help_text,
                    Style::default().fg(Color::DarkGray),
                )),
            ]);
            f.render_widget(header, chunks[0]);

            if state.mode == NoteCreationMode::FolderOnly {
                let display = if state.folder_input.is_empty() {
                    Span::styled("(none)", Style::default().fg(Color::DarkGray))
                } else {
                    Span::styled(
                        state.folder_input.as_str(),
                        Style::default().fg(Color::White),
                    )
                };
                let body = Paragraph::new(Line::from(vec![
                    Span::styled("  Subfolder: ", app.text_primary()),
                    display,
                ]));
                f.render_widget(body, chunks[1]);
            } else {
                let mut options: Vec<String> =
                    Vec::with_capacity(state.available_folders.len() + 1);
                options.push("(none)".to_string());
                options.extend(state.available_folders.iter().cloned());

                let items: Vec<ListItem> = options
                    .iter()
                    .enumerate()
                    .map(|(i, folder)| {
                        let style = if i == state.selected_folder_index {
                            Style::default().fg(Color::Black).bg(Color::LightBlue)
                        } else {
                            app.text_primary()
                        };
                        ListItem::new(format!("  {}", folder)).style(style)
                    })
                    .collect();
                let list = List::new(items);
                f.render_widget(list, chunks[1]);
            }
        }
    }
}

/// Renders the move-note destination picker.
pub fn render_move_note_picker(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout};
    use ratatui::text::{Line, Span};

    let Some(ref state) = app.move_note_state else {
        return;
    };

    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let source_name = state
        .source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Move: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                source_name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "↑/↓: Navigate  Type to filter  Enter: Move  Esc: Cancel",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("Filter: ", Style::default().fg(Color::DarkGray)),
            Span::styled(state.filter.as_str(), Style::default().fg(Color::White)),
        ]),
    ]);
    f.render_widget(header, chunks[0]);

    let filtered = state.filtered_destinations();
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, (_, dest))| {
            let style = if i == state.selected_index {
                Style::default().fg(Color::Black).bg(Color::LightBlue)
            } else {
                app.text_primary()
            };
            ListItem::new(format!("  {}", dest)).style(style)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_text_strips_frontmatter_and_renders_heading() {
        let md = r#"---
uuid: abc
title: Test
---

# Heading
Some body text
"#;
        let text = markdown_to_text(md);
        let rendered = text
            .lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(!rendered.contains("uuid: abc"));
        assert!(rendered.contains("Heading"));
        assert!(rendered.contains("Some body text"));
    }

    #[test]
    fn test_parse_list_item_detects_bullets_and_numbers() {
        assert_eq!(parse_list_item("- item"), Some("item"));
        assert_eq!(parse_list_item("* item"), Some("item"));
        assert_eq!(parse_list_item("2. item"), Some("item"));
        assert_eq!(parse_list_item("plain"), None);
    }

    #[test]
    fn test_split_yaml_frontmatter_returns_body() {
        let md = "---\nstatus: active\nowner: me\n---\n\n# Description\nhello";
        let (frontmatter, body) = split_yaml_frontmatter(md);

        assert_eq!(frontmatter, Some("status: active\nowner: me\n"));
        assert!(body.trim_start().starts_with("# Description"));
    }

    #[test]
    fn test_parse_yaml_frontmatter_rows_keeps_key_order() {
        let md = "---\na: one\nb: two\nlist:\n  - x\n  - y\n---\nbody";
        let rows = parse_yaml_frontmatter_rows(md);

        assert_eq!(rows[0], ("a".to_string(), "one".to_string()));
        assert_eq!(rows[1], ("b".to_string(), "two".to_string()));
        assert_eq!(rows[2], ("list".to_string(), "x, y".to_string()));
    }
}
