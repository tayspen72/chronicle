use crate::Result;
use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn run(text: &str) -> Result<()> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let path = PathBuf::from("data/journal").join(format!("{date}.md"));
    let entry = format!(
        "- {}  {}
",
        Local::now().format("%H:%M"),
        text
    );

    if path.exists() {
        fs::OpenOptions::new()
            .append(true)
            .open(&path)?
            .write_all(entry.as_bytes())?;
    } else {
        let mut header = String::new();
        header.push_str(&format!(
            "# Journal — {date}

"
        ));
        header.push_str(
            "_Use `/todo` to mark actionable items. Use `/note` for highlights._

",
        );
        header.push_str(
            "## Entries

",
        );
        fs::write(&path, header + &entry)?;
    }

    println!("Appended journal entry: {}", path.display());
    Ok(())
}
