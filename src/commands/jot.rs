
use anyhow::Result;
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use std::io::Write;

pub fn run(text: &str) -> Result<()> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let path = PathBuf::from("data/journal").join(format!("{date}.md"));
    let entry = format!("- {}  {}
", Local::now().format("%H:%M"), text);

    if path.exists() {
        fs::OpenOptions::new().append(true).open(&path)?.write_all(entry.as_bytes())?;
    } else {
        let mut header = String::new();
        header.push_str(&format!("# Journal — {date}

"));
        header.push_str("_Use `/todo` to mark actionable items. Use `/note` for highlights._

");
        header.push_str("## Entries

");
        fs::write(&path, header + &entry)?;
    }

    println!("Appended journal entry: {}", path.display());
    Ok(())
}
