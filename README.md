# Chronicle

Chronicle is a Markdown-native planner and journal with a terminal UI.
It manages a hierarchical workspace (`programs -> projects -> milestones -> tasks -> subtasks`) plus planning and journal flows.

## Current capabilities
- Navigate the workspace tree in a TUI.
- Render selected program/project/milestone/task content as: YAML details table, markdown body, and child status/count report.
- Open Markdown files from the tree in your configured editor.
- Create programs/projects/milestones/tasks through a template-driven wizard.
- Open today’s journal and browse journal history.
- Run planning reports from the sidebar/command palette:
  - Backlog (unassigned tasks in current plan, with quick assign-to-me)
  - My Tasks (tasks assigned to you + today’s `# To do` checklist with checkbox toggles)

## Build and run
```bash
cargo build
cargo run
```

## CLI commands
```bash
cargo run -- init
cargo run -- jot "Wrote release notes"
cargo run -- extract
cargo run -- new-task "Plan onboarding" --scope planning/current
```

## Quality checks
```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

## Configuration
Chronicle loads config from:
- `~/.config/chronicle/config.toml`

Required fields (must be present and non-empty):
- `workspace`: root directory for all Chronicle data
- `editor`: external editor command (for example `hx`, `vim`, `helix`)
- `owner`: your identity for created/assigned task workflows

Optional fields (system defaults are used when omitted):
- `workflow`: ordered status list (default: `["New","Active","Blocked","Testing","Completed","Cancelled"]`)
- `importance`: task importance options (default: `["low","medium","high"]`)
- `planning_duration`: one of `weekly|biweekly|6weekly` (default: `weekly`)
- `diagnostics.enabled`: enable file diagnostics logging (default: `false`)
- `diagnostics.level`: `trace|debug|info|warn|error` (default: `debug`)

Removed/unused fields:
- `navigator_width` and `[navigation_keys]` are no longer used by the app.

Example `config.toml`:
```toml
workspace = "/home/user/chronicle"
editor = "hx"
owner = "Your Name"

workflow = ["New", "Active", "Blocked", "Testing", "Completed", "Cancelled"]
importance = ["low", "medium", "high"]
planning_duration = "weekly"

[diagnostics]
enabled = false
level = "debug"
```

Example diagnostics config:

```toml
[diagnostics]
enabled = true
level = "debug"
```

## Workspace layout
Canonical layout under `workspace`:

```text
workspace/
├── programs/
│   └── {program}/{program}.md
│       └── projects/{project}/{project}.md
│           └── milestones/{milestone}/{milestone}.md
│               └── tasks/{task}/{task}.md
│                   └── subtasks/{subtask}/{subtask}.md
├── planning/
│   ├── current/
│   └── history/
├── journal/
│   └── YYYY/MM/YYYY-MM-DD.md
├── .archive/
└── templates/
```

The storage layer tolerates mixed/legacy layouts for discovery, but the TUI wizard now writes new elements in canonical nested paths above.

## TUI navigation
- `/`: open command palette
- `Esc`: close palette / back out of wizard focus
- `Up` / `Down`: move selection
- `Left` / `Right`: collapse/expand levels
- `Enter`: activate selection
- `Tab`: move wizard focus

## Template behavior
Element creation uses Markdown templates with YAML frontmatter placeholders, including:
- `{{UUID}}`, `{{NAME}}`, `{{DEFAULT_STATUS}}`, `{{TODAY}}`, `{{OWNER}}`, `{{DESCRIPTION}}`

Template values are resolved centrally by storage APIs when creating files.

## Diagnostics logging
Chronicle can emit detailed diagnostics to a file while the TUI is running.

- Default log path (when enabled): `~/.config/chronicle/logs/diagnostics.log`
- Enable via config: `[diagnostics].enabled = true`
- Or enable ad-hoc with environment variables:
  - `CHRONICLE_DIAGNOSTICS=1`
  - `CHRONICLE_DIAGNOSTICS_LEVEL=debug`
  - `CHRONICLE_DIAGNOSTICS_LOG=/custom/path/chronicle.log`

Diagnostics include:
- workspace snapshot at startup
- tree navigation path/selection transitions
- hierarchy discovery and dedupe decisions
- template parsing and create-from-template operations
- warnings/errors already emitted through `tracing`

## Command modules
The command modules (`init`, `jot`, `new-task`, `extract`) are wired through the main binary and use `Config.workspace` instead of hardcoded `data/` paths.

- `init`: creates workspace directories
- `jot`: appends a timestamped entry to today’s journal
- `new-task`: writes a templated task into planning/current (or scoped path)
- `extract`: scans journal `/todo` lines into planning/current backlog files

## Notes
- Library code uses layered `thiserror` error types.
- Binary entry (`main.rs`) uses `anyhow` at the boundary.
- All tests should pass before merging changes.
