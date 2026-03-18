//! Review state management.
//!
//! This module contains the review state types used for reviewing
//! tasks in planning sessions.

/// State for reviewing tasks in a planning session.
#[derive(Debug, Clone, Default)]
pub struct ReviewState {
    /// Current selection index in the review list
    pub selection_index: usize,
    /// Focus state for review session input (100=assigned_to, 101=start_date, 102=due_date)
    pub input_focus: Option<usize>,
    /// Focus index for the planning preview (0=task list, 1=start date, 2=due date)
    pub preview_focus: usize,
}

impl ReviewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.selection_index = 0;
        self.input_focus = None;
        self.preview_focus = 0;
    }
}
