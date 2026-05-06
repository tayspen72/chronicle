use crossterm::event::KeyCode;

use crate::tui::command::{CommandAction, CommandMatch};

use super::super::{App, Mode};

impl App {
    pub(crate) fn handle_command_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.command_palette.input.push(c);
                self.command_palette.selection_index = 0;
                self.filter_commands();
            }
            KeyCode::Backspace => {
                self.command_palette.input.pop();
                self.command_palette.selection_index = 0;
                self.filter_commands();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_palette.input.clear();
                self.command_palette.selection_index = 0;
            }
            KeyCode::Enter => {
                if let Some(cmd) = self
                    .command_palette
                    .matches
                    .get(self.command_palette.selection_index)
                    .filter(|cmd| cmd.selectable)
                    .cloned()
                {
                    self.execute_command(&cmd);
                }
                // Only reset mode if we're not entering a special mode that should persist
                if !matches!(
                    self.mode,
                    Mode::TaskSelection
                        | Mode::ReviewSession
                        | Mode::HierarchicalSelection
                        | Mode::ThemeSelection
                ) {
                    self.mode = Mode::Normal;
                }
                self.command_palette.input.clear();
                self.command_palette.selection_index = 0;
            }
            KeyCode::Up => {
                if let Some(idx) =
                    self.prev_selectable_command_index(self.command_palette.selection_index)
                {
                    self.command_palette.selection_index = idx;
                }
            }
            KeyCode::Down => {
                if let Some(idx) =
                    self.next_selectable_command_index(self.command_palette.selection_index)
                {
                    self.command_palette.selection_index = idx;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn next_selectable_command_index(&self, from: usize) -> Option<usize> {
        self.command_palette
            .matches
            .iter()
            .enumerate()
            .skip(from.saturating_add(1))
            .find(|(_, cmd)| cmd.selectable)
            .map(|(idx, _)| idx)
    }

    pub(crate) fn prev_selectable_command_index(&self, from: usize) -> Option<usize> {
        if from == 0 {
            return None;
        }
        self.command_palette
            .matches
            .iter()
            .enumerate()
            .take(from)
            .rev()
            .find(|(_, cmd)| cmd.selectable)
            .map(|(idx, _)| idx)
    }

    pub(crate) fn execute_command(&mut self, cmd: &CommandMatch) {
        if cmd.exit {
            self.should_exit = true;
            return;
        }

        match &cmd.action {
            Some(CommandAction::OpenTodayJournal) => {
                self.open_today_journal();
            }
            Some(CommandAction::ShowArchiveList) => {
                self.show_archive_list();
            }
            Some(CommandAction::ShowProgramsList) => {
                self.show_programs_list();
            }
            Some(CommandAction::ShowProjectsList) => {
                self.show_projects_list();
            }
            Some(CommandAction::ShowMilestonesList) => {
                self.show_milestones_list();
            }
            Some(CommandAction::ShowTasksList) => {
                self.show_tasks_list();
            }
            Some(CommandAction::NewProgram) => {
                self.start_new_program();
            }
            Some(CommandAction::NewProject) => {
                self.start_new_project();
            }
            Some(CommandAction::NewMilestone) => {
                self.start_new_milestone();
            }
            Some(CommandAction::NewTask) => {
                self.start_new_task();
            }
            Some(CommandAction::NewSubtask) => {
                self.start_new_subtask();
            }
            Some(CommandAction::NewNote) => {
                self.start_new_note();
            }
            Some(CommandAction::NewNoteFolder) => {
                self.start_new_note_folder();
            }
            Some(CommandAction::MoveNote) => {
                self.start_move_note();
            }
            Some(CommandAction::Refresh) => {
                self.load_tree_view_data();
            }
            Some(CommandAction::StartPlanningSession) => {
                self.start_planning_session();
            }
            Some(CommandAction::ClosePlanningSession) => {
                self.close_planning_session();
            }
            Some(CommandAction::ReviewSession) => {
                self.start_review_session();
            }
            Some(CommandAction::SwitchTheme) => {
                self.start_theme_selection();
            }
            None => {
                self.current_view = cmd.view.clone();
            }
        }
    }

    pub(crate) fn filter_commands(&mut self) {
        self.command_palette.filter_with_context(
            self.navigation_state.current_program.as_deref(),
            self.navigation_state.current_project.as_deref(),
            self.navigation_state.current_milestone.as_deref(),
            self.navigation_state.current_task.as_deref(),
            !self.tree_data.programs.is_empty(),
        );
    }
}
