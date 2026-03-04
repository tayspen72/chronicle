use crate::Result;
use std::fs;

pub fn run() -> Result<()> {
    let dirs = [
        "data/programs",
        "data/projects",
        "data/milestones",
        "data/tasks",
        "data/journal",
        "data/backlog",
        "templates",
    ];

    for d in dirs {
        fs::create_dir_all(d)?;
    }

    println!("Initialized data/ and templates/ directories.");
    Ok(())
}
