//! Planning session state management.
//!
//! This module contains the state types for managing planning sessions.

use crate::model::SelectedTask;

#[derive(Debug, Clone, Default)]
pub struct PlanningSessionState {
    pub active: bool,
    pub uuid: Option<String>,
    pub tasks: Vec<SelectedTask>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub rolled_over_tasks: Vec<String>,
}

impl PlanningSessionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.uuid = None;
        self.tasks.clear();
        self.start_date = None;
        self.due_date = None;
    }

    pub fn has_tasks(&self) -> bool {
        !self.tasks.is_empty()
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Check if a task path is in the selected tasks list.
    pub fn is_path_selected(&self, path: &str) -> bool {
        use std::path::Path;
        self.tasks
            .iter()
            .any(|t| t.path.as_path() == Path::new(path))
    }

    /// Check if a task UUID is in the selected tasks list.
    pub fn contains_uuid(&self, uuid: &str) -> bool {
        self.tasks.iter().any(|t| t.uuid == uuid)
    }

    /// Get the position of a task by UUID, if present.
    pub fn position_of_uuid(&self, uuid: &str) -> Option<usize> {
        self.tasks.iter().position(|t| t.uuid == uuid)
    }

    /// Get formatted date range as a tuple (start, end), using "?" for missing dates.
    pub fn date_range(&self) -> (String, String) {
        let start = self.start_date.as_deref().unwrap_or("?");
        let end = self.due_date.as_deref().unwrap_or("?");
        (start.to_string(), end.to_string())
    }

    /// Get formatted date range string "start → end".
    pub fn format_date_range(&self) -> String {
        let (start, end) = self.date_range();
        format!("{} → {}", start, end)
    }
}
