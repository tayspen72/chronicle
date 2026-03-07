# Chronicle Development Guide

This file provides guidance for agentic coding agents working in the chronicle codebase.

## Project Overview

- **Type**: Rust TUI Application (Markdown-native planner and journal)
- **Storage**: Markdown files with YAML frontmatter + folder hierarchy
- **Config**: `~/.config/chronicle/config.toml`
- **Edition**: Rust 2021

## Build Commands

```bash
# Build the project
cargo build

# Run (starts TUI)
cargo run

# Check for compilation errors
cargo check

# Run all tests
cargo test

# Run a single test by exact name
cargo test test_name

# Run tests matching a pattern
cargo test part_of_name

# Run documentation tests
cargo test --doc

# Lint and format check
cargo clippy
cargo fmt --check

# Generate documentation
cargo doc --open

# Release build (optimized)
cargo build --release

# Install locally
cargo install --path .
```

## Golden Rules

1. **DESIGN.md is the source of truth.** If code contradicts the design, the code is wrong.
2. **No agent edits files outside their designated scope.**
3. **Branch before significant work.** The architect creates a branch before every sprint.
4. **No destructive git commands.** `git reset`, `git rebase`, `git clean`, `git restore` are banned.
5. **No remote git operations.** `git push`, `git pull`, `git fetch` are off limits.
6. **Tests are mandatory.** New functionality without tests is not done.
7. **`cargo clippy` clean is non-negotiable.** Treat warnings as errors.

## Architecture

| Module | Purpose |
|--------|---------|
| `src/main.rs` | Binary entry point |
| `src/lib.rs` | Crate root, re-exports Error/Result |
| `src/error.rs` | Layered error types (thiserror) |
| `src/config.rs` | Config loading/saving |
| `src/model/mod.rs` | Domain types (Program, Project, Milestone, Task, Element) |
| `src/storage/mod.rs` | File I/O, workspace discovery |
| `src/storage/md.rs` | Markdown parsing/serialization |
| `src/commands/mod.rs` | CLI commands (init, new_task, jot, extract) |
| `src/tui/mod.rs` | Main TUI app, event loop |
| `src/tui/navigation.rs` | Tree navigation state |
| `src/tui/views/` | View rendering components |
| `src/tui/command.rs` | Command palette |
| `src/tui/layout.rs` | Layout management |

## Code Style

### Imports
- Group: std → external → crate-internal
- Use absolute paths: `use crate::config::Config;`
- Order alphabetically within groups

### Types and Naming
- Structs/Enums: `PascalCase` (e.g., `Task`, `ElementKind`)
- Fields/Variables: `snake_case`
- Boolean methods: `is_complete()`, `exists()`
- Error types: suffix with `Error` (e.g., `ConfigError`)

### Serde Patterns
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    #[serde(rename = "creation_date")]
    pub creation_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Element {
    Program(Program),
    Project(Project),
    Milestone(Milestone),
    Task(Task),
}
```

### Error Handling
- **Library code**: Use `thiserror` with layered errors
- **Binary code**: Use `anyhow` at entry point only
- **Never** use `unwrap()` in library code (except tests)

```rust
// error.rs - layered approach
#[derive(Error, Debug)]
pub enum Error {
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

// main.rs - binary entry point
use anyhow::Result;
fn main() -> Result<()> { ... }
```

### Testing
- Unit tests in `#[cfg(test)]` blocks within source files
- Use `tempfile::TempDir` for filesystem tests
- Run specific tests: `cargo test test_name`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task");
        assert_eq!(task.title, "Test task");
    }
}
```

## Data Hierarchy

```
~/chronicle/workspace/
├── programs/{program}/{project}/{milestone}/{task}.md
├── planning/{current,history}/
├── journal/YYYY/MM/DD.md
├── .archive/
└── templates/{task,program,project,milestone}.md
```

## Lint Configuration

The crate uses `#![deny(warnings)]` and `#![deny(clippy::all)]`. Run `cargo fmt` and `cargo clippy` before commits.

## Editor

Default editor is `hx` (helix), configurable via config file.
