# AGENTS.md - Chronicle Development Guide

This file provides guidance for agentic coding agents working in the chronicle codebase.

## Project Overview

- **Type**: Rust TUI Application
- **Storage**: Markdown files with YAML frontmatter + strategic folder structure
- **Config**: `~/.config/chronicle/config.toml`
- **Edition**: Rust 2021
- **Modules**: `app`, `commands`, `config`, `error`, `models`, `navigator`, `storage`, `templates`, `viewer`, `wizard`

## Build Commands

```bash
# Build the project
cargo build

# Run (starts TUI)
cargo run

# Check for compilation errors without building
cargo check

# Run tests
cargo test

# Run a single test by name
cargo test <test_name>

# Generate documentation
cargo doc --open

# Release build (optimized binary)
cargo build --release

# Install locally
cargo install --path .
```

## Architecture

### Entry Point
`src/main.rs` loads config and runs the TUI app: `config::Config::load_or_create()?` → `app::App::new(config)` → `app.run()?`

### Key Modules

| Module | Purpose |
|--------|---------|
| `src/main.rs` | Application entry point |
| `src/lib.rs` | Crate root with error re-exports |
| `src/error.rs` | Layered error types (ConfigError, StorageError, ModelError) |
| `src/config/mod.rs` | Config loading from `~/.config/chronicle/config.toml` |
| `src/models/mod.rs` | Domain data structures (Task, Program, Project, etc.) |
| `src/storage/mod.rs` | File operations, workspace discovery, element read/write |
| `src/navigator/mod.rs` | Tree navigation state and rendering |
| `src/viewer/mod.rs` | Element content rendering |
| `src/commands/mod.rs` | Command definitions and command palette |
| `src/wizard/mod.rs` | Element creation wizard state machine |
| `src/templates/mod.rs` | Template loading from config directory |
| `src/app.rs` | Main TUI application, event loop, key handling |

### Dependencies
- `ratatui` + `crossterm`: Terminal UI
- `serde` + `serde_yaml`: Serialization with YAML frontmatter
- `tokio`: Async runtime
- `tracing`: Logging
- `thiserror` + `anyhow`: Error handling
- `uuid`: Unique identifiers
- `chrono`: Date/time handling
- `pulldown-cmark`: Markdown parsing
- `glob`: File pattern matching
- `dirs`: Config/data directory resolution

## Golden Rules (All Agents Must Follow)

1. **DESIGN.md is the source of truth.** If code contradicts the design, the code is wrong — unless the design has a flaw, in which case flag it to the architect.
2. **No agent edits files outside their designated scope.** This is a hard constraint, not a suggestion.
3. **Branch before significant work.** The architect creates a branch before every sprint. No work happens on `main` directly.
4. **No destructive git commands.** `git reset`, `git rebase`, `git clean`, `git restore`, and `git checkout -- <file>` are banned for all agents without exception.
5. **No remote git operations.** `git push`, `git pull`, and `git fetch` are off limits. This repo is local only.
6. **DRY**: One canonical place for each piece of logic.
7. **KISS**: Complexity must justify itself. Simpler is always preferred when functionality is equivalent.
8. **Readable first**: Code is read far more than it is written. Optimize for the reader.
9. **Tests are mandatory**: New functionality without tests is not done.
10. **`cargo clippy` clean is non-negotiable**: Treat warnings as errors.

## Git Lifecycle (Architect's Responsibility)

```bash
# Sprint start
git status                              # confirm clean state
git tag stable/<prev-milestone>-date    # checkpoint before touching anything
git checkout -b feat/<sprint-name>
# update DESIGN.md Current Sprint: branch name + task checklist

# Sprint end (after quality APPROVED)
git log --oneline -10                   # note HEAD sha; include it in your response
git diff master...HEAD                  # final review; no surprises
git checkout master
git merge --no-ff feat/<sprint-name>
git tag stable/<sprint-name>-YYYY-MM-DD

# On disaster: STOP. Do not attempt recovery.
# Report: the command, the output, the last known HEAD sha. Then wait.
```

## Rust Project Conventions

- **Edition**: Rust 2021
- **Error handling**: `thiserror` for library (layered error types), `anyhow` for binary entry point only
- **Formatting**: `rustfmt` with project defaults — run before every commit
- **Lints**: `#![deny(warnings)]` and `#![deny(clippy::all)]` at crate root (lib.rs / main.rs)
- **Logging**: `tracing` crate (`tracing::warn!`, `tracing::error!`, etc.)
- **Testing**: Unit tests in `#[cfg(test)]` blocks; use `tempfile` crate for filesystem tests
- **Documentation**: Module-level with `//!`, public items with `///` doc comments
- **Serialization**: `serde` with YAML frontmatter; use `#[serde(rename = "...")]` for field renaming

## Code Style Guidelines

### Imports
- Group std library imports first, then external crates, then crate-internal
- Use absolute imports for internal modules: `use crate::config::Config;`
- Prefer importing traits directly when used: `use serde::{Deserialize, Serialize};`
- Order imports alphabetically within groups

### Types and Structs
- Use `PascalCase` for structs, enums, and type aliases
- Use `snake_case` for field names and variables
- Use `Option<T>` for nullable values
- Use `Result<T>` (from `crate::Result`) for fallible operations returning a value
- Use `Result<()>` for fallible operations that don't return a value

### Naming Conventions
- Structs with data: noun forms (`Config`, `Task`, `App`)
- Enums: noun or verb forms (`ViewType`, `CommandAction`)
- Trait names: noun forms describing capability (`JournalStorage`, `WorkspaceStorage`)
- Boolean methods: predicate forms (`is_complete()`, `exists()`)
- Error types: suffix with `Error` (`ParseError`, `ConfigError`)

### Serde Patterns

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    #[serde(rename = "creation_date")]
    pub creation_date: DateTime<Utc>,
}

// Use untagged for enums that serialize to different types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Element {
    Program(Program),
    Project(Project),
    Milestone(Milestone),
    Task(Task),
}

// Implement Display/FromStr for enums that represent user-facing kinds
impl std::fmt::Display for ElementKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementKind::Program => write!(f, "program"),
            ElementKind::Project => write!(f, "project"),
        }
    }
}
```

### Error Handling

```rust
// Library code: layered error types with thiserror
// Each domain has its own error type (ConfigError, StorageError, ModelError)
// A top-level Error enum wraps them all with #[from] for automatic conversion

use thiserror::Error;

// Sub-error example (in config module)
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

// Top-level error (in error.rs) aggregates sub-errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Model error: {0}")]
    Model(#[from] ModelError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

// Type alias for convenience
pub type Result<T> = std::result::Result<T, Error>;

// Binary code: use anyhow only at the top-level entry point
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let app = crate::app::run().context("Failed to start application")?;
    Ok(())
}

// Never use unwrap() in library code except in tests
// In binaries, unwrap() is acceptable at top-level entry points
```

### Testing Guidelines

```rust
// Unit tests in #[cfg(test)] blocks
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task");
        assert_eq!(task.title, "Test task");
    }
}

// For filesystem tests, use tempfile::TempDir
#[cfg(test)]
mod storage_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::new(temp_dir.path());
        // test code...
        Ok(())
    }
}

// Integration tests go in tests/ directory
// Name files by feature: test_journaling.rs, test_navigation.rs
```

### TUI Development
- Use `crossterm` for terminal input/output
- Use `ratatui` for UI components
- Handle terminal reinitialization after external editor launches
- Restore terminal on panics with `std::panic::set_hook`

### File Operations
- Use `std::fs` for file read/write operations
- Create parent directories with `fs::create_dir_all(parent)?` before writing
- Handle missing files gracefully with `Option` or early returns

## Data Hierarchy

```
~/chronicle/workspace/
├── programs/
│   └── {program}/
│       └── {project}/
│           └── {milestone}/
│               └── {task}.md
├── planning/
│   ├── current/
│   └── history/
├── journal/
│   └── YYYY/MM/
│       └── DD.md
├── .archive/
└── templates/
    ├── task.md
    ├── program.md
    ├── project.md
    └── milestone.md
```

## Development Notes

- Default editor is `hx` (helix) but configurable via config file
- Config stored at `~/.config/chronicle/config.toml`
- Use `cargo test <test_name>` to run specific tests
- Use `cargo check` for fast compilation checks during development
