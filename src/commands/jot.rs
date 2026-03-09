use crate::config::Config;
use crate::storage::JournalStorage;
use crate::Result;
use chrono::Local;
use std::fs;
use std::io::Write;

pub fn run(text: &str) -> Result<()> {
    let config = Config::load_or_create()?;
    let workspace = config.workspace;
    let (path, _) = workspace.open_or_create_today_journal()?;

    let entry = format!(
        "- {}  {}
",
        Local::now().format("%H:%M"),
        text
    );

    fs::OpenOptions::new()
        .append(true)
        .open(&path)?
        .write_all(entry.as_bytes())?;

    println!("Appended journal entry: {}", path.display());
    Ok(())
}
