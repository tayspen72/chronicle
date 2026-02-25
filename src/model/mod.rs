use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Task {
    pub title: Option<String>,
    pub assignee: Option<String>,
    pub assigned_to: Option<String>,
    pub status: Option<String>, // e.g., "todo|doing|done|blocked"
    pub priority: Option<String>, // e.g., "low|med|high|urgent"
    pub details: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub due: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

impl Task {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.status.as_deref(), Some("done"))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}
