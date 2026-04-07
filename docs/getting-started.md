# Getting Started

## Build and Run

```bash
cargo build
cargo run
```

Chronicle launches a terminal UI (TUI).

## First Run

On first run, Chronicle creates `~/.config/chronicle/config.toml`.

You will be prompted for:
- workspace location
- editor command
- owner name

## Basic Usage

- Press `/` to open command palette.
- Use arrow keys to navigate.
- Press `Enter` to activate selected item.
- Use `Right`/`Left` to move down/up the tree.
- Access today's journal via the History sidebar section
- View and interact with todo items in the "My Tasks" planning report

## Quality Checks

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```
