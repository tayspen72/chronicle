
use anyhow::Result;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

pub fn run(title: &str, scope: Option<&str>) -> Result<()> {
    let slug = title
        .to_lowercase()
        .replace(' ', "-")
        .replace('/', "-")
        .replace('\', "-");
    let ts = Utc::now().format("%Y%m%d").to_string();

    let mut dir = PathBuf::from("data/tasks");
    if let Some(s) = scope {
        dir = PathBuf::from(s);
        if !dir.starts_with("data/") {
            dir = PathBuf::from("data").join(dir);
        }
    }
    fs::create_dir_all(&dir)?;

    let path = dir.join(format!("{ts}-{slug}.md"));
    let content = include_str!("../../templates/task.md");
    let content = content.replace("{{TITLE}}", title);
    fs::write(&path, content)?;
    println!("Created task: {}", path.display());
    Ok(())
}
