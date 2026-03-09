use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::Result;
use crate::storage::WorkspaceStorage;

pub fn slugify(input: &str) -> String {
    let normalized = input
        .to_lowercase()
        .replace([' ', '/', '\\'], "-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect::<String>();

    let mut collapsed = String::with_capacity(normalized.len());
    let mut previous_was_dash = false;
    for ch in normalized.chars() {
        if ch == '-' {
            if !previous_was_dash {
                collapsed.push(ch);
            }
            previous_was_dash = true;
        } else {
            collapsed.push(ch);
            previous_was_dash = false;
        }
    }

    collapsed.trim_matches('-').to_string()
}

pub fn write_task_from_template(
    workspace: &PathBuf,
    target_path: &Path,
    title: &str,
    description: &str,
    owner: &str,
    default_status: &str,
) -> Result<PathBuf> {
    let mut values = HashMap::new();
    values.insert("NAME".to_string(), title.to_string());
    values.insert("TASK_NAME".to_string(), title.to_string());
    values.insert("DESCRIPTION".to_string(), description.to_string());
    values.insert(
        "TODAY".to_string(),
        Utc::now().format("%Y-%m-%d").to_string(),
    );
    values.insert("OWNER".to_string(), owner.to_string());
    values.insert("DEFAULT_STATUS".to_string(), default_status.to_string());
    values.insert("ASSIGNED_TO".to_string(), String::new());
    values.insert("DUE_DATE".to_string(), String::new());

    let strip_labels: HashSet<String> = HashSet::new();
    workspace.create_from_template("task", target_path, &values, &strip_labels)
}
