//! Command palette module.
//!
//! Handles command palette state, filtering, and execution.

use crossterm::event::KeyCode;

use super::ViewType;

/// Actions that can be triggered by commands.
#[derive(Debug, Clone)]
pub enum CommandAction {
    OpenTodayJournal,
    ShowArchiveList,
    ShowProgramsList,
    ShowProjectsList,
    ShowMilestonesList,
    ShowTasksList,
    NewProgram,
    NewProject,
    NewMilestone,
    NewTask,
    NewSubtask,
    NewNote,
    NewNoteFolder,
    MoveNote,
    Refresh,
    StartPlanningSession,
    ClosePlanningSession,
    ReviewSession,
    SwitchTheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    Commands,
    Planning,
    Journal,
    Notes,
    Navigation,
    System,
}

impl CommandCategory {
    pub fn label(self) -> &'static str {
        match self {
            CommandCategory::Commands => "Task Management",
            CommandCategory::Planning => "Planning",
            CommandCategory::Journal => "Journal",
            CommandCategory::Notes => "Notes",
            CommandCategory::Navigation => "Navigation",
            CommandCategory::System => "System",
        }
    }
}

/// A matched command with its label, target view, and optional action.
#[derive(Debug, Clone)]
pub struct CommandMatch {
    pub label: String,
    pub display_label: String,
    pub category: CommandCategory,
    pub view: ViewType,
    pub exit: bool,
    pub action: Option<CommandAction>,
    pub selectable: bool,
}

/// State for the command palette.
#[derive(Debug, Clone, Default)]
pub struct CommandPalette {
    pub input: String,
    pub matches: Vec<CommandMatch>,
    pub selection_index: usize,
}

impl CommandPalette {
    /// Creates a new command palette with all commands loaded.
    pub fn new() -> Self {
        let matches = filter_commands("", None, None, None, None, true);
        let selection_index = first_selectable_index(&matches).unwrap_or(0);
        Self {
            input: String::new(),
            matches,
            selection_index,
        }
    }

    /// Handles keyboard input for the command palette.
    ///
    /// Returns `Some(CommandMatch)` when a command is executed,
    /// `None` otherwise.
    pub fn handle_input(&mut self, code: KeyCode) -> Option<CommandMatch> {
        match code {
            KeyCode::Char(c) => {
                self.input.push(c);
                self.matches = filter_commands(&self.input, None, None, None, None, true);
                self.selection_index = first_selectable_index(&self.matches).unwrap_or(0);
                None
            }
            KeyCode::Backspace => {
                self.input.pop();
                self.matches = filter_commands(&self.input, None, None, None, None, true);
                self.selection_index = first_selectable_index(&self.matches).unwrap_or(0);
                None
            }
            KeyCode::Esc => {
                self.close();
                None
            }
            KeyCode::Enter => {
                let cmd = self
                    .matches
                    .get(self.selection_index)
                    .filter(|m| m.selectable)
                    .cloned();
                self.close();
                cmd
            }
            KeyCode::Up => {
                if self.selection_index > 0
                    && let Some(idx) = prev_selectable_index(&self.matches, self.selection_index)
                {
                    self.selection_index = idx;
                }
                None
            }
            KeyCode::Down => {
                if let Some(idx) = next_selectable_index(&self.matches, self.selection_index) {
                    self.selection_index = idx;
                }
                None
            }
            _ => None,
        }
    }

    /// Closes the command palette and resets its state.
    pub fn close(&mut self) {
        self.input.clear();
        self.selection_index = 0;
    }

    /// Opens the command palette and resets the input.
    pub fn open(&mut self) {
        self.input.clear();
        self.matches = filter_commands("", None, None, None, None, true);
        self.selection_index = first_selectable_index(&self.matches).unwrap_or(0);
    }

    /// Filters commands based on current input (simple filter without context).
    pub fn filter(&mut self) {
        self.matches = filter_commands(&self.input, None, None, None, None, true);
        self.selection_index = first_selectable_index(&self.matches).unwrap_or(0);
    }

    /// Filters commands based on input and navigation context.
    pub fn filter_with_context(
        &mut self,
        current_program: Option<&str>,
        current_project: Option<&str>,
        current_milestone: Option<&str>,
        current_task: Option<&str>,
        has_programs: bool,
    ) {
        self.matches = filter_commands(
            &self.input,
            current_program,
            current_project,
            current_milestone,
            current_task,
            has_programs,
        );
        self.selection_index = first_selectable_index(&self.matches).unwrap_or(0);
    }

    /// Returns the display text for the command bar/palette.
    pub fn display_text(&self) -> String {
        format!("/{}", self.input)
    }
}

fn command_item(
    label: &str,
    category: CommandCategory,
    view: ViewType,
    action: Option<CommandAction>,
) -> CommandMatch {
    CommandMatch {
        label: label.to_string(),
        display_label: label.to_string(),
        category,
        view,
        exit: false,
        action,
        selectable: true,
    }
}

fn section_header(category: CommandCategory) -> CommandMatch {
    CommandMatch {
        label: category.label().to_string(),
        display_label: category.label().to_string(),
        category,
        view: ViewType::TreeView,
        exit: false,
        action: None,
        selectable: false,
    }
}

fn first_selectable_index(matches: &[CommandMatch]) -> Option<usize> {
    matches.iter().position(|m| m.selectable)
}

fn next_selectable_index(matches: &[CommandMatch], from: usize) -> Option<usize> {
    matches
        .iter()
        .enumerate()
        .skip(from.saturating_add(1))
        .find(|(_, m)| m.selectable)
        .map(|(idx, _)| idx)
}

fn prev_selectable_index(matches: &[CommandMatch], from: usize) -> Option<usize> {
    if from == 0 {
        return None;
    }
    matches
        .iter()
        .enumerate()
        .take(from)
        .rev()
        .find(|(_, m)| m.selectable)
        .map(|(idx, _)| idx)
}

fn grouped_commands(commands: &[CommandMatch]) -> Vec<CommandMatch> {
    let order = [
        CommandCategory::Commands,
        CommandCategory::Journal,
        CommandCategory::Notes,
        CommandCategory::Planning,
        CommandCategory::Navigation,
        CommandCategory::System,
    ];

    let mut grouped = Vec::new();
    for category in order {
        let category_commands: Vec<CommandMatch> = commands
            .iter()
            .filter(|c| c.category == category)
            .cloned()
            .collect();
        if !category_commands.is_empty() {
            grouped.push(section_header(category));
            grouped.extend(category_commands);
        }
    }
    grouped
}

/// Returns the list of all available commands.
pub fn get_command_list() -> Vec<CommandMatch> {
    vec![
        command_item(
            "Programs",
            CommandCategory::Navigation,
            ViewType::TreeView,
            Some(CommandAction::ShowProgramsList),
        ),
        command_item(
            "Projects",
            CommandCategory::Navigation,
            ViewType::TreeView,
            Some(CommandAction::ShowProjectsList),
        ),
        command_item(
            "Milestones",
            CommandCategory::Navigation,
            ViewType::TreeView,
            Some(CommandAction::ShowMilestonesList),
        ),
        command_item(
            "Tasks",
            CommandCategory::Navigation,
            ViewType::TreeView,
            Some(CommandAction::ShowTasksList),
        ),
        command_item("Journal", CommandCategory::Journal, ViewType::Journal, None),
        command_item(
            "Backlog",
            CommandCategory::Planning,
            ViewType::Backlog,
            None,
        ),
        command_item(
            "My Tasks",
            CommandCategory::Planning,
            ViewType::MyTasks,
            None,
        ),
        command_item(
            "Current Plan",
            CommandCategory::Planning,
            ViewType::WeeklyPlanning,
            None,
        ),
        command_item(
            "New Program",
            CommandCategory::Commands,
            ViewType::InputProgram,
            Some(CommandAction::NewProgram),
        ),
        command_item(
            "New Project",
            CommandCategory::Commands,
            ViewType::InputProject,
            Some(CommandAction::NewProject),
        ),
        command_item(
            "New Milestone",
            CommandCategory::Commands,
            ViewType::InputMilestone,
            Some(CommandAction::NewMilestone),
        ),
        command_item(
            "New Task",
            CommandCategory::Commands,
            ViewType::InputTask,
            Some(CommandAction::NewTask),
        ),
        command_item(
            "New Subtask",
            CommandCategory::Commands,
            ViewType::InputTask,
            Some(CommandAction::NewSubtask),
        ),
        command_item(
            "New Note",
            CommandCategory::Notes,
            ViewType::InputNote,
            Some(CommandAction::NewNote),
        ),
        command_item(
            "New Note Folder",
            CommandCategory::Notes,
            ViewType::InputNote,
            Some(CommandAction::NewNoteFolder),
        ),
        command_item(
            "Move Note",
            CommandCategory::Notes,
            ViewType::MoveNote,
            Some(CommandAction::MoveNote),
        ),
        command_item(
            "Open Today's Journal",
            CommandCategory::Journal,
            ViewType::Journal,
            Some(CommandAction::OpenTodayJournal),
        ),
        command_item(
            "Journal History",
            CommandCategory::Journal,
            ViewType::Journal,
            Some(CommandAction::ShowArchiveList),
        ),
        command_item(
            "Refresh",
            CommandCategory::System,
            ViewType::TreeView,
            Some(CommandAction::Refresh),
        ),
        command_item(
            "Start Planning Session",
            CommandCategory::Planning,
            ViewType::WeeklyPlanning,
            Some(CommandAction::StartPlanningSession),
        ),
        command_item(
            "Close Planning Session",
            CommandCategory::Planning,
            ViewType::WeeklyPlanning,
            Some(CommandAction::ClosePlanningSession),
        ),
        command_item(
            "Review Session",
            CommandCategory::Planning,
            ViewType::WeeklyPlanning,
            Some(CommandAction::ReviewSession),
        ),
        command_item(
            "Theme",
            CommandCategory::System,
            ViewType::TreeView,
            Some(CommandAction::SwitchTheme),
        ),
        CommandMatch {
            label: "Exit".to_string(),
            display_label: "Exit".to_string(),
            category: CommandCategory::System,
            view: ViewType::Journal,
            exit: true,
            action: None,
            selectable: true,
        },
    ]
}

/// Filters commands based on input and navigation context.
///
/// # Arguments
/// * `input` - The user's search input (lowercase)
/// * `current_program` - Currently selected program (if any)
/// * `current_project` - Currently selected project (if any)
/// * `current_milestone` - Currently selected milestone (if any)
/// * `has_programs` - Whether any programs exist in the workspace
///
/// # Returns
/// A filtered list of matching commands.
pub fn filter_commands(
    input: &str,
    current_program: Option<&str>,
    current_project: Option<&str>,
    current_milestone: Option<&str>,
    current_task: Option<&str>,
    has_programs: bool,
) -> Vec<CommandMatch> {
    let trimmed_input = input.trim();
    let input = trimmed_input.to_lowercase();

    if input.starts_with("journal") || input.starts_with("/journal") {
        let remainder = input
            .trim_start_matches('/')
            .trim_start_matches("journal")
            .trim();

        let journal_commands = vec![
            command_item(
                "Open Today's Journal",
                CommandCategory::Journal,
                ViewType::Journal,
                Some(CommandAction::OpenTodayJournal),
            ),
            command_item(
                "Journal History",
                CommandCategory::Journal,
                ViewType::Journal,
                Some(CommandAction::ShowArchiveList),
            ),
        ];

        if remainder.is_empty() {
            if trimmed_input.is_empty() {
                grouped_commands(&journal_commands)
            } else {
                journal_commands
            }
        } else {
            let filtered: Vec<CommandMatch> = journal_commands
                .into_iter()
                .filter(|cmd| cmd.label.to_lowercase().contains(remainder))
                .collect();
            grouped_commands(&filtered)
        }
    } else {
        let filtered: Vec<CommandMatch> = get_command_list()
            .into_iter()
            .filter(|cmd| {
                let matches_input = cmd.label.to_lowercase().contains(&input);

                // Context-based command availability:
                // - "New Program" ALWAYS available (especially when no programs exist)
                // - "New Project" available when current_program is set
                // - "New Milestone" available when current_program AND current_project are set
                // - "New Task" available when current_program, current_project, AND current_milestone are set
                let is_context_valid = match cmd.label.as_str() {
                    "New Program" => true, // Always available
                    "New Project" => current_program.is_some(),
                    "New Milestone" => current_program.is_some() && current_project.is_some(),
                    "New Task" => {
                        current_program.is_some()
                            && current_project.is_some()
                            && current_milestone.is_some()
                    }
                    "New Subtask" => {
                        current_program.is_some()
                            && current_project.is_some()
                            && current_milestone.is_some()
                            && current_task.is_some()
                    }
                    // Navigation commands - always available
                    "Programs"
                    | "Journal"
                    | "Backlog"
                    | "My Tasks"
                    | "Current Plan"
                    | "Open Today's Journal"
                    | "Journal History"
                    | "New Note"
                    | "New Note Folder"
                    | "Move Note"
                    | "Exit" => true,
                    // Tier-specific navigation - context-based
                    "Projects" => current_program.is_some() || has_programs,
                    "Milestones" => current_project.is_some(),
                    "Tasks" => current_milestone.is_some(),
                    _ => true,
                };

                matches_input && is_context_valid
            })
            .collect();

        if trimmed_input.is_empty() {
            grouped_commands(&filtered)
        } else {
            grouped_commands(&filtered)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_palette_new() {
        let palette = CommandPalette::new();
        assert!(palette.input.is_empty());
        assert!(!palette.matches.is_empty());
        assert!(palette.matches[palette.selection_index].selectable);
    }

    #[test]
    fn test_command_palette_char_input() {
        let mut palette = CommandPalette::new();
        let result = palette.handle_input(KeyCode::Char('a'));
        assert!(result.is_none());
        assert_eq!(palette.input, "a");
        assert!(palette.matches[palette.selection_index].selectable);
    }

    #[test]
    fn test_command_palette_backspace() {
        let mut palette = CommandPalette::new();
        palette.input = "test".to_string();
        let result = palette.handle_input(KeyCode::Backspace);
        assert!(result.is_none());
        assert_eq!(palette.input, "tes");
    }

    #[test]
    fn test_command_palette_escape() {
        let mut palette = CommandPalette::new();
        palette.input = "test".to_string();
        palette.selection_index = 5;
        let result = palette.handle_input(KeyCode::Esc);
        assert!(result.is_none());
        assert!(palette.input.is_empty());
        assert_eq!(palette.selection_index, 0);
    }

    #[test]
    fn test_command_palette_navigation() {
        let mut palette = CommandPalette::new();
        assert!(palette.matches[palette.selection_index].selectable);

        // Navigate down
        let before = palette.selection_index;
        palette.handle_input(KeyCode::Down);
        assert!(palette.selection_index >= before);
        assert!(palette.matches[palette.selection_index].selectable);

        // Navigate up
        palette.handle_input(KeyCode::Up);
        assert!(palette.matches[palette.selection_index].selectable);
    }

    #[test]
    fn test_command_palette_enter() {
        let mut palette = CommandPalette::new();
        let result = palette.handle_input(KeyCode::Enter);
        assert!(result.is_some());
        assert!(palette.input.is_empty());
        assert_eq!(palette.selection_index, 0);
    }

    #[test]
    fn test_filter_commands_empty_input() {
        let commands = filter_commands("", None, None, None, None, true);
        // Should always include "New Program" when no programs exist
        assert!(commands.iter().any(|c| c.label == "New Program"));
    }

    #[test]
    fn test_filter_commands_journal_prefix() {
        let commands = filter_commands("journal", None, None, None, None, true);
        assert!(
            commands
                .iter()
                .all(|c| c.category == CommandCategory::Journal)
        );
    }

    #[test]
    fn test_filter_commands_empty_includes_section_headers() {
        let commands = filter_commands("", None, None, None, None, true);
        assert!(commands.iter().any(|c| !c.selectable));
    }

    #[test]
    fn test_filter_commands_by_context() {
        // No program selected: "New Program" should be available
        let commands = filter_commands("", None, None, None, None, true);
        assert!(commands.iter().any(|c| c.label == "New Program"));

        // Program selected, no project: "New Project" should be available
        let commands = filter_commands("", Some("MyProgram"), None, None, None, true);
        assert!(commands.iter().any(|c| c.label == "New Project"));

        // Program and project selected, no milestone: "New Milestone" should be available
        let commands = filter_commands("", Some("MyProgram"), Some("MyProject"), None, None, true);
        assert!(commands.iter().any(|c| c.label == "New Milestone"));

        // Program, project, and milestone selected: "New Task" should be available
        let commands = filter_commands(
            "",
            Some("MyProgram"),
            Some("MyProject"),
            Some("MyMilestone"),
            None,
            true,
        );
        assert!(commands.iter().any(|c| c.label == "New Task"));

        let commands = filter_commands(
            "",
            Some("MyProgram"),
            Some("MyProject"),
            Some("MyMilestone"),
            Some("MyTask"),
            true,
        );
        assert!(commands.iter().any(|c| c.label == "New Subtask"));
    }

    #[test]
    fn test_filter_commands_new_program_always_available() {
        // Even with programs existing, "New Program" should still be available
        let commands = filter_commands("", Some("MyProgram"), None, None, None, false);
        assert!(commands.iter().any(|c| c.label == "New Program"));
    }

    #[test]
    fn test_filter_commands_empty_workspace() {
        // When workspace has no programs and nothing is selected, "New Program" should be available
        let commands = filter_commands("", None, None, None, None, false);
        assert!(commands.iter().any(|c| c.label == "New Program"));
        assert!(commands.iter().any(|c| c.label == "New Note Folder"));

        // Also verify that basic navigation commands are available
        assert!(commands.iter().any(|c| c.label == "Programs"));
        assert!(commands.iter().any(|c| c.label == "Journal"));
        assert!(commands.iter().any(|c| c.label == "Exit"));
    }
}
