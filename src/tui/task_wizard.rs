//! Task detail wizard module.
//!
//! This module contains the state and helper methods for the task detail wizard
//! that allows users to edit task properties before adding them to a planning session.

use crate::model::SelectedTask;

/// Field indices for the task detail wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskWizardField {
    TaskName = 0,
    Status = 1,
    AssignedTo = 2,
    StartDate = 3,
    DueDate = 4,
    Priority = 5,
    Description = 6,
    AddToPlanButton = 7,
    CancelButton = 8,
}

impl TaskWizardField {
    /// Convert an index to a TaskWizardField enum.
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(TaskWizardField::TaskName),
            1 => Some(TaskWizardField::Status),
            2 => Some(TaskWizardField::AssignedTo),
            3 => Some(TaskWizardField::StartDate),
            4 => Some(TaskWizardField::DueDate),
            5 => Some(TaskWizardField::Priority),
            6 => Some(TaskWizardField::Description),
            7 => Some(TaskWizardField::AddToPlanButton),
            8 => Some(TaskWizardField::CancelButton),
            _ => None,
        }
    }

    /// Get the total number of fields (excluding buttons).
    pub const fn editable_field_count() -> usize {
        6 // task name, status, assigned_to, start_date, due_date, priority
    }

    /// Check if this field accepts text input.
    pub fn accepts_text_input(&self) -> bool {
        matches!(
            self,
            TaskWizardField::AssignedTo
                | TaskWizardField::StartDate
                | TaskWizardField::DueDate
                | TaskWizardField::Description
        )
    }
}

/// State for the task detail wizard.
#[derive(Debug, Clone, Default)]
pub struct TaskWizardState {
    /// The task being edited.
    pub task: Option<SelectedTask>,
    /// Index of currently focused field.
    pub field_index: usize,
    /// Input buffer for text fields.
    pub input_buffer: String,
}

impl TaskWizardState {
    /// Create new empty task wizard state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create state with an existing task.
    pub fn with_task(task: SelectedTask) -> Self {
        Self {
            task: Some(task),
            field_index: 0,
            input_buffer: String::new(),
        }
    }

    /// Reset to empty state.
    pub fn reset(&mut self) {
        self.task = None;
        self.field_index = 0;
        self.input_buffer.clear();
    }

    /// Get current field as TaskWizardField enum.
    pub fn current_field(&self) -> Option<TaskWizardField> {
        TaskWizardField::from_index(self.field_index)
    }

    /// Check if wizard is active (has a task).
    pub fn is_active(&self) -> bool {
        self.task.is_some()
    }

    /// Total number of fields including buttons (7 fields + 2 buttons).
    const TOTAL_FIELDS: usize = 9;

    /// Navigate to next field.
    pub fn next_field(&mut self) {
        self.field_index = (self.field_index + 1) % Self::TOTAL_FIELDS;
    }

    /// Navigate to previous field.
    pub fn prev_field(&mut self) {
        self.field_index = (self.field_index + Self::TOTAL_FIELDS - 1) % Self::TOTAL_FIELDS;
    }

    /// Check if current field is editable text field.
    pub fn current_field_is_editable(&self) -> bool {
        matches!(
            self.current_field(),
            Some(TaskWizardField::AssignedTo)
                | Some(TaskWizardField::StartDate)
                | Some(TaskWizardField::DueDate)
                | Some(TaskWizardField::Description)
        )
    }
}

/// Cycle to the next field or cycle field values based on current field.
///
/// For text fields (assigned_to, start_date, due_date), this moves to the next field.
/// For status, this cycles through workflow values.
/// For priority, this cycles through priority levels.
pub fn cycle_task_wizard_field_or_next(wizard: &mut TaskWizardState, workflow: &[String]) {
    if let Some(ref task) = wizard.task {
        match wizard.field_index {
            0 => {
                // Task name is not editable - move to next field
                wizard.next_field();
            }
            1 => {
                // Cycle status through workflow values
                let current_idx = workflow.iter().position(|s| s == &task.status).unwrap_or(0);
                let next_idx = (current_idx + 1) % workflow.len();
                if let Some(ref mut t) = wizard.task {
                    t.status = workflow[next_idx].clone();
                }
            }
            2..=4 => {
                // Text fields - move to next field
                wizard.next_field();
            }
            5 => {
                // Cycle priority, then move to next field
                let priorities = ["low", "medium", "high"];
                let current = task.priority.clone().unwrap_or_default();
                let current_idx = priorities
                    .iter()
                    .position(|&p| p == current.as_str())
                    .unwrap_or(0);
                let next_idx = (current_idx + 1) % priorities.len();
                if let Some(ref mut t) = wizard.task {
                    t.priority = Some(priorities[next_idx].to_string());
                }
                wizard.next_field();
            }
            6 => {
                // Description - move to ADD TO PLAN button
                wizard.next_field();
            }
            _ => {}
        }
    }
}

/// Handle character input for text fields in the task wizard.
///
/// This is called when the user types a character while in the task wizard.
/// Only editable text fields (assigned_to, start_date, due_date) accept input.
pub fn handle_task_wizard_char(wizard: &mut TaskWizardState, c: char) {
    if let Some(ref mut task) = wizard.task {
        match wizard.field_index {
            2 => {
                // Assigned to
                let mut value = task.assigned_to.clone().unwrap_or_default();
                value.push(c);
                task.assigned_to = Some(value);
            }
            3 => {
                // Start date
                let mut value = task.start_date.clone().unwrap_or_default();
                value.push(c);
                task.start_date = Some(value);
            }
            4 => {
                // Due date
                let mut value = task.due_date.clone().unwrap_or_default();
                value.push(c);
                task.due_date = Some(value);
            }
            6 => {
                // Description
                let mut value = task.description.clone().unwrap_or_default();
                value.push(c);
                task.description = Some(value);
            }
            _ => {
                // Other fields don't accept text input (task name, status, priority cycle instead)
            }
        }
    }
}

/// Handle backspace for text fields in the task wizard.
///
/// This is called when the user presses backspace while in the task wizard.
pub fn handle_task_wizard_backspace(wizard: &mut TaskWizardState) {
    if let Some(ref mut task) = wizard.task {
        match wizard.field_index {
            2 => {
                // Assigned to
                let mut value = task.assigned_to.clone().unwrap_or_default();
                value.pop();
                task.assigned_to = if value.is_empty() { None } else { Some(value) };
            }
            3 => {
                // Start date
                let mut value = task.start_date.clone().unwrap_or_default();
                value.pop();
                task.start_date = if value.is_empty() { None } else { Some(value) };
            }
            4 => {
                // Due date
                let mut value = task.due_date.clone().unwrap_or_default();
                value.pop();
                task.due_date = if value.is_empty() { None } else { Some(value) };
            }
            6 => {
                // Description
                let mut value = task.description.clone().unwrap_or_default();
                value.pop();
                task.description = if value.is_empty() { None } else { Some(value) };
            }
            _ => {
                // Other fields don't accept text input
            }
        }
    }
}

/// Confirm the task detail wizard and add the task to the planning session.
///
/// Returns the task if one was being edited, or None if the wizard was empty.
pub fn confirm_task_detail_wizard(wizard: &mut TaskWizardState) -> Option<SelectedTask> {
    wizard.task.take()
}

/// Cancel the task detail wizard and clear the state.
pub fn cancel_task_detail_wizard(wizard: &mut TaskWizardState) {
    wizard.reset();
}

/// Confirm field input and update the task with the input buffer value.
///
/// This is called when exiting input mode for a text field.
pub fn confirm_task_detail_field_input(wizard: &mut TaskWizardState, input_buffer: &str) {
    if let Some(ref mut task) = wizard.task {
        let value = if input_buffer.is_empty() {
            None
        } else {
            Some(input_buffer.to_string())
        };
        match wizard.field_index {
            2 => task.assigned_to = value,
            3 => task.start_date = value,
            4 => task.due_date = value,
            6 => task.description = value,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_wizard_field_from_index() {
        assert_eq!(
            TaskWizardField::from_index(0),
            Some(TaskWizardField::TaskName)
        );
        assert_eq!(
            TaskWizardField::from_index(1),
            Some(TaskWizardField::Status)
        );
        assert_eq!(
            TaskWizardField::from_index(8),
            Some(TaskWizardField::CancelButton)
        );
        assert_eq!(TaskWizardField::from_index(9), None);
    }

    #[test]
    fn test_task_wizard_field_accepts_text_input() {
        assert!(!TaskWizardField::TaskName.accepts_text_input());
        assert!(!TaskWizardField::Status.accepts_text_input());
        assert!(TaskWizardField::AssignedTo.accepts_text_input());
        assert!(TaskWizardField::StartDate.accepts_text_input());
        assert!(TaskWizardField::DueDate.accepts_text_input());
        assert!(!TaskWizardField::Priority.accepts_text_input());
        assert!(TaskWizardField::Description.accepts_text_input());
    }

    #[test]
    fn test_task_wizard_state_navigation() {
        let mut state = TaskWizardState::new();
        assert_eq!(state.field_index, 0);

        state.next_field();
        assert_eq!(state.field_index, 1);

        state.prev_field();
        assert_eq!(state.field_index, 0);
    }

    #[test]
    fn test_task_wizard_state_wraps() {
        let mut state = TaskWizardState::new();
        // Navigate to last field (index 8)
        for _ in 0..8 {
            state.next_field();
        }
        assert_eq!(state.field_index, 8);

        // Should wrap to 0
        state.next_field();
        assert_eq!(state.field_index, 0);

        // Should wrap back to 8
        state.prev_field();
        assert_eq!(state.field_index, 8);
    }

    #[test]
    fn test_cycle_task_wizard_field_or_next_moves_through_fields() {
        let mut state = TaskWizardState::with_task(SelectedTask {
            uuid: "test".to_string(),
            path: std::path::PathBuf::from("/test.md"),
            program: "Test".to_string(),
            project: "Test".to_string(),
            milestone: "Test".to_string(),
            task_name: "Test Task".to_string(),
            status: "todo".to_string(),
            assigned_to: None,
            start_date: None,
            due_date: None,
            priority: None,
            description: None,
        });
        let workflow = vec![
            "todo".to_string(),
            "in_progress".to_string(),
            "done".to_string(),
        ];

        // Field 0 (TaskName) -> moves to field 1
        cycle_task_wizard_field_or_next(&mut state, &workflow);
        assert_eq!(state.field_index, 1);

        // Field 1 (Status) -> cycles status, stays on field 1
        cycle_task_wizard_field_or_next(&mut state, &workflow);
        assert_eq!(state.task.as_ref().unwrap().status, "in_progress");
        assert_eq!(state.field_index, 1);

        // Field 2 (AssignedTo) -> moves to field 3
        state.field_index = 2;
        cycle_task_wizard_field_or_next(&mut state, &workflow);
        assert_eq!(state.field_index, 3);

        // Field 5 (Priority) -> cycles priority
        state.field_index = 5;
        cycle_task_wizard_field_or_next(&mut state, &workflow);
        assert_eq!(
            state.task.as_ref().unwrap().priority,
            Some("medium".to_string())
        );
    }

    #[test]
    fn test_handle_task_wizard_char() {
        let mut state = TaskWizardState::with_task(SelectedTask {
            uuid: "test".to_string(),
            path: std::path::PathBuf::from("/test.md"),
            program: "Test".to_string(),
            project: "Test".to_string(),
            milestone: "Test".to_string(),
            task_name: "Test Task".to_string(),
            status: "todo".to_string(),
            assigned_to: None,
            start_date: None,
            due_date: None,
            priority: None,
            description: None,
        });

        // Field 2 = AssignedTo
        state.field_index = 2;
        handle_task_wizard_char(&mut state, 'J');
        handle_task_wizard_char(&mut state, 'o');
        handle_task_wizard_char(&mut state, 'h');
        handle_task_wizard_char(&mut state, 'n');
        assert_eq!(
            state.task.as_ref().unwrap().assigned_to,
            Some("John".to_string())
        );

        // Field 3 = StartDate
        state.field_index = 3;
        handle_task_wizard_char(&mut state, '2');
        handle_task_wizard_char(&mut state, '0');
        handle_task_wizard_char(&mut state, '2');
        handle_task_wizard_char(&mut state, '4');
        assert_eq!(
            state.task.as_ref().unwrap().start_date,
            Some("2024".to_string())
        );
    }

    #[test]
    fn test_handle_task_wizard_backspace() {
        let mut state = TaskWizardState::with_task(SelectedTask {
            uuid: "test".to_string(),
            path: std::path::PathBuf::from("/test.md"),
            program: "Test".to_string(),
            project: "Test".to_string(),
            milestone: "Test".to_string(),
            task_name: "Test Task".to_string(),
            status: "todo".to_string(),
            assigned_to: Some("John".to_string()),
            start_date: Some("2024-01-15".to_string()),
            due_date: None,
            priority: None,
            description: None,
        });

        // Field 2 = AssignedTo
        state.field_index = 2;
        handle_task_wizard_backspace(&mut state);
        assert_eq!(
            state.task.as_ref().unwrap().assigned_to,
            Some("Joh".to_string())
        );
        handle_task_wizard_backspace(&mut state);
        handle_task_wizard_backspace(&mut state);
        handle_task_wizard_backspace(&mut state);
        // Should be None now (empty string becomes None)
        assert_eq!(state.task.as_ref().unwrap().assigned_to, None);
    }

    #[test]
    fn test_confirm_task_detail_wizard_returns_task() {
        let mut state = TaskWizardState::with_task(SelectedTask {
            uuid: "test".to_string(),
            path: std::path::PathBuf::from("/test.md"),
            program: "Test".to_string(),
            project: "Test".to_string(),
            milestone: "Test".to_string(),
            task_name: "Test Task".to_string(),
            status: "todo".to_string(),
            assigned_to: None,
            start_date: None,
            due_date: None,
            priority: None,
            description: None,
        });

        let task = confirm_task_detail_wizard(&mut state);
        assert!(task.is_some());
        assert!(state.task.is_none());
    }

    #[test]
    fn test_cancel_task_detail_wizard_resets_state() {
        let mut state = TaskWizardState::with_task(SelectedTask {
            uuid: "test".to_string(),
            path: std::path::PathBuf::from("/test.md"),
            program: "Test".to_string(),
            project: "Test".to_string(),
            milestone: "Test".to_string(),
            task_name: "Test Task".to_string(),
            status: "todo".to_string(),
            assigned_to: Some("John".to_string()),
            start_date: None,
            due_date: None,
            priority: None,
            description: None,
        });

        state.field_index = 5;
        cancel_task_detail_wizard(&mut state);
        assert!(state.task.is_none());
        assert_eq!(state.field_index, 0);
    }

    #[test]
    fn test_confirm_task_detail_field_input() {
        let mut state = TaskWizardState::with_task(SelectedTask {
            uuid: "test".to_string(),
            path: std::path::PathBuf::from("/test.md"),
            program: "Test".to_string(),
            project: "Test".to_string(),
            milestone: "Test".to_string(),
            task_name: "Test Task".to_string(),
            status: "todo".to_string(),
            assigned_to: None,
            start_date: None,
            due_date: None,
            priority: None,
            description: None,
        });

        // Field 2 = AssignedTo
        state.field_index = 2;
        confirm_task_detail_field_input(&mut state, "Jane");
        assert_eq!(
            state.task.as_ref().unwrap().assigned_to,
            Some("Jane".to_string())
        );

        // Field 3 = StartDate
        state.field_index = 3;
        confirm_task_detail_field_input(&mut state, "2024-02-01");
        assert_eq!(
            state.task.as_ref().unwrap().start_date,
            Some("2024-02-01".to_string())
        );

        // Empty input sets None
        state.field_index = 2;
        confirm_task_detail_field_input(&mut state, "");
        assert_eq!(state.task.as_ref().unwrap().assigned_to, None);
    }
}
