use super::views;
use crate::tui::{App, Mode, ViewType};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

#[derive(Debug, Clone)]
pub struct TreeItem {
    pub name: String,
    pub indent: usize,
}

pub fn compute_continuation_levels(items: &[TreeItem]) -> Vec<Vec<bool>> {
    let max_indent = items.iter().map(|i| i.indent).max().unwrap_or(0);
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            (1..=max_indent)
                .map(|d| {
                    if item.indent > d + 1 {
                        true
                    } else {
                        items[i + 1..].iter().any(|x| x.indent >= d)
                    }
                })
                .collect()
        })
        .collect()
}

pub fn tree_prefix_for_item(
    items: &[TreeItem],
    item_index: usize,
    continuation_levels: &[Vec<bool>],
) -> String {
    let item = &items[item_index];
    if item.indent == 0 {
        return item.name.clone();
    }

    // Pipes for levels 1 through indent-1 (level 0 is root, never has pipes)
    let pipes: String = (1..item.indent)
        .map(|d| {
            let has_pipe = continuation_levels
                .get(item_index)
                .and_then(|l| l.get(d - 1))
                .copied()
                .unwrap_or(false);
            if has_pipe { "│   " } else { "    " }
        })
        .collect();

    let is_last = items[item_index + 1..]
        .iter()
        .all(|p| p.indent != item.indent);

    let prefix = if is_last { "└── " } else { "├── " };

    format!("{}{}{}", pipes, prefix, item.name)
}

pub fn render(f: &mut Frame, app: &App) {
    f.render_widget(Block::default().style(app.background_style()), f.area());

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
        String::new()
    };

    let command_bar = Paragraph::new(command_text)
        .style(app.text_secondary())
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

    // Theme selection overlay
    if matches!(app.mode, Mode::ThemeSelection) {
        render_theme_selection(f, app);
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
                let current_path = item.tree_path.as_ref().or(item.journal_path.as_ref());
                if let Some(current_path) = current_path {
                    let parent_path = &current_path[..current_path.len().saturating_sub(1)];

                    // Pipes for levels 1..indent-1, constrained to the same section/tree.
                    let pipes: String = (1..item.indent)
                        .map(|depth| {
                            if depth == 1 {
                                // Never draw root-level continuation pipes.
                                return "    ";
                            }
                            let ancestor = &current_path[..depth];
                            let ancestor_parent = &current_path[..depth.saturating_sub(1)];
                            let has_pipe = items[i + 1..].iter().any(|candidate| {
                                if candidate.section != item.section {
                                    return false;
                                }
                                let candidate_path = candidate
                                    .tree_path
                                    .as_ref()
                                    .or(candidate.journal_path.as_ref());
                                let Some(candidate_path) = candidate_path else {
                                    return false;
                                };
                                if candidate_path.len() < depth
                                    || !candidate_path.starts_with(ancestor_parent)
                                {
                                    return false;
                                }
                                candidate_path[depth - 1] != ancestor[depth - 1]
                            });
                            if has_pipe { "│   " } else { "    " }
                        })
                        .collect();

                    let has_next_sibling = items[i + 1..].iter().any(|candidate| {
                        if candidate.section != item.section {
                            return false;
                        }
                        let candidate_path = candidate
                            .tree_path
                            .as_ref()
                            .or(candidate.journal_path.as_ref());
                        let Some(candidate_path) = candidate_path else {
                            return false;
                        };
                        candidate_path.len() == current_path.len()
                            && candidate_path.starts_with(parent_path)
                    });

                    let tree_prefix = if has_next_sibling {
                        "├── "
                    } else {
                        "└── "
                    };
                    format!("{}{}{}", pipes, tree_prefix, item.name)
                } else {
                    let is_last_in_section = items[i + 1..]
                        .iter()
                        .filter(|candidate| candidate.section == item.section)
                        .all(|candidate| candidate.indent != item.indent);
                    let tree_prefix = if is_last_in_section {
                        "└── "
                    } else {
                        "├── "
                    };
                    format!(
                        "{}{}{}",
                        "    ".repeat(item.indent.saturating_sub(1)),
                        tree_prefix,
                        item.name
                    )
                }
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
                app.sidebar_header_style()
            } else if item.is_create_action {
                app.sidebar_create_action_style(is_selected)
            } else if is_selected {
                app.sidebar_selected_style()
            } else {
                app.sidebar_style()
            };
            ListItem::new(full_label).style(style)
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style())
                .title("Navigator"),
        )
        .style(app.text_primary());

    f.render_widget(list, area);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    match app.current_view {
        ViewType::TreeView => views::render_tree_view(f, app, area),
        ViewType::Journal => views::render_journal_welcome(f, app, area),
        ViewType::JournalArchiveList => views::render_archive_list(f, app, area),
        ViewType::JournalToday => views::render_journal_today(f, app, area),
        ViewType::Backlog => views::render_backlog(f, app, area),
        ViewType::MyTasks => views::render_my_tasks(f, app, area),
        ViewType::WeeklyPlanning => views::render_weekly_planning(f, app, area),
        ViewType::ViewingContent => views::render_content_viewer(f, app, area),
        ViewType::InputProgram => views::render_input(f, app, area, "Enter program name:"),
        ViewType::InputProject => views::render_input(f, app, area, "Enter project name:"),
        ViewType::InputMilestone => views::render_input(f, app, area, "Enter milestone name:"),
        ViewType::InputTask => views::render_input(f, app, area, "Enter task name:"),
        ViewType::InputTemplateField => {
            views::render_template_fields(f, app, area);
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
        ViewType::InputNote => {
            views::render_note_wizard(f, app, area);
        }
        ViewType::MoveNote => {
            views::render_move_note_picker(f, app, area);
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
        .style(app.command_input_style())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.command_border_style())
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
            let style = if !cmd.selectable {
                app.command_section_style()
            } else if idx == app.command_palette.selection_index {
                app.command_result_selected_style()
            } else {
                app.command_result_style()
            };
            ListItem::new(cmd.display_label.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.border_style()),
        )
        .style(app.command_result_style());

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

    let mode_text = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::CommandPalette => "COMMAND",
        Mode::Input => "INPUT",
        Mode::TaskSelection => "SELECT",
        Mode::ReviewSession => "REVIEW",
        Mode::HierarchicalSelection => "ADD TASKS",
        Mode::PlanningPreview => "PREVIEW",
        Mode::TaskDetailWizard => "EDIT TASK",
        Mode::InputTaskDetailField => "INPUT",
        Mode::CurrentPlanNavigation => "PLAN NAV",
        Mode::ThemeSelection => "THEME",
    };

    let mode_color = app.status_color();

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
        .style(app.text_secondary())
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(breadcrumb_widget, chunks[0]);

    // Render mode indicator (right side)
    let mode_widget = Paragraph::new(mode_text)
        .style(Style::default().fg(mode_color))
        .block(Block::default().borders(Borders::NONE))
        .alignment(ratatui::layout::Alignment::Right);
    f.render_widget(mode_widget, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bug1_last_child_missing_pipe() {
        // Bug #1: When expanding level 2 to see level 3 elements,
        // the last level 3 child should have a pipe on level 2 row.
        // Structure:
        // Alpha1 (indent 0)
        // ├── Beta1 (indent 1)
        // │   ├── Gamma1 (indent 2)
        // │   ├── Gamma2 (indent 2)
        // │   └── Gamma3 (indent 2) <- last child, needs pipe from Beta1
        // ├── Beta2 (indent 1)
        // └── Beta3 (indent 1)
        let items = vec![
            TreeItem {
                name: "Alpha1".to_string(),
                indent: 0,
            },
            TreeItem {
                name: "Beta1".to_string(),
                indent: 1,
            },
            TreeItem {
                name: "Gamma1".to_string(),
                indent: 2,
            },
            TreeItem {
                name: "Gamma2".to_string(),
                indent: 2,
            },
            TreeItem {
                name: "Gamma3".to_string(),
                indent: 2,
            },
            TreeItem {
                name: "Beta2".to_string(),
                indent: 1,
            },
            TreeItem {
                name: "Beta3".to_string(),
                indent: 1,
            },
        ];

        let continuation = compute_continuation_levels(&items);

        // Beta1 (index 1) is at indent 1 - it should NOT have a pipe at level 0
        // because level 0 is the root level and root items never have pipes (Rule 6)
        let beta1_prefix = tree_prefix_for_item(&items, 1, &continuation);
        assert!(
            !beta1_prefix.starts_with("│"),
            "Beta1 should NOT have pipe at level 0 (root). Got: {}",
            beta1_prefix
        );

        // But Beta1 SHOULD have a pipe at level 1 (from its own indentation)
        // because Gamma3 comes after it at deeper indent
        // continuation starts at d=1, so level 1 is at index 0
        let beta1_cont_at_1 = continuation.get(1).and_then(|c| c.get(0));
        assert_eq!(
            beta1_cont_at_1,
            Some(&true),
            "Beta1 should have pipe at level 1 because it has descendants"
        );

        // Check Gamma3 (index 4) - this is the last child of Beta1
        // It should also have a pipe because Beta1 has more children (Beta2, Beta3)
        // The pipe at level 1 should be present (continuation now starts at d=1, so index 0 = level 1)
        assert_eq!(
            continuation[4].get(0),
            Some(&true),
            "Gamma3 should have pipe at level 1 (from Beta1)"
        );
    }

    #[test]
    fn test_bug3_single_child_with_grandchildren() {
        // Bug #3: When a parent has only one child, and that child has grandchildren,
        // the parent should use └── (not ├──) because it has no siblings.
        // Structure:
        // Alpha (indent 0)
        // └── Beta (indent 1) <- single child, should use └── not ├──
        //     ├── Gamma1 (indent 2)
        //     ├── Gamma2 (indent 2)
        //     └── Gamma3 (indent 2)
        let items = vec![
            TreeItem {
                name: "Alpha".to_string(),
                indent: 0,
            },
            TreeItem {
                name: "Beta".to_string(),
                indent: 1,
            },
            TreeItem {
                name: "Gamma1".to_string(),
                indent: 2,
            },
            TreeItem {
                name: "Gamma2".to_string(),
                indent: 2,
            },
            TreeItem {
                name: "Gamma3".to_string(),
                indent: 2,
            },
        ];

        let continuation = compute_continuation_levels(&items);
        let beta_prefix = tree_prefix_for_item(&items, 1, &continuation);

        // Beta is the last sibling (no items after it with indent 1),
        // so it should use └── even if it has descendants.
        assert!(
            beta_prefix.contains("└──"),
            "Beta should use └── because it is the last sibling. Got: {}",
            beta_prefix
        );
    }

    #[test]
    fn test_single_child_is_last_and_has_descendants() {
        // Edge case: single child that has descendants still uses └──,
        // because it is the last sibling.
        let items = vec![
            TreeItem {
                name: "Parent".to_string(),
                indent: 0,
            },
            TreeItem {
                name: "Child".to_string(),
                indent: 1,
            },
            TreeItem {
                name: "Grandchild".to_string(),
                indent: 2,
            },
        ];

        let continuation = compute_continuation_levels(&items);
        let child_prefix = tree_prefix_for_item(&items, 1, &continuation);

        assert!(
            child_prefix.contains("└──"),
            "Child with grandchildren should use └── as last sibling. Got: {}",
            child_prefix
        );
    }

    #[test]
    fn test_leaf_node_uses_last_marker() {
        // Leaf node (no children) that is last sibling should use └──.
        let items = vec![
            TreeItem {
                name: "Parent".to_string(),
                indent: 0,
            },
            TreeItem {
                name: "Child1".to_string(),
                indent: 1,
            },
            TreeItem {
                name: "Child2".to_string(),
                indent: 1,
            },
        ];

        let continuation = compute_continuation_levels(&items);
        let child2_prefix = tree_prefix_for_item(&items, 2, &continuation);

        assert!(
            child2_prefix.contains("└──"),
            "Last leaf should use └──. Got: {}",
            child2_prefix
        );
    }

    #[test]
    fn test_pipes_for_deep_tree() {
        // Test that pipes are correctly placed for deeply nested trees.
        // Structure:
        // Root (indent 0)
        // └── Level1 (indent 1)
        //     ├── Level2a (indent 2)
        //     └── Level2b (indent 2)
        //         └── Level3 (indent 3)
        let items = vec![
            TreeItem {
                name: "Root".to_string(),
                indent: 0,
            },
            TreeItem {
                name: "Level1".to_string(),
                indent: 1,
            },
            TreeItem {
                name: "Level2a".to_string(),
                indent: 2,
            },
            TreeItem {
                name: "Level2b".to_string(),
                indent: 2,
            },
            TreeItem {
                name: "Level3".to_string(),
                indent: 3,
            },
        ];

        let continuation = compute_continuation_levels(&items);

        // Level1 should NOT have pipe at level 0 (root level has no pipes per Rule 6)
        let level1_prefix = tree_prefix_for_item(&items, 1, &continuation);
        assert!(
            !level1_prefix.starts_with("│"),
            "Level1 should NOT have pipe at level 0 (root). Got: {}",
            level1_prefix
        );

        // But Level1 SHOULD have pipe at level 1 (its own level)
        // because Level2b exists
        // continuation starts at d=1, so index 0 = d=1
        let level1_cont_at_1 = continuation.get(1).and_then(|c| c.get(0));
        assert_eq!(
            level1_cont_at_1,
            Some(&true),
            "Level1 should have pipe at level 1 because Level2b exists"
        );

        // Level2b should have pipe at level 1 (its parent level) because Level3 exists
        // continuation starts at d=1, so index 0 = d=1
        let level2b_cont_at_1 = continuation.get(3).and_then(|c| c.get(0));
        assert_eq!(
            level2b_cont_at_1,
            Some(&true),
            "Level2b should have pipe at level 1 because Level3 exists"
        );
    }
}

fn render_theme_selection(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(5),    // Theme list
            Constraint::Length(3), // Instructions
        ])
        .margin(10)
        .split(area);

    // Clear the background behind the popup
    f.render_widget(Clear, area);

    // Title
    let title = Paragraph::new("Select Theme")
        .style(
            app.text_primary()
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme_border_style()),
        );
    f.render_widget(title, popup[0]);

    // Theme list
    let items: Vec<ListItem> = app
        .available_themes
        .iter()
        .enumerate()
        .map(|(idx, theme)| {
            let is_selected = idx == app.theme_selection_index;
            let is_active = theme == &app.config.theme;
            let style = if is_selected {
                app.sidebar_selected_style()
            } else {
                app.sidebar_style()
            };
            let prefix = if is_selected { "▶ " } else { "  " };
            let suffix = if is_active { " *" } else { "" };
            ListItem::new(format!("{}{}{}", prefix, theme, suffix)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme_border_style()),
        )
        .style(app.text_primary());

    f.render_widget(list, popup[1]);

    // Instructions
    let instructions = Paragraph::new("↑↓ Navigate • Enter Confirm • Esc Cancel  (* = saved)")
        .style(app.text_secondary())
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(instructions, popup[2]);
}
