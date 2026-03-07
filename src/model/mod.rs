//! Domain model for Chronicle elements.
//!
//! This module defines the core data structures for Programs, Projects,
//! Milestones, and Tasks, along with unified Element enum.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    #[serde(default = "default_creation_date")]
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
    #[serde(default = "default_creation_date")]
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
    pub title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default = "default_creation_date")]
    pub creation_date: DateTime<Utc>,
    pub created_by: Option<String>,
    pub assigned_to: Option<String>,
    pub due_date: Option<String>,
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
            title: title.into(),
            status: "todo".to_string(),
            creation_date: Utc::now(),
            created_by: None,
            assigned_to: None,
            due_date: None,
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
