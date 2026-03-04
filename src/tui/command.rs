//! Command palette module.
//!
//! Handles command palette state, filtering, and execution.
//!
//! NOTE: This module contains extracted types and logic for the command palette.
//! The App struct in mod.rs still has inline implementations that duplicate this logic.
//! TODO: Wire up CommandPalette to replace inline command handling in App.

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
}

/// A matched command with its label, target view, and optional action.
#[derive(Debug, Clone)]
pub struct CommandMatch {
    pub label: String,
    pub view: ViewType,
    pub exit: bool,
    pub action: Option<CommandAction>,
}

/// State for the command palette.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CommandPalette {
    pub input: String,
    pub matches: Vec<CommandMatch>,
    pub selection_index: usize,
}

impl CommandPalette {
    /// Creates a new command palette with all commands loaded.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            input: String::new(),
            matches: get_command_list(),
            selection_index: 0,
        }
    }

    /// Handles keyboard input for the command palette.
    ///
    /// Returns `Some(CommandMatch)` when a command is executed,
    /// `None` otherwise.
    #[allow(dead_code)]
    pub fn handle_input(&mut self, code: KeyCode) -> Option<CommandMatch> {
        match code {
            KeyCode::Char(c) => {
                self.input.push(c);
                self.selection_index = 0;
                None
            }
            KeyCode::Backspace => {
                self.input.pop();
                self.selection_index = 0;
                None
            }
            KeyCode::Esc => {
                self.close();
                None
            }
            KeyCode::Enter => {
                let cmd = self.matches.get(self.selection_index).cloned();
                self.close();
                cmd
            }
            KeyCode::Up => {
                if self.selection_index > 0 {
                    self.selection_index -= 1;
                }
                None
            }
            KeyCode::Down => {
                if self.selection_index < self.matches.len().saturating_sub(1) {
                    self.selection_index += 1;
                }
                None
            }
            _ => None,
        }
    }

    /// Closes the command palette and resets its state.
    #[allow(dead_code)]
    pub fn close(&mut self) {
        self.input.clear();
        self.selection_index = 0;
    }

    /// Opens the command palette and resets the input.
    #[allow(dead_code)]
    pub fn open(&mut self) {
        self.input.clear();
        self.selection_index = 0;
        self.matches = get_command_list();
    }
}

/// Returns the list of all available commands.
pub fn get_command_list() -> Vec<CommandMatch> {
    vec![
        CommandMatch {
            label: "Programs".to_string(),
            view: ViewType::TreeView,
            exit: false,
            action: Some(CommandAction::ShowProgramsList),
        },
        CommandMatch {
            label: "Projects".to_string(),
            view: ViewType::TreeView,
            exit: false,
            action: Some(CommandAction::ShowProjectsList),
        },
        CommandMatch {
            label: "Milestones".to_string(),
            view: ViewType::TreeView,
            exit: false,
            action: Some(CommandAction::ShowMilestonesList),
        },
        CommandMatch {
            label: "Tasks".to_string(),
            view: ViewType::TreeView,
            exit: false,
            action: Some(CommandAction::ShowTasksList),
        },
        CommandMatch {
            label: "Journal".to_string(),
            view: ViewType::Journal,
            exit: false,
            action: None,
        },
        CommandMatch {
            label: "Backlog".to_string(),
            view: ViewType::Backlog,
            exit: false,
            action: None,
        },
        CommandMatch {
            label: "Weekly Planning".to_string(),
            view: ViewType::WeeklyPlanning,
            exit: false,
            action: None,
        },
        CommandMatch {
            label: "New Program".to_string(),
            view: ViewType::InputProgram,
            exit: false,
            action: Some(CommandAction::NewProgram),
        },
        CommandMatch {
            label: "New Project".to_string(),
            view: ViewType::InputProject,
            exit: false,
            action: Some(CommandAction::NewProject),
        },
        CommandMatch {
            label: "New Milestone".to_string(),
            view: ViewType::InputMilestone,
            exit: false,
            action: Some(CommandAction::NewMilestone),
        },
        CommandMatch {
            label: "New Task".to_string(),
            view: ViewType::InputTask,
            exit: false,
            action: Some(CommandAction::NewTask),
        },
        CommandMatch {
            label: "Open Today's Journal".to_string(),
            view: ViewType::Journal,
            exit: false,
            action: Some(CommandAction::OpenTodayJournal),
        },
        CommandMatch {
            label: "Journal History".to_string(),
            view: ViewType::Journal,
            exit: false,
            action: Some(CommandAction::ShowArchiveList),
        },
        CommandMatch {
            label: "Exit".to_string(),
            view: ViewType::Journal,
            exit: true,
            action: None,
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
    has_programs: bool,
) -> Vec<CommandMatch> {
    let input = input.to_lowercase();

    if input.starts_with("journal") || input.starts_with("/journal") {
        let remainder = input
            .trim_start_matches('/')
            .trim_start_matches("journal")
            .trim();

        let journal_commands = vec![
            CommandMatch {
                label: "Open Today's Journal".to_string(),
                view: ViewType::Journal,
                exit: false,
                action: Some(CommandAction::OpenTodayJournal),
            },
            CommandMatch {
                label: "Journal History".to_string(),
                view: ViewType::Journal,
                exit: false,
                action: Some(CommandAction::ShowArchiveList),
            },
        ];

        if remainder.is_empty() {
            journal_commands
        } else {
            journal_commands
                .into_iter()
                .filter(|cmd| cmd.label.to_lowercase().contains(remainder))
                .collect()
        }
    } else {
        let all_commands = get_command_list();
        all_commands
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
                    // Navigation commands - always available
                    "Programs"
                    | "Journal"
                    | "Backlog"
                    | "Weekly Planning"
                    | "Open Today's Journal"
                    | "Journal History"
                    | "Exit" => true,
                    // Tier-specific navigation - context-based
                    "Projects" => current_program.is_some() || has_programs,
                    "Milestones" => current_project.is_some(),
                    "Tasks" => current_milestone.is_some(),
                    _ => true,
                };

                matches_input && is_context_valid
            })
            .collect()
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
        assert_eq!(palette.selection_index, 0);
    }

    #[test]
    fn test_command_palette_char_input() {
        let mut palette = CommandPalette::new();
        let result = palette.handle_input(KeyCode::Char('a'));
        assert!(result.is_none());
        assert_eq!(palette.input, "a");
        assert_eq!(palette.selection_index, 0);
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
        assert_eq!(palette.selection_index, 0);

        // Navigate down
        palette.handle_input(KeyCode::Down);
        assert_eq!(palette.selection_index, 1);

        // Navigate up
        palette.handle_input(KeyCode::Up);
        assert_eq!(palette.selection_index, 0);

        // Can't go above 0
        palette.handle_input(KeyCode::Up);
        assert_eq!(palette.selection_index, 0);
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
        let commands = filter_commands("", None, None, None, true);
        // Should always include "New Program" when no programs exist
        assert!(commands.iter().any(|c| c.label == "New Program"));
    }

    #[test]
    fn test_filter_commands_journal_prefix() {
        let commands = filter_commands("journal", None, None, None, true);
        assert!(commands.iter().all(|c| c.label.contains("Journal")));
    }

    #[test]
    fn test_filter_commands_by_context() {
        // No program selected: "New Program" should be available
        let commands = filter_commands("", None, None, None, true);
        assert!(commands.iter().any(|c| c.label == "New Program"));

        // Program selected, no project: "New Project" should be available
        let commands = filter_commands("", Some("MyProgram"), None, None, true);
        assert!(commands.iter().any(|c| c.label == "New Project"));

        // Program and project selected, no milestone: "New Milestone" should be available
        let commands = filter_commands("", Some("MyProgram"), Some("MyProject"), None, true);
        assert!(commands.iter().any(|c| c.label == "New Milestone"));

        // Program, project, and milestone selected: "New Task" should be available
        let commands = filter_commands(
            "",
            Some("MyProgram"),
            Some("MyProject"),
            Some("MyMilestone"),
            true,
        );
        assert!(commands.iter().any(|c| c.label == "New Task"));
    }

    #[test]
    fn test_filter_commands_new_program_always_available() {
        // Even with programs existing, "New Program" should still be available
        let commands = filter_commands("", Some("MyProgram"), None, None, false);
        assert!(commands.iter().any(|c| c.label == "New Program"));
    }

    #[test]
    fn test_filter_commands_empty_workspace() {
        // When workspace has no programs and nothing is selected, "New Program" should be available
        let commands = filter_commands("", None, None, None, false);
        assert!(commands.iter().any(|c| c.label == "New Program"));

        // Also verify that basic navigation commands are available
        assert!(commands.iter().any(|c| c.label == "Programs"));
        assert!(commands.iter().any(|c| c.label == "Journal"));
        assert!(commands.iter().any(|c| c.label == "Exit"));
    }
}
