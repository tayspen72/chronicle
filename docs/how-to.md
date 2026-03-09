# Chronicle How-To

This quick guide is a practical companion to the full docs index at [docs/README.md](./README.md).

## Create a Program

1. Launch Chronicle.
2. Press `/`.
3. Run `New Program`.
4. Fill wizard fields and confirm.

Output path:
- `programs/<Program>/<Program>.md`

## Create Nested Elements

Use command palette actions:
- `New Project`
- `New Milestone`
- `New Task`

Chronicle uses current tree context and writes canonical nested paths.

## Navigate Tree Quickly

- `Right`: expand and move into first child in one keypress.
- `Left`: collapse and move to parent in one keypress.
- `Enter`: open selected leaf content.

## Enable Diagnostics

In `~/.config/chronicle/config.toml`:

```toml
[diagnostics]
enabled = true
level = "debug"
```

Log file:
- `~/.config/chronicle/logs/diagnostics.log`
