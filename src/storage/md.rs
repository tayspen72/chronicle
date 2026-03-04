//! Markdown parsing utilities.
//!
//! These functions parse element metadata from markdown files with YAML frontmatter.
//! They are used by tests but not yet wired into the TUI.
//! TODO: Wire up parse_element and element_to_markdown for element modification features.

use crate::error::{ModelError, Result};
use crate::model::{Element, LegacyTask, Milestone, Program, Project, Task};
use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;

/// Parse an element from markdown content with YAML frontmatter.
///
/// Format:
/// ```markdown
/// ---
/// title: My Title
/// status: todo
/// creation_date: 2026-03-04T12:00:00Z
/// ---
///
/// # Description
/// The element description goes here...
/// ```
///
/// Returns `Ok(None)` if no YAML frontmatter is found.
/// Returns `Err` if frontmatter is present but malformed.
#[allow(dead_code)]
pub fn parse_element(content: &str) -> Result<Option<Element>> {
    // Extract YAML frontmatter between --- delimiters
    let frontmatter_regex = Regex::new(r"^---\s*\n([\s\S]*?)\n---\s*\n")
        .map_err(|e| ModelError::Parse(format!("Failed to compile frontmatter regex: {}", e)))?;

    let Some(captures) = frontmatter_regex.captures(content) else {
        return Ok(None);
    };

    let frontmatter = captures.get(1).map_or("", |m| m.as_str());
    let body = &content[captures[0].len()..];

    // Determine element type from the frontmatter
    // First, parse as a generic YAML value to extract the type
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(frontmatter)?;
    let element_type = yaml_value
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("task");

    // Parse based on element type
    let element = match element_type {
        "program" => {
            let mut program: Program = serde_yaml::from_str(frontmatter)?;
            program.description = body.trim().to_string();
            Element::Program(program)
        }
        "project" => {
            let mut project: Project = serde_yaml::from_str(frontmatter)?;
            project.description = body.trim().to_string();
            Element::Project(project)
        }
        "milestone" => {
            let mut milestone: Milestone = serde_yaml::from_str(frontmatter)?;
            milestone.description = body.trim().to_string();
            Element::Milestone(milestone)
        }
        "task" | "subtask" => {
            let mut task: Task = serde_yaml::from_str(frontmatter)?;
            task.description = body.trim().to_string();
            Element::Task(task)
        }
        _ => {
            return Err(
                ModelError::Parse(format!("Unknown element type: {}", element_type)).into(),
            );
        }
    };

    Ok(Some(element))
}

/// Parse legacy task metadata from markdown content (hashtag-style format).
///
/// Format:
/// ```markdown
/// #title: Implement feature
/// #assignee: John
/// #assigned-to: john
/// #status: todo
/// #priority: high
/// #due: 2026-03-01
/// #tags: #task #backend #project:auth
///
/// ## Details
/// The task description goes here...
/// ```
#[allow(dead_code)]
pub fn parse_task(content: &str) -> Result<LegacyTask> {
    let mut task = LegacyTask::new();
    let mut details_lines = Vec::new();
    let mut in_metadata = true;

    for line in content.lines() {
        let trimmed = line.trim();

        // Check if we've left the metadata section (empty lines don't exit metadata mode)
        if in_metadata && !trimmed.is_empty() && !trimmed.starts_with('#') {
            in_metadata = false;
        }

        if in_metadata {
            // Parse hashtag-style metadata
            if let Some(rest) = trimmed.strip_prefix("#title:") {
                task.title = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("#assignee:") {
                task.assignee = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("#assigned-to:") {
                task.assigned_to = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("#status:") {
                let status = rest.trim().to_lowercase();
                // Validate status values
                if matches!(status.as_str(), "todo" | "doing" | "done" | "blocked") {
                    task.status = Some(status);
                }
            } else if let Some(rest) = trimmed.strip_prefix("#priority:") {
                let priority = rest.trim().to_lowercase();
                // Validate priority values
                if matches!(priority.as_str(), "low" | "med" | "high" | "urgent") {
                    task.priority = Some(priority);
                }
            } else if let Some(rest) = trimmed.strip_prefix("#due:") {
                if let Ok(date) = parse_date(rest.trim()) {
                    task.due = Some(date);
                }
            } else if let Some(rest) = trimmed.strip_prefix("#tags:") {
                task.tags = parse_tags(rest.trim());
            }
        } else {
            // Collect the rest as details
            details_lines.push(line);
        }
    }

    // Join details, trimming leading/trailing empty lines
    let details = details_lines
        .iter()
        .skip_while(|l| l.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end_matches('\n')
        .to_string();

    task.details = if details.is_empty() {
        None
    } else {
        Some(details)
    };

    Ok(task)
}

/// Parse tags from a space-separated string like "#task #backend #project:auth"
#[allow(dead_code)]
fn parse_tags(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| {
            // Preserve the # if present, or add it for consistency
            if s.starts_with('#') {
                s.to_string()
            } else {
                format!("#{}", s)
            }
        })
        .collect()
}

/// Parse a date string into a DateTime<Utc>
///
/// Supports formats:
/// - YYYY-MM-DD
/// - YYYY-MM-DDTHH:MM:SSZ
#[allow(dead_code)]
fn parse_date(input: &str) -> Result<DateTime<Utc>> {
    let trimmed = input.trim();

    // Try YYYY-MM-DD first
    if let Ok(naive_date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap_or_default();
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
    }

    // Try ISO 8601 format
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&Utc));
    }

    Err(ModelError::Parse(format!("Could not parse date: '{}'", input)).into())
}

/// Generate markdown content from a LegacyTask struct (hashtag-style format).
#[allow(dead_code)]
pub fn task_to_markdown(task: &LegacyTask) -> String {
    let mut lines = Vec::new();

    if let Some(title) = &task.title {
        lines.push(format!("#title: {}", title));
    }
    if let Some(assignee) = &task.assignee {
        lines.push(format!("#assignee: {}", assignee));
    }
    if let Some(assigned_to) = &task.assigned_to {
        lines.push(format!("#assigned-to: {}", assigned_to));
    }
    if let Some(status) = &task.status {
        lines.push(format!("#status: {}", status));
    }
    if let Some(priority) = &task.priority {
        lines.push(format!("#priority: {}", priority));
    }
    if let Some(due) = &task.due {
        lines.push(format!("#due: {}", due.format("%Y-%m-%d")));
    }
    if !task.tags.is_empty() {
        lines.push(format!("#tags: {}", task.tags.join(" ")));
    }

    // Empty line separator before details
    lines.push(String::new());

    if let Some(details) = &task.details {
        lines.push(details.clone());
    }

    lines.join("\n")
}

/// Generate markdown content with YAML frontmatter from an Element.
#[allow(dead_code)]
pub fn element_to_markdown(element: &Element) -> String {
    match element {
        Element::Program(program) => {
            let frontmatter = serde_yaml::to_string(program).unwrap_or_default();
            format!(
                "---\n{}\n---\n\n{}",
                frontmatter.trim_end(),
                program.description
            )
        }
        Element::Project(project) => {
            let frontmatter = serde_yaml::to_string(project).unwrap_or_default();
            format!(
                "---\n{}\n---\n\n{}",
                frontmatter.trim_end(),
                project.description
            )
        }
        Element::Milestone(milestone) => {
            let frontmatter = serde_yaml::to_string(milestone).unwrap_or_default();
            format!(
                "---\n{}\n---\n\n{}",
                frontmatter.trim_end(),
                milestone.description
            )
        }
        Element::Task(task) => {
            let frontmatter = serde_yaml::to_string(task).unwrap_or_default();
            format!(
                "---\n{}\n---\n\n{}",
                frontmatter.trim_end(),
                task.description
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ElementKind;

    #[test]
    fn test_parse_basic_task() {
        let content = r#"
#title: Implement feature
#status: todo
#priority: high

## Details
This is the task description.
"#;
        let task = parse_task(content).unwrap();
        assert_eq!(task.title, Some("Implement feature".to_string()));
        assert_eq!(task.status, Some("todo".to_string()));
        assert_eq!(task.priority, Some("high".to_string()));
        assert!(task.details.unwrap().contains("task description"));
    }

    #[test]
    fn test_parse_tags() {
        let tags = parse_tags("#task #backend #project:auth");
        assert_eq!(tags, vec!["#task", "#backend", "#project:auth"]);
    }

    #[test]
    fn test_parse_date() {
        let dt = parse_date("2026-03-01").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-03-01");
    }

    #[test]
    fn test_roundtrip() {
        let original = LegacyTask {
            title: Some("Test task".to_string()),
            assignee: Some("John".to_string()),
            assigned_to: Some("john".to_string()),
            status: Some("todo".to_string()),
            priority: Some("high".to_string()),
            details: Some("Task details here".to_string()),
            created_at: None,
            due: None,
            tags: vec!["#task".to_string(), "#backend".to_string()],
        };

        let markdown = task_to_markdown(&original);
        let parsed = parse_task(&markdown).unwrap();

        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.assignee, original.assignee);
        assert_eq!(parsed.assigned_to, original.assigned_to);
        assert_eq!(parsed.status, original.status);
        assert_eq!(parsed.priority, original.priority);
        assert_eq!(parsed.tags, original.tags);
    }

    #[test]
    fn test_invalid_status_is_ignored() {
        let content = "#title: Test\n#status: invalid";
        let task = parse_task(content).unwrap();
        assert!(task.status.is_none());
    }

    // Tests for parse_element with YAML frontmatter

    #[test]
    fn test_parse_element_program() {
        let content = r#"---
title: My Program
status: active
tags:
  - program
  - backend
type: program
---

# DESCRIPTION
This is the program description.
"#;
        let element = parse_element(content)
            .unwrap()
            .expect("Should parse program");
        assert_eq!(element.kind(), ElementKind::Program);
        assert_eq!(element.title(), "My Program");
        assert_eq!(element.status(), "active");

        if let Element::Program(program) = element {
            assert!(program.tags.contains(&"program".to_string()));
            assert!(program.description.contains("program description"));
        } else {
            panic!("Expected Program element");
        }
    }

    #[test]
    fn test_parse_element_task() {
        let content = r#"---
title: Implement feature X
status: todo
creation_date: 2026-03-04T12:00:00Z
created_by: alice
assigned_to: bob
due_date: "2026-03-10"
type: task
tags:
  - backend
  - urgent
---

# Description
This task is about implementing feature X.
"#;
        let element = parse_element(content).unwrap().expect("Should parse task");
        assert_eq!(element.kind(), ElementKind::Task);
        assert_eq!(element.title(), "Implement feature X");
        assert_eq!(element.status(), "todo");

        if let Element::Task(task) = element {
            assert_eq!(task.created_by, Some("alice".to_string()));
            assert_eq!(task.assigned_to, Some("bob".to_string()));
            assert!(task.description.contains("implementing feature X"));
        } else {
            panic!("Expected Task element");
        }
    }

    #[test]
    fn test_parse_element_no_frontmatter() {
        let content = "Just some regular markdown\nwithout frontmatter";
        let result = parse_element(content).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_element_project() {
        let content = r#"---
title: Website Redesign
status: doing
creation_date: 2026-03-01T09:00:00Z
type: project
---

# Description
Redesign the company website.
"#;
        let element = parse_element(content)
            .unwrap()
            .expect("Should parse project");
        assert_eq!(element.kind(), ElementKind::Project);
        assert_eq!(element.title(), "Website Redesign");
    }

    #[test]
    fn test_parse_element_milestone() {
        let content = r#"---
title: MVP Release
status: todo
creation_date: 2026-03-01T09:00:00Z
type: milestone
---

# Description
First release with core features.
"#;
        let element = parse_element(content)
            .unwrap()
            .expect("Should parse milestone");
        assert_eq!(element.kind(), ElementKind::Milestone);
        assert_eq!(element.title(), "MVP Release");
    }

    #[test]
    fn test_element_roundtrip_task() {
        let task = Task::new("Test Task");
        let element = Element::Task(task);
        let markdown = element_to_markdown(&element);
        let parsed = parse_element(&markdown)
            .unwrap_or_else(|e| panic!("Parse error: {:?}", e))
            .expect("Should parse");

        assert_eq!(parsed.kind(), ElementKind::Task);
        assert_eq!(parsed.title(), "Test Task");
    }
}
