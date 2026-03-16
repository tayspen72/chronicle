// Views module - content display handlers

use crate::storage::{JournalStorage, WorkspaceStorage};
use crate::tui::{App, Mode};
use ratatui::{
    Frame,
    layout::Constraint,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap},
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
            } else if let Some(plan_type) = &item.is_planning_item {
                match plan_type.as_str() {
                    "WeeklyPlanning" => {
                        title = "Current Plan".to_string();
                        if app.planning_session_active {
                            let start = app.planning_session_start_date.as_deref().unwrap_or("?");
                            let end = app.planning_session_due_date.as_deref().unwrap_or("?");
                            let count = app.planning_session_tasks.len();
                            content_to_show = format!(
                                "# Current Plan\n\n\
                                Period: {} to {}\n\
                                Tasks: {}\n\n\
                                Press / to access planning commands (Review Session, Close Planning Session, etc.)",
                                start, end, count
                            );
                        } else {
                            content_to_show = "# Current Plan\n\n\
                                No active planning session.\n\n\
                                Press / and type 'Start Planning Session' to begin planning."
                                .to_string();
                        }
                    }
                    "Backlog" => {
                        title = "Backlog".to_string();
                        content_to_show = "Under development".to_string();
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

    let paragraph = Paragraph::new(markdown_to_text(&content_to_show))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false })
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
    let _ = app;
    let paragraph = Paragraph::new("Under development")
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Backlog"),
        );
    f.render_widget(paragraph, area);
}

/// Renders the Current Plan view showing tasks organized by hierarchy with workflow columns.
pub fn render_weekly_planning(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let workflow_columns = &app.config.workflow;

    // Build header row
    let mut header_cells = vec![
        Cell::from("Program"),
        Cell::from("Project"),
        Cell::from("Milestone"),
        Cell::from("Task"),
    ];
    header_cells.extend(workflow_columns.iter().map(|s| Cell::from(s.as_str())));

    let header = Row::new(header_cells)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray))
        .height(1);

    // Collect tasks and build rows
    let is_review = app.mode == Mode::ReviewSession;
    let selected_idx = app.review_selection_index;
    let rolled_over = &app.rolled_over_tasks;
    let (rows, completed_count, total_count) =
        build_planning_rows(workflow_columns, is_review, selected_idx, rolled_over, app);

    // Calculate column widths
    let base_width = 80 / 4;
    let workflow_width = 20 / workflow_columns.len().max(1) as u16;
    let mut constraints = vec![
        Constraint::Percentage(base_width),
        Constraint::Percentage(base_width),
        Constraint::Percentage(base_width),
        Constraint::Percentage(base_width),
    ];
    constraints.extend(std::iter::repeat_n(
        Constraint::Percentage(workflow_width),
        workflow_columns.len(),
    ));

    let title = if app.mode == Mode::ReviewSession {
        format!("Review Session [{}/{}]", completed_count, total_count)
    } else if app.planning_session_active {
        format!("Current Plan [{}/{}]", completed_count, total_count)
    } else {
        "Current Plan".to_string()
    };

    let table = Table::new(rows, &constraints)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .column_spacing(1);

    f.render_widget(table, area);

    // Show action hints in review mode
    if app.mode == Mode::ReviewSession {
        let hints = Paragraph::new(
            "s: status | r: roll | d: done | x: remove | a: assign | b: start | e: due | m: add | c: close | Enter: confirm | Esc: cancel",
        )
        .style(Style::default().fg(Color::DarkGray));
        let hint_area =
            ratatui::layout::Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
        f.render_widget(hints, hint_area);
    }
}

/// Planning row data for rendering
struct PlanningRowData {
    program: String,
    project: String,
    milestone: String,
    task: String,
    status: String,
    is_selected: bool,
    is_rolled: bool,
}

impl PlanningRowData {
    fn into_row(self, workflow_columns: &[String]) -> Row<'static> {
        let style = if self.is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let task_display = if self.is_rolled {
            format!("{} [R]", self.task)
        } else {
            self.task
        };
        let mut cells = vec![
            Cell::from(self.program).style(style),
            Cell::from(self.project).style(style),
            Cell::from(self.milestone).style(style),
            Cell::from(task_display).style(style),
        ];
        for ws in workflow_columns {
            let cell_style = if ws == &self.status {
                Style::default().fg(Color::Black).bg(Color::LightGreen)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            cells.push(Cell::from(" ").style(cell_style));
        }
        Row::new(cells).height(1)
    }
}

/// Builds rows for the planning table, returning (rows, completed_count, total_count).
fn build_planning_rows(
    workflow_columns: &[String],
    is_review: bool,
    selected_idx: usize,
    rolled_over: &[String],
    app: &App,
) -> (Vec<Row<'static>>, usize, usize) {
    if app.planning_session_active && !app.planning_session_tasks.is_empty() {
        let total = app.planning_session_tasks.len();
        let completed = app
            .planning_session_tasks
            .iter()
            .filter(|t| t.status == "done" || t.status == "complete")
            .count();
        let rows: Vec<Row> = app
            .planning_session_tasks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                PlanningRowData {
                    program: t.program.clone(),
                    project: t.project.clone(),
                    milestone: t.milestone.clone(),
                    task: t.task_name.clone(),
                    status: t.status.clone(),
                    is_selected: is_review && i == selected_idx,
                    is_rolled: rolled_over.contains(&t.uuid),
                }
                .into_row(workflow_columns)
            })
            .collect();
        (rows, completed, total)
    } else if app.planning_session_active {
        // Active session but no tasks selected
        let mut cells = vec![
            Cell::from("No tasks selected"),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ];
        for _ in workflow_columns {
            cells.push(Cell::from(""));
        }
        (vec![Row::new(cells).height(1)], 0, 0)
    } else {
        // No active session - show all tasks with hint
        let mut rows = Vec::new();
        if let Ok(programs) = app.config.workspace.list_programs() {
            for program in programs {
                if let Ok(projects) = app.config.workspace.list_projects(&program.name) {
                    for project in projects {
                        if let Ok(milestones) = app
                            .config
                            .workspace
                            .list_milestones(&program.name, &project.name)
                        {
                            for milestone in milestones {
                                if let Ok(tasks) = app.config.workspace.list_tasks(
                                    &program.name,
                                    &project.name,
                                    &milestone.name,
                                ) {
                                    for task in tasks {
                                        let status = app
                                            .config
                                            .workspace
                                            .read_md_file(&task.path)
                                            .ok()
                                            .map(|c| extract_status_from_content(&c))
                                            .unwrap_or_else(|| "Unknown".into());
                                        rows.push(
                                            PlanningRowData {
                                                program: program.name.clone(),
                                                project: project.name.clone(),
                                                milestone: milestone.name.clone(),
                                                task: task.name.clone(),
                                                status,
                                                is_selected: false,
                                                is_rolled: false,
                                            }
                                            .into_row(workflow_columns),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if rows.is_empty() {
            let mut cells = vec![
                Cell::from("No tasks found"),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ];
            for _ in workflow_columns {
                cells.push(Cell::from(""));
            }
            rows.push(Row::new(cells).height(1));
        }
        (rows, 0, 0)
    }
}

/// Extract status from markdown content (simple frontmatter parsing)
fn extract_status_from_content(content: &str) -> String {
    // Look for "status: <value>" in the frontmatter
    for line in content.lines() {
        if line.starts_with("status:") {
            return line.trim_start_matches("status:").trim().to_string();
        }
    }
    "Unknown".to_string()
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

    let paragraph = Paragraph::new(markdown_to_text(&content))
        .style(Style::default().fg(Color::White))
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
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return content;
    }

    let mut byte_offset = 0usize;
    for line in content.lines() {
        byte_offset += line.len() + 1;
        if line == "---" && byte_offset > 4 {
            return &content[byte_offset..];
        }
    }
    content
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

/// Renders the template field wizard with inline editing - field names and values on same line.
pub fn render_template_fields(f: &mut Frame, app: &App, area: ratatui::layout::Rect, prompt: &str) {
    use crate::tui::WizardFocus;
    use ratatui::layout::{Constraint, Layout};

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
    let header_height = 3u16; // prompt + blank + instructions
    let button_height = 2u16; // blank + buttons
    let available_height = area.height.saturating_sub(header_height + button_height);

    // Calculate how many fields can be visible
    let visible_fields = available_height as usize;

    // Calculate scroll offset based on focused field
    let focused_field_idx = match state.focus {
        WizardFocus::Field(idx) => idx,
        WizardFocus::ConfirmButton | WizardFocus::CancelButton => field_count,
    };
    let scroll_offset = if focused_field_idx >= visible_fields {
        focused_field_idx - visible_fields + 1
    } else {
        0
    };

    // Create vertical layout chunks
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(header_height),
                Constraint::Min(1),
                Constraint::Length(button_height),
            ]
            .as_ref(),
        )
        .split(area);

    // Render header (prompt and instructions) - prompt is bold
    use ratatui::text::{Line, Span};
    let prompt_line = Line::from(vec![Span::styled(
        prompt,
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )]);
    let instructions_line = Line::from(Span::styled(
        "↑/↓: Navigate | Enter: Next/Confirm | Esc: Cancel",
        Style::default().fg(Color::DarkGray),
    ));
    let header = Paragraph::new(vec![prompt_line, instructions_line]);
    f.render_widget(header, chunks[0]);

    // Render fields in a scrollable area
    let mut field_lines: Vec<(String, String, bool, bool)> = Vec::new();

    // Scroll indicator if needed
    if field_count > visible_fields && scroll_offset > 0 {
        field_lines.push((
            format!("  ↑ {} more fields above...", scroll_offset),
            String::new(),
            false,
            false,
        ));
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

        // Create horizontal layout for this field
        // Use :: separator, "empty" for unfilled, "(auto-filled)" suffix
        let label_text = field.label.clone();
        let value_text = if field.value.is_empty() && field.is_editable {
            "empty".to_string()
        } else if field.is_editable {
            field.value.clone()
        } else {
            format!("{} (auto-filled)", field.value)
        };

        field_lines.push((label_text, value_text, is_focused, field.is_editable));
    }

    // Scroll indicator if there are more fields below
    if field_count > visible_fields && scroll_offset + visible_fields < field_count {
        field_lines.push((
            format!(
                "  ↓ {} more fields below...",
                field_count - scroll_offset - visible_fields
            ),
            String::new(),
            false,
            false,
        ));
    }

    // Render all field lines with proper styling

    let mut lines_vec: Vec<Line> = Vec::new();

    for (label, value, focused, editable) in &field_lines {
        if value.is_empty() {
            // Scroll indicator
            lines_vec.push(Line::from(Span::styled(
                format!("  {}", label),
                Style::default().fg(Color::DarkGray),
            )));
        } else if *focused && *editable {
            // Focused editable field - background highlight, no arrow
            lines_vec.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", label),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightBlue)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled(
                    value,
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightBlue)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
            ]));
        } else if *editable {
            // Non-focused editable field - bold label
            let is_empty = value == "empty";
            lines_vec.push(Line::from(vec![
                Span::styled("  ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}: ", label),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled(
                    value,
                    Style::default().fg(if is_empty {
                        Color::DarkGray
                    } else {
                        Color::White
                    }),
                ),
            ]));
        } else {
            // Prepopulated/auto-filled field - bold label, white text with "(auto-filled)"
            lines_vec.push(Line::from(vec![
                Span::styled("  ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}: ", label),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled(value, Style::default().fg(Color::White)),
            ]));
        }
    }

    let fields_widget = Paragraph::new(lines_vec);
    f.render_widget(fields_widget, chunks[1]);

    // Render buttons - no brackets, use background highlight for selection
    const CONFIRM_TEXT: &str = "CONFIRM";
    const CANCEL_TEXT: &str = "CANCEL";

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

    let buttons = Paragraph::new(Line::from(vec![
        Span::styled(CONFIRM_TEXT, confirm_style),
        Span::styled("     ", Style::default().fg(Color::DarkGray)),
        Span::styled(CANCEL_TEXT, cancel_style),
    ]));
    f.render_widget(buttons, chunks[2]);
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
    let title = if let (Some(program), Some(project)) = (&app.current_program, &app.current_project)
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
    let title = if let (Some(program), Some(project), Some(milestone)) = (
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
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

    let prompt_line = Line::from(vec![Span::styled(
        "Start Planning Session",
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )]);
    let instructions_line = Line::from(Span::styled(
        "↑/↓: Navigate | Enter: Next/Confirm | Esc: Cancel",
        Style::default().fg(Color::DarkGray),
    ));
    let header = Paragraph::new(vec![prompt_line, instructions_line]);
    f.render_widget(header, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();

    // Start date field
    let start_date_display_with_cursor = format!("{}_", start_date_display);
    if focus == 0 {
        lines.push(Line::from(vec![
            Span::styled(
                "  Start date: ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                &start_date_display_with_cursor,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            "    Type to enter custom date (YYYY-MM-DD)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::White)),
            Span::styled(
                "Start date: ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(&start_date_display, Style::default().fg(Color::White)),
        ]));
    }

    // Duration field
    if focus == 1 {
        lines.push(Line::from(vec![
            Span::styled(
                "  Duration: ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                duration,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            "    ←/→: Cycle | weekly, biweekly, 6weekly",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::White)),
            Span::styled(
                "Duration: ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(duration, Style::default().fg(Color::White)),
        ]));
    }

    // End date field - editable, allows overriding auto-calculated date
    // When focused (editing), show input_buffer; otherwise show stored or auto-calculated value
    let end_date_display = if focus == 2 {
        wizard.input_buffer.clone()
    } else if wizard.end_date.is_empty() {
        end_date.clone()
    } else {
        wizard.end_date.clone()
    };
    let end_date_display_with_cursor = format!("{}_", end_date_display);

    if focus == 2 {
        lines.push(Line::from(vec![
            Span::styled(
                "  End date: ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                &end_date_display_with_cursor,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]));
        // Show hint about auto-calculation - only when field is focused
        if wizard.end_date.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("    (suggested based on {} duration)", duration),
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "    Enter to confirm your custom date",
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::White)),
            Span::styled(
                "End date: ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(&end_date_display, Style::default().fg(Color::White)),
        ]));
    }

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
        format!(
            "{} → {}",
            app.planning_session_start_date.as_deref().unwrap_or("?"),
            app.planning_session_due_date.as_deref().unwrap_or("?")
        ),
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
            Constraint::Length(3),
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
    let breadcrumb_line = Line::from(Span::styled(
        picker.breadcrumb(),
        Style::default().fg(Color::DarkGray),
    ));
    let header = Paragraph::new(vec![title_line, breadcrumb_line]);
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

    // Show element type with bold/styled formatting for clarity
    let level_indicator = match picker.level {
        PickerLevel::Programs => "📁 Programs",
        PickerLevel::Projects => "📂 Projects",
        PickerLevel::Milestones => "🎯 Milestones",
        PickerLevel::Tasks => "✓ Tasks",
    };
    let level_title = format!("{} ({} items)", level_indicator, picker.items.len());
    let list = List::new(items).block(
        ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::NONE)
            .title(level_title),
    );
    f.render_widget(list, chunks[2]);

    let hint_text = match picker.level {
        PickerLevel::Programs => "↑/↓: Navigate | Enter: Select | ←/→: Back/Forward | Esc: Cancel",
        PickerLevel::Projects => "↑/↓: Navigate | Enter: Select | ←/→: Back/Forward | Esc: Back",
        PickerLevel::Milestones => "↑/↓: Navigate | Enter: Select | ←/→: Back/Forward | Esc: Back",
        PickerLevel::Tasks => "↑/↓: Navigate | Enter: Add Task | Esc: Back",
    };
    let session_task_count = app.planning_session_tasks.len();
    let count_text = if session_task_count > 0 && picker.is_wizard_mode {
        format!("{} | f: Finish | Tasks: {}", hint_text, session_task_count)
    } else {
        hint_text.to_string()
    };
    let buttons = Paragraph::new(Line::from(Span::styled(
        count_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(buttons, chunks[3]);
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
        &app.planning_session_start_date,
        &app.planning_session_due_date,
    ) {
        format!("{} → {}", start, end)
    } else {
        "No dates set".to_string()
    };
    let subtitle = Line::from(Span::styled(
        date_range,
        Style::default().fg(Color::DarkGray),
    ));
    let header = Paragraph::new(vec![title, subtitle]);
    f.render_widget(header, chunks[0]);

    let tasks = &app.planning_session_tasks;
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
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!("Tasks ({})", task_count)),
    );
    f.render_widget(tasks_para, chunks[1]);

    let buttons = ["ADD MORE", "CONFIRM", "CANCEL"];
    let mut spans = Vec::new();
    for (i, label) in buttons.iter().enumerate() {
        let style = if i == app.planning_preview_focus {
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
    if app.planning_preview_confirmed {
        let confirm_msg = Line::from(vec![Span::styled(
            " ✓ Session confirmed and saved! ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )]);
        let confirm_para = Paragraph::new(confirm_msg);
        // Render below buttons
        let confirm_area = ratatui::layout::Rect {
            x: chunks[2].x,
            y: chunks[2].y + 2,
            width: chunks[2].width,
            height: 1,
        };
        f.render_widget(confirm_para, confirm_area);
    }
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
        ("Priority", task.priority.clone().unwrap_or_default()),
        ("Description", task.description.clone().unwrap_or_default()),
    ];

    let chunks = Layout::default()
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Line::from(vec![Span::styled(
        "Edit Task Details",
        Style::default()
            .fg(Color::White)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )]);
    let hint = Line::from(Span::styled(
        "↑/↓: Navigate | Enter: Next | Type: Edit | Esc: Cancel",
        Style::default().fg(Color::DarkGray),
    ));
    let header = Paragraph::new(vec![title, hint]);
    f.render_widget(header, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();
    for (i, (label, value)) in fields.iter().enumerate() {
        let is_focused = i == focused_field;
        let is_editable_field = matches!(i, 2..=4 | 6); // assigned_to, start_date, due_date, description
        let is_description = i == 6;
        let style = if is_focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
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
            let label_line = Line::from(vec![Span::styled(format!("  {}: ", label), style)]);
            lines.push(label_line);

            if value.is_empty() && !display_value.ends_with('_') {
                lines.push(Line::from(vec![Span::styled(
                    "    (empty)",
                    Style::default().fg(Color::DarkGray),
                )]));
            } else {
                // Wrap description text - split into chunks of ~50 chars
                let wrapped_text = if display_value.len() > 50 {
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
                        style,
                    )]));
                }
            }
        } else {
            let line = if value.is_empty() && !display_value.ends_with('_') {
                Line::from(vec![
                    Span::styled(format!("  {}: ", label), style),
                    Span::styled("(empty)", Style::default().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(format!("  {}: ", label), style),
                    Span::styled(display_value, style),
                ])
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

    let buttons = ["ADD TO PLAN", "CANCEL"];
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
}
