# Architecture

## High-Level Components

- `src/main.rs`: binary entrypoint
- `src/lib.rs`: crate module exports
- `src/error.rs`: layered error types
- `src/config.rs`: config schema and loading
- `src/diagnostics.rs`: file-based tracing setup
- `src/storage/`: filesystem discovery + template write APIs
- `src/model/`: domain structs and element enums
- `src/tui/`: application state, event handling, rendering
- `src/commands/`: non-TUI command helpers (init/jot/new_task/extract)

## Runtime Flow

1. `Config::load_or_create()` loads config.
2. `diagnostics::init()` optionally enables file logging.
3. `tui::App::new(config)` initializes UI state and tree data.
4. `App::run()` starts event loop:
- process key events
- update app state
- redraw UI

## State Ownership

The `App` struct in `src/tui/mod.rs` owns:
- current view and mode
- selected and expanded tree paths
- sidebar list items
- command palette state
- template wizard state
- content preview/editor launch state

## Error Strategy

- Library code returns `crate::Result<T>`.
- `main.rs` maps library errors into `anyhow::Result` at the boundary.
