# Development Guide

## Build and Test

```bash
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

## Key Modules

- Config: `src/config.rs`
- Diagnostics: `src/diagnostics.rs`
- Storage API: `src/storage/mod.rs`
- TUI State/Event Loop: `src/tui/mod.rs`
- TUI Rendering: `src/tui/layout.rs`, `src/tui/views/mod.rs`
- Commands: `src/commands/*.rs`

## Current Notes

- `src/tui/navigation.rs` and `src/tui/command.rs` include extracted logic, but `App` in `src/tui/mod.rs` still owns the active behavior.
- Storage discovery supports mixed layouts for backwards compatibility.
- Wizard writes should remain canonical unless a migration decision says otherwise.

## When Changing Navigation

Always validate with:
- unit tests in `src/tui/mod.rs`
- mixed layout scenarios
- duplicate-name scenarios
- diagnostics logs for selected path and expanded path transitions

## When Changing Creation Paths

Update in sync:
- `resolve_template_target_path` in `src/tui/mod.rs`
- docs in `README.md` and `docs/workspace-model.md`
- relevant wizard tests in `src/tui/mod.rs`
