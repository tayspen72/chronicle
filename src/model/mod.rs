//! Domain model for Chronicle elements.
//!
//! This module defines the core data structures for Programs, Projects,
//! Milestones, and Tasks, along with unified Element enum.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

/// Custom serde module for flexible date parsing.
/// Handles both RFC 3339 format (2026-03-13T12:00:00Z) and date-only format (2026-03-13).
pub mod date_serde {
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        // First, try to deserialize as a string
        let s = String::deserialize(deserializer)?;
        let trimmed = s.trim();

        // Try RFC 3339 format first
        if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try date-only format (YYYY-MM-DD)
        if let Ok(naive_date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap_or_default();
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
        }

        Err(D::Error::custom(format!(
            "invalid date format '{}', expected YYYY-MM-DD or RFC 3339",
            trimmed
        )))
    }

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as RFC 3339 format
        serializer.serialize_str(&date.to_rfc3339())
    }
}

/// Program - top-level container for projects.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Program {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
}

/// Project - belongs to a program, contains milestones.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Project {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default = "default_creation_date", with = "date_serde")]
    pub creation_date: DateTime<Utc>,
    pub created_by: Option<String>,
    pub assigned_to: Option<String>,
    pub due_date: Option<String>,
    #[serde(rename = "type")]
    pub element_type: Option<String>,
    #[serde(default)]
    pub description: String,
}

/// Milestone - belongs to a project, contains tasks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Milestone {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default = "default_creation_date", with = "date_serde")]
    pub creation_date: DateTime<Utc>,
    pub created_by: Option<String>,
    pub assigned_to: Option<String>,
    pub due_date: Option<String>,
    #[serde(rename = "type")]
    pub element_type: Option<String>,
    #[serde(default)]
    pub description: String,
}

/// Helper function to provide default creation date.
fn default_creation_date() -> DateTime<Utc> {
    Utc::now()
}

/// Task domain model for structured task data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Task {
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default = "default_creation_date", with = "date_serde")]
    pub creation_date: DateTime<Utc>,
    pub created_by: Option<String>,
    pub assigned_to: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub importance: Option<String>,
    #[serde(rename = "type")]
    pub element_type: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Task {
    /// Create a new task with the given title and default values.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            uuid: String::new(),
            title: title.into(),
            status: "todo".to_string(),
            creation_date: Utc::now(),
            created_by: None,
            assigned_to: None,
            start_date: None,
            due_date: None,
            importance: None,
            element_type: Some("task".to_string()),
            description: String::new(),
            tags: Vec::new(),
        }
    }

    /// Check if the task is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.status == "done"
    }
}

/// A task selected for inclusion in a planning session.
/// Stores context (program/project/milestone) for display in planning view.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelectedTask {
    pub uuid: String,
    pub path: PathBuf,
    pub program: String,
    pub project: String,
    pub milestone: String,
    pub task_name: String,
    pub status: String,
    pub assigned_to: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub importance: Option<String>,
    pub description: Option<String>,
}

/// Element kind enum for type identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementKind {
    Program,
    Project,
    Milestone,
    Task,
    Subtask,
}

impl std::fmt::Display for ElementKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementKind::Program => write!(f, "program"),
            ElementKind::Project => write!(f, "project"),
            ElementKind::Milestone => write!(f, "milestone"),
            ElementKind::Task => write!(f, "task"),
            ElementKind::Subtask => write!(f, "subtask"),
        }
    }
}

impl std::str::FromStr for ElementKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "program" => Ok(ElementKind::Program),
            "project" => Ok(ElementKind::Project),
            "milestone" => Ok(ElementKind::Milestone),
            "task" => Ok(ElementKind::Task),
            "subtask" => Ok(ElementKind::Subtask),
            _ => Err(format!("Unknown element kind: {}", s)),
        }
    }
}

/// Element enum that can hold any element type.
#[derive(Debug, Clone)]
pub enum Element {
    Program(Program),
    Project(Project),
    Milestone(Milestone),
    Task(Task),
}

impl Element {
    /// Get the kind of this element.
    #[must_use]
    pub fn kind(&self) -> ElementKind {
        match self {
            Element::Program(_) => ElementKind::Program,
            Element::Project(_) => ElementKind::Project,
            Element::Milestone(_) => ElementKind::Milestone,
            Element::Task(_) => ElementKind::Task,
        }
    }

    /// Get the title of this element.
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            Element::Program(p) => &p.title,
            Element::Project(p) => &p.title,
            Element::Milestone(m) => &m.title,
            Element::Task(t) => &t.title,
        }
    }

    /// Get the status of this element.
    #[must_use]
    pub fn status(&self) -> &str {
        match self {
            Element::Program(p) => &p.status,
            Element::Project(p) => &p.status,
            Element::Milestone(m) => &m.status,
            Element::Task(t) => &t.status,
        }
    }
}

/// Legacy Task struct for backward compatibility with parse_task.
/// This uses the old hashtag-style metadata format.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegacyTask {
    pub title: Option<String>,
    pub assignee: Option<String>,
    pub assigned_to: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub details: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub due: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

impl LegacyTask {
    /// Create a new legacy task with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the task is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        matches!(self.status.as_deref(), Some("done"))
    }
}

/// Status of a planning session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "archived")]
    Archived,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Planning session for tracking selected tasks over a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningSession {
    #[serde(rename = "type")]
    pub element_type: String,
    pub uuid: String,
    pub title: String,
    pub creation_date: String,
    pub created_by: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub duration: String,
    #[serde(default)]
    pub status: SessionStatus,
    pub tasks: Vec<String>,
}
