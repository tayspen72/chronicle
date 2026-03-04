// TODO: Task is a domain model for future use in element modification features.
// Currently the TUI uses DirectoryEntry for all display, but this type will be used
// for structured data manipulation in upcoming sprints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Task domain model for structured task data.
/// Used by storage/md.rs parsing functions (planned for future element modification).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct Task {
    pub title: Option<String>,
    pub assignee: Option<String>,
    pub assigned_to: Option<String>,
    pub status: Option<String>,   // e.g., "todo|doing|done|blocked"
    pub priority: Option<String>, // e.g., "low|med|high|urgent"
    pub details: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub due: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[allow(dead_code)]
impl Task {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.status.as_deref(), Some("done"))
    }
}
