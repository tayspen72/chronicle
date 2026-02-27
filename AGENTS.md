# AGENTS.md - Chronicle Development Guide

This file provides guidance for agentic coding agents working in the chronicle codebase.

## Project Overview

Chronicle is a Markdown-native planner and journal with a terminal UI (TUI). It uses a hierarchical folder structure (`programs/ → projects/ → milestones/ → tasks/`) plus `journal/` and `backlog/` for daily notes.

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
`src/main.rs` loads config and runs the TUI app: `config::Config::load_or_create()?` → `tui::App::new(config)` → `app.run()?`

### Key Modules

| Module | Purpose |
|--------|---------|
| `src/main.rs` | Application entry point |
| `src/config.rs` | Config loading from `~/.config/chronicle.rs/config.toml` |
| `src/storage/mod.rs` | Trait-based file operations (`JournalStorage`, `WorkspaceStorage`) |
| `src/storage/md.rs` | Markdown parsing utilities |
| `src/model/mod.rs` | Data structures (`Task`, `ParseError`) |
| `src/tui/mod.rs` | Main TUI application with views, navigation, commands |
| `src/tui/views/` | TUI view components |
| `src/tui/command.rs` | Command handling |
| `src/tui/navigation.rs` | Navigation state management |
| `src/commands/` | CLI commands (extract, init, jot, new_task) |

### Dependencies
- `ratatui` + `crossterm`: Terminal UI
- `serde` + `serde_yaml` + `serde_json`: Serialization
- `chrono`: Date/time handling
- `anyhow` + `thiserror`: Error handling
- `directories`: Config/data directory resolution
- `comrak`: Markdown parsing
- `regex`: Pattern matching

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
- Use `Result<T>` (from `anyhow`) for fallible operations returning a value
- Use `Result<()>` for fallible operations that don't return a value

### Naming Conventions
- Structs with data: noun forms (`Config`, `Task`, `App`)
- Enums: noun or verb forms (`ViewType`, `CommandAction`)
- Trait names: noun forms describing capability (`JournalStorage`, `WorkspaceStorage`)
- Boolean methods: predicate forms (`is_complete()`, `exists()`)
- Error types: suffix with `Error` (`ParseError`, `ConfigError`)

### Error Handling
- Use `anyhow::Result<T>` for application-level error handling
- Use `?` operator for propagating errors
- Create custom error types with `thiserror` for structured errors
- Implement `std::fmt::Display` and `std::error::Error` for custom errors
- Use `anyhow::anyhow!("message")` for simple contextual errors

### Struct Definitions
- Prefer `#[derive(Debug, Clone, Serialize, Deserialize, Default)]` for data structs
- Use `pub` fields for structs that are primarily data containers
- Use tuple structs for wrapper types when appropriate
- Add doc comments (`///`) for public structs and important methods

### Traits and Implementations
- Define traits in `storage/mod.rs` for file operations
- Implement traits on `PathBuf` for storage operations (see `storage/mod.rs`)
- Use trait bounds on generic functions when appropriate
- Prefer default method implementations in traits when sensible

### Pattern: Impl Blocks with Associated Functions
```rust
impl Config {
    pub fn config_path() -> Option<PathBuf> { ... }
    pub fn load_or_create() -> Result<Self> { ... }
}
```

### Pattern: Trait Implementation on PathBuf
```rust
impl JournalStorage for PathBuf {
    fn journal_dir(&self) -> PathBuf { self.join("journal") }
    // ...
}
```

### Pattern: Custom Error Type
```rust
#[derive(Debug, Clone, Default)]
pub struct ParseError { pub message: String }

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}
```

### TUI Development
- Use `crossterm` for terminal input/output
- Use `ratatui` for UI components
- Handle terminal reinitialization after external editor launches
- Use `ViewType` enum to manage different screen states
- Restore terminal on panics with `std::panic::set_hook`

### File Operations
- Use `std::fs` for file read/write operations
- Use `include_str!` macro for embedding template files
- Create parent directories with `fs::create_dir_all(parent)?` before writing
- Handle missing files gracefully with `Option` or early returns

### Markdown Conventions
Tasks use hashtag-style keys at line start:
- `#title:`, `#assignee:`, `#assigned-to:`, `#status:`, `#priority:`, `#due:`, `#tags:`
- Status values: `todo|doing|done|blocked`
- Priority values: `low|med|high|urgent`

## Data Hierarchy

```
~/chronicle/workspace/
├── programs/
│   └── {program}/
│       └── {project}/
│           └── {milestone}/
│               └── {task}.md
├── journal/
│   └── YYYY-MM-DD.md
├── backlog/
└── templates/
    ├── task.md
    ├── task_min.md
    ├── program.md
    ├── project.md
    └── milestone.md
```

## Development Notes

- No linting tools are currently configured (no clippy, rustfmt.toml)
- Tests exist but may be minimal (MVP still in progress)
- Default editor is `hx` (helix) but configurable via config file
- Config stored at `~/.config/chronicle.rs/config.toml`
- Use `cargo test <test_name>` to run specific tests
- Use `cargo check` for fast compilation checks during development
