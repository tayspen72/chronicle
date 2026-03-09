// Views module - content display handlers

use crate::storage::{JournalStorage, WorkspaceStorage};
use crate::tui::App;
use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
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
                        title = "Weekly Planning".to_string();
                        content_to_show = "Under development".to_string();
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

/// Renders the Weekly Planning view showing current week and task statistics.
pub fn render_weekly_planning(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let _ = app;
    let paragraph = Paragraph::new("Under development")
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("Weekly Planning"),
        );

    f.render_widget(paragraph, area);
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
