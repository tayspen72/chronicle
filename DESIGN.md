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

## Tree Navigation Model

This section defines the canonical behavior for ALL tree-based navigation in the TUI sidebar (Programs, Journal History, Planning).

The intended behavior is that the navigator panel is what the user is controlling and moving around, and the main window is a view port of what is contained in the navigator panel.

### Terminology

- **Expansion**: Making an element's children visible in the sidebar.
- **Collapse**: Hiding an element's children from the sidebar.
- **Selection**: The currently highlighted sidebar item (cursor).
- **Leaf**: An element with no children (or a deeply nested element the user stops expanding at).
- **Section**: Level 1 elements are categorized into logical groups, called sections.

### Rules

1. **Default state**: All elements start collapsed.
2. **Right / `l` — Expand**: Expand the selected element, reveal its children, and move selection to the first child.
   - If the element is already expanded or has no children, the key does nothing.
3. **Left / `h` — Collapse**: Collapse the selected element (hide its children) and move selection to its parent.
   - If the element is at the root level (e.g., "History", "Programs"), the key does nothing.
4. **Expand goes one level deep per press**: Right on a parent shows its immediate children only. It does NOT auto-expand grandchildren.
5. **Collapse hides all descendants**: Left on a collapsed item collapses that entire subtree. Left on a month hides all entries; Left on a year hides all months and entries.
6. **Tree rendering**: `├──` for non-last children, `└──` for the last child, `│` for continuation lines of expanded siblings above, `  ` for no-continuation lines. Never show `│` at the root level of a tree section.
7. **Section Name**: Section names are not interact-able. The user cannot select them and thus should not display anything or expand/contract.

### Example States

In the example states below:
**_Section<x>_**: A section name, used to categorize level 1 elements
**Alpha<x>**: Level 1 element, unique designator <x>
**Beta<x>**: Level 2 element, unique designator <x>
**Gamma<x>**: Level 3 element, unique designator <x>
**Delta<x>**: Level 4 element, unique designator <x>

**State 1 — All collapsed (initial)**
```
Alpha
```

**State 2 — Level 1 Element Expanded, Single Child**
```
Alpha
   └── Beta
```
Pressing Right-Arrow on Aplha will expand to show Beta
  * A single entry child, display: `└──`

**State 3 — Level 1 Element Expanded, Single Child, Collapsed Level 1 Element**
```
Alpha1
   └── Beta
Alpha2
```
Note the first level entries are not linked with a vertical pipe
  
**State 4 — Level 2 Expanded**
```
Alpha
   ├── Beta1
   │   ├── Gamma1
   │   ├── Gamma2
   │   └── Gamma3
   └── Beta2
```
Pressing Right-Arrow on a level 1 entry will expand to show child elements. Note the vertical connection between Beta1 and Beta2

**State 5 — Level 3 Expanded**
```
Alpha
   ├── Beta1
   │   ├── Gamma1
   │   ├── Gamma2
   │   │   ├── Delta1
   │   │   ├── Delta2
   │   │   └── Delta3
   │   └── Gamma3
   └── Beta2
```
Expansion can continue to the right in this manner so long as the data type has child nodes available

**State 6 — Level 4 Collapse**
```
Alpha
   ├── Beta1
   │   ├── Gamma1
   │   ├── Gamma2
   │   │   ├── Delta1
   │   │   ├── Delta2
   │   │   └── Delta3
   │   └── Gamma3
   └── Beta2
```
Assume this tree view and selection is on Delta1, and the user presses left-arrow
```
Alpha
   ├── Beta1
   │   ├── Gamma1
   │   ├── Gamma2
   │   └── Gamma3
   └── Beta2
```
Pressing left-arrow will collapse the all sibling elements under the parent node, and the selection will move to the parent node
Gamma2 is now selected

**State 7 — Tree view persistent when not collapsed**
```
Alpha
   ├── Beta1
   │   ├── Gamma1
   │   ├── Gamma2
   │   │   ├── Delta1
   │   │   ├── Delta2
   │   │   └── Delta3
   │   └── Gamma3
   └── Beta2
```
Assume the user is currently on Delta3 and presses down arrow.

The child nodes under Gamma2 do *not* collapse, but persist until navigation returns to a sibling node under Gamma2 and the user pressed left arrow

If the user then has Gamma3 selected and pressed right arrow:
```
Alpha
   ├── Beta1
   │   ├── Gamma1
   │   ├── Gamma2
   │   │   ├── Delta1
   │   │   ├── Delta2
   │   │   └── Delta3
   │   └── Gamma3
   │       ├── Delta4
   │       ├── Delta5
   │       └── Delta6
   └── Beta2
```
The tree under Gamma2 has not been collapsed (with left arrow) and the user expands Gamma3 (with right arrow)

**State 8 — Collapse All Elements In Section With Cross-Section Navigation**
```
_Section1_
Alpha1
   ├── Beta1
   │   ├── Gamma1
   │   ├── Gamma2
   │   │   ├── Delta1
   │   │   ├── Delta2
   │   │   └── Delta3
   │   └── Gamma3
   │       ├── Delta4
   │       ├── Delta5
   │       └── Delta6
   └── Beta2
Alpha2
Alpha3

_Section2_
Alpha4
Alpha5
```
Assume the user has Alpha3 selected (by navigation with up/down arrow) and has not collapsed any levels

Pressing down arrow on Alpha 3, thus changing from elements in Section1 to Section2, will collapse all elements in Section1
```
_Section1_
Alpha1
Alpha2
Alpha3

_Section2_
Alpha4
Alpha5
```
Selection is not on Alpha4 (Section headers are not select-able)

---

## Current Implementation Status

### Working Features

- Command palette (`/` opens, typing filters, Up/Down navigates, Enter executes)
- Tree navigation: arrow keys, expand/collapse, 4-level hierarchy
- Navigator panel unified behavior: Programs and Journal History now follow the same Tree Navigation Model semantics for expand/collapse/selection and tree rendering
- Element creation: template-based wizard for Programs/Projects/Milestones/Tasks
- Journal: today's journal, history browser with year/month tree expansion
- External editor: launches configured editor, restores TUI
- Planning sessions: weekly planning with task selection and preview
- Keyboard shortcuts: hjkl, Tab, Escape, Enter

### Open Issues

1. **Dead code**: `commands/` directory (CLI commands) is disconnected from TUI
2. **Journal folder structure**:
   - `journal/current/YYYY-MM-DD.md` — today's journal entry
   - `journal/history/YYYY/MMM/YYYY-MM-DD.md` — history entries grouped by year and 3-letter month (e.g., `journal/history/2026/mar/2026-03-18.md`)
3. **Current Plan Enter key**: Pressing Enter on "Current Plan" shows all tasks in the system instead of a useful view. Should either route to start a new planning session or be removed entirely

### Creation Wizard

Fields should come from a mix of what the software is creating and dynamic parsed fields from the template files. Field names should be bold, field values should be white. Three field value types: auto-filled (eg uuid, creation date), suggested (eg start date in new planning session) and empty (eg title). auto-filled are non adjustable, suggested will start with a value but the user can select that field and edit the value, and empty should read "empty" until the user adjusts it. When stored in the resulting object file after creation, empty should not be transferred. Field name should be bold, field value should be white if fixed or adjusted, gray if suggested or empty until edited.

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

Uses the universal **Tree Navigation Model** defined above.

```
Journal sidebar:
  └─ History
       └─ Year (depth 1)
            └─ Month (depth 2)
                 └─ Entry (depth 3)  →  Enter opens in viewer
```

Each depth follows the same expand/collapse/selection rules from the Tree Navigation Model.

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
| 2026-03-22 | Navigator panel and unified tree structure completed: Programs and Journal History now share consistent tree semantics (expand/collapse/selection/rendering) |
| 2026-03-21 | Spec: Added Tree Navigation Model section with 8 example states |
| 2026-03-21 | Bug fix: Journal history tree connector chars and indentation |
| 2026-03-21 | Bug fix: Journal history collapse cascades correctly |
| 2026-03-19 | Bug fix: Today journal opens in external editor (Enter key) |
| 2026-03-19 | Bug fix: Programs remain visible when History is expanded or collapsed |
| 2026-03-19 | Bug fix: Esc dismisses Current Plan view and returns to TreeView |
| 2026-03-19 | Partial fix: Journal history expansion shows hierarchy; collapse behavior still broken |
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

### Next Recommended Feature: Current Plan Report

Replace the current "Current Plan" behavior with a dedicated, useful report in the main window:
- Group tasks by Program → Project → Milestone
- Show task status, assignee, priority, start/due dates
- Support quick filtering (e.g., by status, assignee)
- Keep sidebar behavior unchanged (main window remains a viewport of selection)

This resolves an active open issue and gives immediate user-facing value.

### Creation Wizard UX

Refactor all element creation wizards (Program, Project, Milestone, Task, Subtask) to use consistent field handling:
- Field names bold, field values white (fixed/adjusted) or gray (suggested/empty)
- Three field types: auto-filled (non-editable, gray), suggested (editable, gray until changed), empty (shows "empty", not stored)
- This applies to all wizards: element creation, planning session, etc.

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
