//! Planning wizard state management.
//!
//! This module groups all planning-related wizard state that was previously
//! scattered across the App struct. Following the pattern established by
//! HierarchicalPickerState.

use crate::tui::cache::TaskMetadata;

// Re-export TaskWizardState from task_wizard module for backwards compatibility
pub use crate::tui::task_wizard::TaskWizardState;

// Re-export TaskWizardField for use in this module
pub use crate::tui::task_wizard::TaskWizardField;

/// State for the planning wizard (date selection + task picker).
///
/// This replaces 10+ fields that were previously on App:
/// - planning_wizard_start_date
/// - planning_wizard_duration
/// - planning_wizard_end_date
/// - planning_wizard_focus
/// - planning_wizard_task_filter
/// - planning_wizard_selected_tasks
/// - planning_wizard_task_index
/// - planning_wizard_date_error
/// - planning_wizard_tasks
#[derive(Debug, Clone)]
pub struct PlanningWizardState {
    /// Start date for the planning session (YYYY-MM-DD format)
    pub start_date: String,

    /// Duration option (weekly, biweekly, 6weekly)
    pub duration: String,

    /// Custom end date override (empty = auto-calculate from duration)
    pub end_date: String,

    /// Currently focused field in the date wizard
    pub focus: PlanningDateFocus,

    /// Error message for date validation (if any)
    pub date_error: Option<String>,

    /// Filter text for task picker (legacy - may be deprecated)
    pub task_filter: String,

    /// Selected task UUIDs (legacy picker)
    pub selected_tasks: Vec<String>,

    /// Current index in task list (legacy picker)
    pub task_index: usize,

    /// All available tasks for selection
    pub tasks: Vec<TaskMetadata>,

    /// Input buffer for editing date fields
    pub input_buffer: String,
}

/// Focus states for the planning date wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanningDateFocus {
    /// Start date field
    #[default]
    StartDate,
    /// Duration selector
    Duration,
    /// End date field
    EndDate,
    /// Confirm button
    ConfirmButton,
    /// Cancel button
    CancelButton,
}

impl PlanningDateFocus {
    /// Navigate to the next field.
    pub fn next(self) -> Self {
        match self {
            PlanningDateFocus::StartDate => PlanningDateFocus::Duration,
            PlanningDateFocus::Duration => PlanningDateFocus::EndDate,
            PlanningDateFocus::EndDate => PlanningDateFocus::ConfirmButton,
            PlanningDateFocus::ConfirmButton => PlanningDateFocus::CancelButton,
            PlanningDateFocus::CancelButton => PlanningDateFocus::StartDate,
        }
    }

    /// Navigate to the previous field.
    pub fn prev(self) -> Self {
        match self {
            PlanningDateFocus::StartDate => PlanningDateFocus::CancelButton,
            PlanningDateFocus::Duration => PlanningDateFocus::StartDate,
            PlanningDateFocus::EndDate => PlanningDateFocus::Duration,
            PlanningDateFocus::ConfirmButton => PlanningDateFocus::EndDate,
            PlanningDateFocus::CancelButton => PlanningDateFocus::ConfirmButton,
        }
    }

    /// Convert to index for view rendering.
    pub fn index(self) -> usize {
        match self {
            PlanningDateFocus::StartDate => 0,
            PlanningDateFocus::Duration => 1,
            PlanningDateFocus::EndDate => 2,
            PlanningDateFocus::ConfirmButton => 3,
            PlanningDateFocus::CancelButton => 4,
        }
    }

    /// Check if this focus is a date field (start or end).
    pub fn is_date_field(self) -> bool {
        matches!(
            self,
            PlanningDateFocus::StartDate | PlanningDateFocus::EndDate
        )
    }

    /// Check if this focus is a button.
    pub fn is_button(self) -> bool {
        matches!(
            self,
            PlanningDateFocus::ConfirmButton | PlanningDateFocus::CancelButton
        )
    }
}

impl PlanningWizardState {
    /// Create new wizard state with today's date as default.
    pub fn new(default_duration: &str) -> Self {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        Self {
            start_date: today.clone(),
            duration: default_duration.to_string(),
            end_date: String::new(),
            focus: PlanningDateFocus::default(),
            date_error: None,
            task_filter: String::new(),
            selected_tasks: Vec::new(),
            task_index: 0,
            tasks: Vec::new(),
            input_buffer: today,
        }
    }

    /// Calculate the suggested end date based on start date + duration.
    pub fn calculate_suggested_end_date(&self) -> String {
        let days = match self.duration.as_str() {
            "biweekly" => 14,
            "6weekly" => 42,
            _ => 7,
        };

        let start_date_str = if self.start_date.is_empty() {
            chrono::Local::now().format("%Y-%m-%d").to_string()
        } else {
            self.start_date.clone()
        };

        chrono::NaiveDate::parse_from_str(&start_date_str, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.checked_add_days(chrono::Days::new(days)))
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or(start_date_str)
    }

    /// Validate dates and return error message if invalid.
    pub fn validate_dates(&self) -> Option<String> {
        if chrono::NaiveDate::parse_from_str(&self.start_date, "%Y-%m-%d").is_err() {
            return Some("Invalid start date format. Use YYYY-MM-DD".to_string());
        }

        if !self.end_date.is_empty() {
            if chrono::NaiveDate::parse_from_str(&self.end_date, "%Y-%m-%d").is_err() {
                return Some("Invalid end date format. Use YYYY-MM-DD".to_string());
            }

            let start = chrono::NaiveDate::parse_from_str(&self.start_date, "%Y-%m-%d").ok();
            let end = chrono::NaiveDate::parse_from_str(&self.end_date, "%Y-%m-%d").ok();

            if let (Some(s), Some(e)) = (start, end) {
                if e < s {
                    return Some("End date must be on or after start date".to_string());
                }
            }
        }

        None
    }

    /// Get the effective end date (custom or calculated from duration).
    pub fn effective_end_date(&self) -> String {
        if !self.end_date.is_empty() {
            self.end_date.clone()
        } else {
            self.calculate_suggested_end_date()
        }
    }

    /// Cycle duration left (-1) or right (+1).
    pub fn cycle_duration(&mut self, delta: isize) {
        const DURATIONS: &[&str] = &["weekly", "biweekly", "6weekly"];
        let current_idx = DURATIONS
            .iter()
            .position(|&d| d == self.duration)
            .unwrap_or(0);
        let len = DURATIONS.len() as isize;
        let new_idx = ((current_idx as isize + delta).rem_euclid(len)) as usize;
        self.duration = DURATIONS[new_idx].to_string();
    }

    /// Reset to initial state.
    pub fn reset(&mut self, default_duration: &str) {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.start_date = today.clone();
        self.duration = default_duration.to_string();
        self.end_date.clear();
        self.focus = PlanningDateFocus::default();
        self.date_error = None;
        self.task_filter.clear();
        self.selected_tasks.clear();
        self.task_index = 0;
        self.tasks.clear();
        self.input_buffer = today;
    }

    /// Sync input buffer to the currently focused date field.
    pub fn sync_input_to_field(&mut self) {
        if self.focus == PlanningDateFocus::StartDate {
            self.start_date = self.input_buffer.clone();
        } else if self.focus == PlanningDateFocus::EndDate {
            self.end_date = self.input_buffer.clone();
        }
    }

    /// Sync currently focused date field to input buffer.
    pub fn sync_field_to_input(&mut self) {
        if self.focus == PlanningDateFocus::StartDate {
            self.input_buffer = if self.start_date.is_empty() {
                chrono::Local::now().format("%Y-%m-%d").to_string()
            } else {
                self.start_date.clone()
            };
        } else if self.focus == PlanningDateFocus::EndDate {
            self.input_buffer = if self.end_date.is_empty() {
                self.calculate_suggested_end_date()
            } else {
                self.end_date.clone()
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_state_calculates_end_date_weekly() {
        let mut state = PlanningWizardState::new("weekly");
        state.start_date = "2024-01-01".to_string();
        assert_eq!(state.calculate_suggested_end_date(), "2024-01-08");
    }

    #[test]
    fn test_wizard_state_calculates_end_date_biweekly() {
        let mut state = PlanningWizardState::new("biweekly");
        state.start_date = "2024-01-01".to_string();
        assert_eq!(state.calculate_suggested_end_date(), "2024-01-15");
    }

    #[test]
    fn test_wizard_state_calculates_end_date_6weekly() {
        let mut state = PlanningWizardState::new("6weekly");
        state.start_date = "2024-01-01".to_string();
        assert_eq!(state.calculate_suggested_end_date(), "2024-02-12");
    }

    #[test]
    fn test_wizard_state_cycles_duration_right() {
        let mut state = PlanningWizardState::new("weekly");
        state.cycle_duration(1);
        assert_eq!(state.duration, "biweekly");
        state.cycle_duration(1);
        assert_eq!(state.duration, "6weekly");
        state.cycle_duration(1);
        assert_eq!(state.duration, "weekly"); // wraps
    }

    #[test]
    fn test_wizard_state_cycles_duration_left() {
        let mut state = PlanningWizardState::new("weekly");
        state.cycle_duration(-1);
        assert_eq!(state.duration, "6weekly"); // wraps backward
        state.cycle_duration(-1);
        assert_eq!(state.duration, "biweekly");
        state.cycle_duration(-1);
        assert_eq!(state.duration, "weekly");
    }

    #[test]
    fn test_wizard_state_validates_start_date_format() {
        let mut state = PlanningWizardState::new("weekly");
        state.start_date = "invalid".to_string();
        assert!(state.validate_dates().is_some());
    }

    #[test]
    fn test_wizard_state_validates_end_date_after_start() {
        let mut state = PlanningWizardState::new("weekly");
        state.start_date = "2024-01-15".to_string();
        state.end_date = "2024-01-10".to_string();
        let error = state.validate_dates();
        assert!(error.is_some());
        assert!(error.unwrap().contains("on or after"));
    }

    #[test]
    fn test_wizard_state_allows_empty_end_date() {
        let mut state = PlanningWizardState::new("weekly");
        state.start_date = "2024-01-01".to_string();
        state.end_date = String::new();
        assert!(state.validate_dates().is_none());
    }

    #[test]
    fn test_planning_date_focus_next() {
        assert_eq!(
            PlanningDateFocus::StartDate.next(),
            PlanningDateFocus::Duration
        );
        assert_eq!(
            PlanningDateFocus::Duration.next(),
            PlanningDateFocus::EndDate
        );
        assert_eq!(
            PlanningDateFocus::EndDate.next(),
            PlanningDateFocus::ConfirmButton
        );
        assert_eq!(
            PlanningDateFocus::ConfirmButton.next(),
            PlanningDateFocus::CancelButton
        );
        assert_eq!(
            PlanningDateFocus::CancelButton.next(),
            PlanningDateFocus::StartDate
        );
    }

    #[test]
    fn test_planning_date_focus_prev() {
        assert_eq!(
            PlanningDateFocus::StartDate.prev(),
            PlanningDateFocus::CancelButton
        );
        assert_eq!(
            PlanningDateFocus::Duration.prev(),
            PlanningDateFocus::StartDate
        );
        assert_eq!(
            PlanningDateFocus::EndDate.prev(),
            PlanningDateFocus::Duration
        );
        assert_eq!(
            PlanningDateFocus::ConfirmButton.prev(),
            PlanningDateFocus::EndDate
        );
        assert_eq!(
            PlanningDateFocus::CancelButton.prev(),
            PlanningDateFocus::ConfirmButton
        );
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
}
