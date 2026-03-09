use crate::config::Config;
use crate::Result;
use std::fs;

pub fn run() -> Result<()> {
    let config = Config::load_or_create()?;
    let workspace = config.workspace;

    let dirs = [
        workspace.join("programs"),
        workspace.join("planning").join("current"),
        workspace.join("planning").join("history"),
        workspace.join("journal"),
        workspace.join(".archive"),
        workspace.join("templates"),
    ];

    for d in dirs {
        fs::create_dir_all(d)?;
    }

    println!(
        "Initialized workspace directories at {}.",
        workspace.display()
    );
    Ok(())
}
