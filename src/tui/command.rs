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
#[allow(dead_code)]
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

/// Filters commands based on input and tree depth.
///
/// # Arguments
/// * `input` - The user's search input (lowercase)
/// * `depth` - Current tree navigation depth
///
/// # Returns
/// A filtered list of matching commands.
#[allow(dead_code)]
pub fn filter_commands(input: &str, depth: usize) -> Vec<CommandMatch> {
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

                if depth == 0 {
                    matches_input
                        && matches!(
                            cmd.label.as_str(),
                            "New Program"
                                | "New Project"
                                | "Programs"
                                | "Journal"
                                | "Backlog"
                                | "Weekly Planning"
                                | "Open Today's Journal"
                                | "Journal History"
                                | "Exit"
                        )
                } else if depth == 1 {
                    matches_input
                        && matches!(
                            cmd.label.as_str(),
                            "New Project"
                                | "Programs"
                                | "Projects"
                                | "Journal"
                                | "Backlog"
                                | "Weekly Planning"
                                | "Open Today's Journal"
                                | "Journal History"
                                | "Exit"
                        )
                } else if depth == 2 {
                    matches_input
                        && matches!(
                            cmd.label.as_str(),
                            "New Milestone"
                                | "Programs"
                                | "Projects"
                                | "Milestones"
                                | "Journal"
                                | "Backlog"
                                | "Weekly Planning"
                                | "Open Today's Journal"
                                | "Journal History"
                                | "Exit"
                        )
                } else {
                    matches_input
                        && matches!(
                            cmd.label.as_str(),
                            "New Task"
                                | "Programs"
                                | "Projects"
                                | "Milestones"
                                | "Tasks"
                                | "Journal"
                                | "Backlog"
                                | "Weekly Planning"
                                | "Open Today's Journal"
                                | "Journal History"
                                | "Exit"
                        )
                }
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
        let commands = filter_commands("", 0);
        // Should return context-aware commands for depth 0
        assert!(commands.iter().any(|c| c.label == "New Program"));
    }

    #[test]
    fn test_filter_commands_journal_prefix() {
        let commands = filter_commands("journal", 0);
        assert!(commands.iter().all(|c| c.label.contains("Journal")));
    }

    #[test]
    fn test_filter_commands_by_depth() {
        // Depth 0: should include "New Program"
        let commands = filter_commands("", 0);
        assert!(commands.iter().any(|c| c.label == "New Program"));

        // Depth 3: should include "New Task"
        let commands = filter_commands("", 3);
        assert!(commands.iter().any(|c| c.label == "New Task"));
    }
}
