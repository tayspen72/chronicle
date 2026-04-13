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

## Use Journal Todos

1. Open today's journal from the History sidebar (press Enter)
2. Add items to the "# To do" section using `- [ ] ` or `* [ ] ` syntax
3. View and toggle todos in the "My Tasks" planning report
4. Use `chronicle jot "note"` to add timestamped entries to the "# Notes" section

## Edit Element Metadata

1. Navigate to a program, project, milestone, task, or subtask in the Programs tree.
2. Press `e` to open the metadata editor.
3. Update fields shown by the element template (for example status, ownership, dates, importance, description).
4. Press `Enter` on `CONFIRM` to persist changes to that element's markdown file.
