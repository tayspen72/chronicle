use crate::Result;
use std::fs;
use std::path::Path;

/// Naive extractor: scans journal for lines containing `/todo` and writes backlog tasks.
pub fn run() -> Result<()> {
    let journal_dir = Path::new("data/journal");
    let backlog_dir = Path::new("data/backlog");
    fs::create_dir_all(backlog_dir)?;

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
                    let slug = todo_text
                        .to_lowercase()
                        .replace(' ', "-")
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                        .collect::<String>();
                    let fname = backlog_dir.join(format!("backlog-{slug}.md"));
                    if !fname.exists() {
                        let tmpl = include_str!("../../templates/task.md")
                            .replace("{{TASK_NAME}}", todo_text);
                        fs::write(&fname, tmpl)?;
                        println!("Extracted backlog: {}", fname.display());
                    }
                }
            }
        }
    }
    Ok(())
}
