# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**chronicle** is a Markdown-native planner and journal with a terminal UI. It uses a hierarchical folder structure (`programs/ → projects/ → milestones/ → tasks/`) plus `journal/` and `backlog/` for daily notes.

## Commands

```bash
# Build
cargo build

# Run (starts TUI)
cargo run

# Run tests
cargo test

# Install locally
cargo install --path .

# Release build (optimized binary)
cargo build --release
```

## Architecture

### Entry Point

`main.rs` loads config and runs the TUI app: `config::Config::load_or_create()?` → `tui::App::new(config)` → `app.run()?`

### Key Modules

- **`config.rs`**: Handles config loading from `~/.config/chronicle.rs/config.toml`. First run prompts for data path and editor. Default data path is `~/chronicle/workspace`, default editor is `hx` (helix).

- **`storage/mod.rs`**: Defines `JournalStorage` and `WorkspaceStorage` traits, implemented on `PathBuf`. Handles file/directory operations for the data hierarchy.

- **`model/mod.rs`**: Contains the `Task` struct with fields like `title`, `assigned_to`, `status`, `priority`, `due`, `tags`.

- **`tui/mod.rs`**: The main TUI application. Contains:
  - `ViewType` enum for all screens (ProgramsList, ProjectsList, MilestonesList, TasksList, Journal, etc.)
  - `App` struct with state including `current_program`, `current_project`, `current_milestone` for workspace navigation
  - Command palette system (`filter_commands()`, `execute_command()`)
  - Terminal reinitialization logic after external editor launches (`needs_terminal_reinit` flag)

- **`commands/`**: Individual command modules (extract, init, jot, new_task) that can be invoked.

### Data Hierarchy

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
    └── task_min.md
```

### Markdown Conventions

Tasks use hashtag-style keys at line start:
- `#title:`, `#assignee:`, `#assigned-to:`, `#status:`, `#priority:`, `#due:`, `#tags:`

Status values: `todo|doing|done|blocked`
Priority values: `low|med|high|urgent`

### TUI Keybindings

- `/`: Open command palette
- `Esc`: Close command palette or return to previous view
- `↑/↓`: Navigate lists
- `Enter`: Select item

### Terminal Reinitialization

When launching the external editor, the app:
1. Leaves alternate screen and disables raw mode
2. Clears terminal
3. Runs the editor via `std::process::Command`
4. Sets `needs_terminal_reinit = true`
5. On next loop iteration, reinitializes terminal if flag is set

This is necessary because external editors interfere with the TUI's terminal state.

## Dependencies

- `ratatui` and `crossterm`: Terminal UI and input handling
- `serde`/`serde_yaml`: Config serialization
- `chrono`: Date handling for journal entries
- `anyhow`/`thiserror`: Error handling
- `directories`: Standard config/data directory resolution

## Development Notes

- No tests currently exist in the codebase (MVP still in progress)
- The `.gitignore` excludes local data directories under `/data/`
- Default editor is `hx` (helix) but configurable in first-run setup