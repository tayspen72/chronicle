use crate::model::{PlanningSession, SessionStatus};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

pub type Result<T> = std::result::Result<T, PlanningError>;

#[derive(Debug, thiserror::Error)]
pub enum PlanningError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Invalid session file: {0}")]
    InvalidFile(String),
}

/// Generates a unique session UUID.
pub fn generate_session_uuid() -> String {
    format!("ses_{}", uuid::Uuid::new_v4())
}

/// Gets the path to the current planning session directory.
pub fn current_session_dir(workspace: &Path) -> PathBuf {
    workspace.join("planning").join("current")
}

/// Gets the path to the archived planning session directory.
pub fn history_session_dir(workspace: &Path) -> PathBuf {
    workspace.join("planning").join("history")
}

/// Lists all active planning sessions in the current directory.
pub fn list_active_sessions(workspace: &Path) -> Result<Vec<PathBuf>> {
    let dir = current_session_dir(workspace);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            sessions.push(path);
        }
    }
    Ok(sessions)
}

/// Creates a new planning session file.
pub fn create_planning_session(
    workspace: &Path,
    uuid: &str,
    title: &str,
    creation_date: &str,
    start_date: &str,
    end_date: &str,
    duration: &str,
) -> Result<PathBuf> {
    let dir = current_session_dir(workspace);
    fs::create_dir_all(&dir)?;

    let filename = format!("{}-planning.md", start_date);
    let target_path = dir.join(&filename);

    let mut values = HashMap::new();
    values.insert("UUID".to_string(), uuid.to_string());
    values.insert("NAME".to_string(), title.to_string());
    values.insert("TODAY".to_string(), creation_date.to_string());
    values.insert("START_DATE".to_string(), start_date.to_string());
    values.insert("END_DATE".to_string(), end_date.to_string());
    values.insert("DURATION".to_string(), duration.to_string());
    values.insert("STATUS".to_string(), "active".to_string());
    values.insert("OWNER".to_string(), "".to_string());

    let template = include_str!("../../templates/planning_session.md");
    let content = resolve_template(template, &values);

    fs::write(&target_path, content)?;
    debug!(path = %target_path.display(), "created planning session");
    Ok(target_path)
}

/// Saves a planning session to disk (overwrites existing).
pub fn save_planning_session(workspace: &Path, session: &PlanningSession) -> Result<PathBuf> {
    let dir = current_session_dir(workspace);
    fs::create_dir_all(&dir)?;

    let filename = format!("{}-planning.md", session.start_date);
    let path = dir.join(&filename);

    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&serde_yaml::to_string(session)?);
    content.push_str("---\n\n");
    content.push_str(&format!(
        "# Planning Session: {} to {}\n\n## Tasks\n\n",
        session.start_date, session.end_date
    ));

    if session.tasks.is_empty() {
        content.push_str("_Selected tasks will appear here when added to the planning session._\n");
    } else {
        for task_uuid in &session.tasks {
            content.push_str(&format!("- Task UUID: {}\n", task_uuid));
        }
    }

    fs::write(&path, content)?;
    debug!(path = %path.display(), "saved planning session");
    Ok(path)
}

/// Loads a planning session from disk.
pub fn load_planning_session(path: &Path) -> Result<PlanningSession> {
    let content = fs::read_to_string(path)?;
    parse_session_from_content(&content)
}

/// Parses a planning session from content.
pub fn parse_session_from_content(content: &str) -> Result<PlanningSession> {
    let frontmatter = extract_frontmatter(content)?;
    let session: PlanningSession = serde_yaml::from_str(&frontmatter)?;
    Ok(session)
}

/// Extracts YAML frontmatter from content.
fn extract_frontmatter(content: &str) -> Result<String> {
    let start = content
        .find("---\n")
        .ok_or_else(|| PlanningError::InvalidFile("Missing frontmatter start".to_string()))?;
    let content_after_start = &content[start + 4..];

    let end = content_after_start
        .find("\n---")
        .ok_or_else(|| PlanningError::InvalidFile("Missing frontmatter end".to_string()))?;

    Ok(content_after_start[..end].to_string())
}

/// Archives a planning session by moving it to the history directory.
pub fn archive_planning_session(workspace: &Path, start_date: &str) -> Result<PathBuf> {
    let current_dir = current_session_dir(workspace);
    let history_dir = history_session_dir(workspace);
    fs::create_dir_all(&history_dir)?;

    let filename = format!("{}-planning.md", start_date);
    let current_path = current_dir.join(&filename);
    let history_path = history_dir.join(&filename);

    if !current_path.exists() {
        return Err(PlanningError::NotFound(filename));
    }

    // Update status to Archived before moving
    let mut session = load_planning_session(&current_path)?;
    session.status = SessionStatus::Archived;
    save_planning_session(workspace, &session)?;

    fs::rename(&current_path, &history_path)?;
    debug!(
        from = %current_path.display(),
        to = %history_path.display(),
        "archived planning session"
    );
    Ok(history_path)
}

/// Resolves template placeholders.
fn resolve_template(template: &str, values: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in values {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_planning_session() {
        let dir = tempdir().unwrap();
        let uuid = generate_session_uuid();
        let path = create_planning_session(
            dir.path(),
            &uuid,
            "Weekly Plan",
            "2026-03-12",
            "2026-03-12",
            "2026-03-19",
            "weekly",
        )
        .unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("2026-03-12-planning.md"));
    }

    #[test]
    fn test_save_and_load_session() {
        let dir = tempdir().unwrap();
        let session = PlanningSession {
            element_type: "planning".to_string(),
            uuid: "ses_test".to_string(),
            title: "Weekly Plan".to_string(),
            creation_date: "2026-03-12".to_string(),
            created_by: None,
            start_date: "2026-03-12".to_string(),
            end_date: "2026-03-19".to_string(),
            duration: "weekly".to_string(),
            status: SessionStatus::Active,
            tasks: vec!["task1".to_string(), "task2".to_string()],
        };

        save_planning_session(dir.path(), &session).unwrap();
        let loaded =
            load_planning_session(&current_session_dir(dir.path()).join("2026-03-12-planning.md"))
                .unwrap();

        assert_eq!(loaded.uuid, "ses_test");
        assert_eq!(loaded.tasks.len(), 2);
    }

    #[test]
    fn test_archive_session() {
        let dir = tempdir().unwrap();
        let uuid = generate_session_uuid();
        create_planning_session(
            dir.path(),
            &uuid,
            "Weekly Plan",
            "2026-03-12",
            "2026-03-12",
            "2026-03-19",
            "weekly",
        )
        .unwrap();

        let history_path = archive_planning_session(dir.path(), "2026-03-12").unwrap();
        assert!(history_path.exists());
        assert!(history_path.to_string_lossy().contains("history"));

        let current_path = current_session_dir(dir.path()).join("2026-03-12-planning.md");
        assert!(!current_path.exists());
    }
}
