use crate::commands::task_template::{slugify, write_task_from_template};
use crate::config::Config;
use crate::storage::JournalStorage;
use crate::Result;
use std::fs;

/// Naive extractor: scans journal for lines containing `/todo` and writes backlog tasks.
pub fn run() -> Result<()> {
    let config = Config::load_or_create()?;
    let workspace = config.workspace.clone();
    let default_status = config
        .workflow
        .first()
        .cloned()
        .unwrap_or_else(|| "todo".to_string());

    let journal_dir = workspace.journal_dir();
    let backlog_dir = workspace.join("planning").join("current");
    fs::create_dir_all(&backlog_dir)?;

    if !journal_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(journal_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        for line in content.lines() {
            if let Some(idx) = line.find("/todo") {
                let todo_text = line[(idx + "/todo".len())..].trim();
                if !todo_text.is_empty() {
                    let slug = slugify(todo_text);
                    let fname = backlog_dir.join(format!("backlog-{slug}.md"));
                    if !fname.exists() {
                        write_task_from_template(
                            &workspace,
                            &fname,
                            todo_text,
                            "",
                            &config.owner,
                            &default_status,
                        )?;
                        println!("Extracted backlog: {}", fname.display());
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_for_extract() {
        assert_eq!(slugify("Backlog Item"), "backlog-item");
    }
}
