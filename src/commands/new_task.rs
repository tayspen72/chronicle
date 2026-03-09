use crate::Result;
use crate::commands::task_template::{slugify, write_task_from_template};
use crate::config::Config;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

pub fn run(title: &str, scope: Option<&str>) -> Result<()> {
    let config = Config::load_or_create()?;
    let workspace = config.workspace.clone();
    let default_status = config
        .workflow
        .first()
        .cloned()
        .unwrap_or_else(|| "todo".to_string());

    let slug = slugify(title);
    let ts = Utc::now().format("%Y%m%d").to_string();

    let mut dir = workspace.join("planning").join("current");
    if let Some(s) = scope {
        let scoped = PathBuf::from(s);
        dir = if scoped.is_absolute() {
            scoped
        } else {
            workspace.join(scoped)
        };
        if !dir.starts_with(&workspace) {
            dir = workspace.join("planning").join("current").join(s);
        }
    }
    fs::create_dir_all(&dir)?;

    let path = dir.join(format!("{ts}-{slug}.md"));
    write_task_from_template(&workspace, &path, title, "", &config.owner, &default_status)?;
    println!("Created task: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_title() {
        assert_eq!(slugify("My Task"), "my-task");
        assert_eq!(slugify("Task/With\\\\Separators"), "task-with-separators");
    }
}
