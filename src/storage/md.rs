use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};

use crate::model::{ParseError, Task};

/// Parse task metadata from markdown content.
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
pub fn parse_task(content: &str) -> Result<Task> {
    let mut task = Task::new();
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

    Err(ParseError {
        message: format!("Could not parse date: '{}'", input),
    }
    .into())
}

/// Generate markdown content from a Task struct
pub fn task_to_markdown(task: &Task) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let original = Task {
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
}
