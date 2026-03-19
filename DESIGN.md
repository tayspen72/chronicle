# Chronicle Design Document

## Overview

Chronicle is a Markdown-native planner and journal with a terminal UI (TUI). It uses a hierarchical folder structure (`programs/ → projects/ → milestones/ → tasks/`) plus `journal/` and `planning/` directories.

---

## Architecture

### Module Map

```
src/
├── main.rs           # Entry: Config::load_or_create() → App::new().run()
├── lib.rs            # Crate root with module exports
├── config.rs        # Config loading from ~/.config/chronicle/config.toml
├── error.rs         # Layered error types (thiserror)
├── model/
│   └── mod.rs        # Domain types: Task, Program, Project, Milestone + parse_element()
├── storage/
│   ├── mod.rs        # JournalStorage, WorkspaceStorage traits
│   ├── md.rs         # Markdown parsing, template resolution
│   └── planning.rs   # Planning session persistence
└── tui/
    ├── mod.rs        # App state and event loop (~1500 lines)
    ├── cache.rs      # TreeData struct (programs/projects/milestones/tasks/subtasks)
    ├── layout.rs     # Layout orchestration
    ├── navigation.rs # JournalTreeState, SidebarItem, tree helpers
    ├── command.rs    # Command palette types
    └── views/
        └── mod.rs    # All view render functions
```

### Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `App` | tui/mod.rs | Main TUI state, event loop, key handling |
| `Mode` | tui/mod.rs | Normal, CommandPalette, Input |
| `ViewType` | tui/mod.rs | All view variants |
| `Config` | config.rs | Workspace, editor, workflow, keys |
| `TreeData` | tui/cache.rs | Cached tree vectors (programs/projects/milestones/tasks/subtasks) |
| `JournalTreeState` | tui/navigation.rs | Tracks journal history expansion state |
| `Task` | model/mod.rs | Task with title, status, priority, dates, description |
| `PlanningSessionState` | tui/mod.rs | Active planning session data |

---

## Current Implementation Status

### Working Features

- Command palette (`/` opens, typing filters, Up/Down navigates, Enter executes)
- Tree navigation: arrow keys, expand/collapse, 4-level hierarchy
- Element creation: template-based wizard for Programs/Projects/Milestones/Tasks
- Journal: today's journal, history browser with year/month tree expansion
- External editor: launches configured editor, restores TUI
- Planning sessions: weekly planning with task selection and preview
- Keyboard shortcuts: hjkl, Tab, Escape, Enter

### Open Issues

1. **Tree vertical pipes**: Tree view lacks visual `│` connecting sibling elements
2. **Navigation edge cases**: Selection may jump unexpectedly during expand/collapse + element creation
3. **Dead code**: `commands/` directory (CLI commands) is disconnected from TUI

### Journal History Navigation Bugs (TODO)

1. **Today not opening editor**: Pressing Enter on "Today" has no response — should open today's journal in external editor
2. **Programs disappear when History expands**: Navigating Right on "History" causes all programs to disappear from sidebar and become inaccessible
3. **History selection should move to child**: Right on History/Year/Month should select the newly revealed child, not stay on parent (match programs behavior)
4. **Cannot collapse month level**: After opening the final tier (months) in history tree, there is no way to close it

### Planning Navigation Bug (TODO)

1. **Current Plan view cannot be closed**: When a planning session exists and user presses Enter on "Current Plan", the view shows useful content but cannot be dismissed without a command (e.g., `/programs`). Esc should return to the previous view or tree root.

---

## Data Flow

### Application Startup

```
main.rs
  └─ Config::load_or_create()
       └─ App::new()
            └─ load_tree_view_data()  →  App::run()
                                              └─ Event Loop
                                                   ├─ CommandPalette  →  filter_commands()
                                                   └─ Normal  →  handle_key()
```

### Planning Session Flow

```
Start Planning Session
  └─ Wizard (name, duration, dates)
       └─ Task Picker (Programs → Projects → Milestones → Tasks)
            └─ Enter on task  →  Task Detail Wizard
                 └─ ADD TO PLAN  →  return to picker
                      └─ f  →  Preview Screen
                           ├─ ADD TASKS TO PLAN  →  return to picker
                           ├─ CONFIRM  →  save session, return to Normal
                           └─ CANCEL  →  discard, return to Normal
```

### Journal History Navigation

```
Journal sidebar:
  ├─ Today  →  opens today's journal in viewer
  └─ History
       ├─ Right  →  expands years
       │    ├─ Right on year  →  expands months
       │    │    └─ Right on month  →  shows entries
       │    │         └─ Enter on entry  →  opens in viewer
       │    └─ Left  →  collapses to parent level
       └─ Left on History  →  collapses entire history tree
```

---

## Keyboard Reference

| Key | Context | Action |
|-----|---------|--------|
| `j` / `↓` | Normal | Navigate down in sidebar |
| `k` / `↑` | Normal | Navigate up in sidebar |
| `l` / `→` | Normal | Expand item / enter subdirectory |
| `h` / `←` | Normal | Collapse parent / go back |
| `Enter` | Normal | Open item / confirm action |
| `Esc` | Any | Cancel / go back |
| `/` | Normal | Open command palette |
| `Space` | Task picker | Open task detail wizard |
| `f` | Task picker | Show preview screen |
| Tab | Wizard | Next field |
| `Shift+Tab` | Wizard | Previous field |

---

## Changelog

| Date | Event |
|------|-------|
| 2026-03-18 | Feature: Journal history tree structure (grouped by year/month) |
| 2026-03-18 | Feature: Journal history navigates in sidebar (Right expands, Left collapses, Enter opens) |
| 2026-03-18 | Refactor: TreeData extracted to cache.rs — 5 fields consolidated into `app.tree_data` |
| 2026-03-18 | Refactor: PlanningError integrated into Error hierarchy, removed redundant Io/Yaml variants |
| 2026-03-18 | Bug fix: Task description no longer shows duplicate "# Description" header |
| 2026-03-18 | Integration tests: 18 navigation and command palette tests added (142 total tests) |
| 2026-03-17 | Feature: Planning Preview screen with ADD TASKS TO PLAN, CONFIRM, CANCEL buttons |
| 2026-03-17 | Refactor: Removed Review mode — Preview directly saves on CONFIRM |
| 2026-03-17 | Bug fix: Task detail wizard properly updates task file on disk |
| 2026-03-17 | Bug fix: Enter navigates wizard fields, Escape jumps to Cancel |
| 2026-03-06 | Bug fix: Navigator stays at parent level after element creation |
| 2026-03-06 | Feature: `/refresh` command to reload tree from disk |
| 2026-03-05 | Bug fix: Element creation broken — `target_path` was never set |
| 2026-03-05 | Bug fix: Navigator not refreshing after element creation |
| 2026-03-04 | Feature: Status bar with breadcrumb and mode indicator |
| 2026-03-04 | Feature: Backlog and WeeklyPlanning views |
| 2026-03-04 | Feature: Domain model (Program, Project, Milestone, Task structs + parse_element) |
| 2026-03-04 | Refactor: Error types layered with thiserror, lib.rs crate root |
| 2026-03-04 | Refactor: View functions extracted to views/mod.rs |
| 2026-03-03 | Bug fix: Navigate left selects parent instead of header |
| 2026-03-03 | Bug fix: First program selected on load, Right moves to first child |
| 2026-03-03 | Bug fix: Storage discovery handles flat and nested structures |
| 2026-03-03 | Bug fix: TOML config parsing fixed |

---

## Future Features

### Program/Element Reports

Add a report view for each element type in the Programs section that shows children with their status:

- **Program report**: lists all projects under it with status (e.g., "Alpha: Project 1 [Active], Project 2 [Blocked], Project 3 [New]")
- **Project report**: lists milestones under it with status
- **Milestone report**: lists tasks under it with status
- **Task report**: lists subtasks under it with status

Reports appear in the main window when the element is selected in the sidebar (matching the viewport principle).

### Planning Reports

1. **Current Plan report**: When a planning session exists, Enter on "Current Plan" shows a structured report of tasks in the session. Tasks should be grouped and separated by Program → Project → Milestone, with task details (name, status, dates, priority) shown in a readable table format.

2. **Preview Plan report**: The preview screen shown when pressing `f` should use organized tables instead of plain text strings. Group tasks by Program/Project/Milestone hierarchy for readability.

3. **Backlog report**: Currently shows all tasks. Should be refined to show only tasks in the current plan that are in the "New" state and assigned to the current user.

4. **My Tasks**: Add a new entry under the Planning section that shows a useful report of all tasks assigned to the current user that are not in the "New" state (i.e., Active, Blocked, Testing, etc.).

### Theming

Support theme files (e.g., from Helix editor's `themes/` directory or Alacritty). Users should be able to select from a list of themes to customize the TUI color scheme. The `ayu_evolve` theme from Helix can be used as a reference or directly ported.
