# Chronicle Design Document

## Overview

Chronicle is a Markdown-native planner, journal, and note-taking tool with a terminal UI (TUI). It uses a hierarchical folder structure (`programs/ → projects/ → milestones/ → tasks/`) plus `journal/`, `planning/`, and `notes/` directories. Notes follow the PARA methodology (Projects, Areas, Resources, Archive).

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
│   ├── mod.rs        # JournalStorage, WorkspaceStorage, NotesStorage traits
│   ├── md.rs         # Markdown parsing, template resolution
│   └── planning.rs   # Planning session persistence
└── tui/
    ├── mod.rs        # App state and event loop
    ├── cache.rs      # TreeData, JournalNode, NoteNode structs
    ├── layout.rs     # Layout orchestration
    ├── navigation.rs # JournalTreeState, NotesTreeState, SidebarItem, tree helpers
    ├── command.rs    # Command palette types
    └── views/
        └── mod.rs    # All view render functions
templates/
├── note.md           # Note template (UUID, title, creation_date, created_by, tags)
└── ...               # Other element templates
```

### Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `App` | tui/mod.rs | Main TUI state, event loop, key handling |
| `Mode` | tui/mod.rs | Normal, CommandPalette, Input |
| `ViewType` | tui/mod.rs | All view variants |
| `Config` | config.rs | Workspace, editor, workflow, keys |
| `NotesConfig` | config.rs | PARA category names (overridable) |
| `TreeData` | tui/cache.rs | Cached tree vectors (programs/projects/milestones/tasks/subtasks) |
| `JournalNode` | tui/cache.rs | Journal tree node variants (Year, Month, Entry) |
| `NoteNode` | tui/cache.rs | Notes tree node variants (Category, Folder, Entry) |
| `JournalTreeState` | tui/navigation.rs | Tracks journal history expansion state |
| `NotesTreeState` | tui/navigation.rs | Tracks PARA notes tree data and expansion state |
| `SidebarNodeData` | tui/navigation.rs | Enum discriminator for sidebar item type dispatch |
| `Task` | model/mod.rs | Task with title, status, priority, dates, description |
| `PlanningSessionState` | tui/mod.rs | Active planning session data |
| `NoteCreationState` | tui/mod.rs | Two-step note wizard state (category → folder → template) |
| `MoveNoteState` | tui/mod.rs | Move-note picker state with filterable destination list |

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

- **Command palette loses category grouping while typing**: When the user types after pressing `/`, the match list switches to a flat list with `"Category: Label"` display labels (e.g., `"Commands: New Task"`) instead of keeping commands grouped under their category headers. Categories should be preserved as section headers even while filtering — only the matching commands within each category should be shown.
- **`CommandCategory::Commands` label is misleading**: The "Commands" section header groups creation actions (New Program, New Project, New Milestone, New Task, New Subtask). This category should be renamed to "Task Management" in both `CommandCategory::label()` and `grouped_commands()` ordering to match domain intent.
- **Sidebar top section label "Programs" should be "Task Management"**: The root sidebar section that holds programs/projects/milestones/tasks is labelled "Programs". It should be renamed to "Task Management" to reflect its broader scope.
- **New Note wizard ignores existing subfolders**: After selecting a PARA category, the wizard prompts for a free-text folder name but does not scan for existing subfolders. It should display a selectable list — `(none)` at the top, followed by existing subfolder names — so the user can pick a destination rather than type one. This follows the UX pattern of the rest of the creation wizard (selectable list, not free text).
- **Main UI shows stray "Commands: /" label at top**: A `Commands: /` label appears in the main window header. It should be removed.
- **Notes tree renders spurious `│` connector on single-subfolder entries**: Note entries inside a subfolder are prefixed with a vertical pipe (`│`) even when there is only one subfolder at that level (i.e., no sibling to connect to). The connector characters must follow the Tree Navigation Model rendering rules: `│` is only drawn when a prior sibling at the same depth is still expanded above the current item.
- **Collapse does not cascade up to parent**: When the user selects a node and collapses it (Left / `h`), only the direct children are hidden — the node itself does not collapse into its parent. The intended behavior is a two-phase collapse: (1) the selected node's children fold into it, then (2) the node itself folds into its parent along with all siblings at the same level. After collapse, the selection must land on the parent of the node that was collapsed. This applies to all tree levels and all tree sections (Task Management, Journal, Notes). Example: if a milestone is expanded and selected, pressing Left should hide the tasks under it **and** collapse the milestone back into its parent project, leaving the project selected.

### Creation Wizard

The creation UX must be consistent across:
- Element creation wizard (program/project/milestone/task)
- Planning session date wizard
- Add task to plan flow (tree picker + add-to-plan task editor)

#### Field source and ordering

- Wizard rows come from YAML frontmatter fields in the template file.
- Rows are displayed in the same order as fields appear in the template file.
- Row labels are generated from YAML keys and converted to title case.
- All template YAML fields are shown in the wizard.
- The old "Creating in:" row is removed.
- The wizard top line shows the target breadcrumb path (example: `Program -> Project -> Milestone -> new task`) instead of a generic prompt like "Fill in fields for ...".

#### Field visual states

- Field names are bold.
- Non-editable rows are not focusable.
- Gray value text means the current value is system-generated or suggested and has not been user-edited yet.
- White value text means the value was edited by the user.
- Empty editable fields show gray `empty` until edited.

#### Field value categories

- Auto-filled: system-generated, non-editable, gray until written.
- Suggested: editable; starts gray, turns white after user edit.
- Empty: editable; shows gray `empty`, turns white after user edit.
- Fixed: from template literal values, non-editable.
- Choice: list-backed editable values.

#### Choice field behavior

- `status` options come from workflow values in config.
- `importance` options come from config (new list).
- Left/Right changes the option only when focus is on a choice field.
- Enter always moves to the next row (does not cycle choices).
- A right-side suggestion/options list is shown only when a choice field is focused.

#### Description handling

- `# Description` remains in markdown body (outside YAML).
- `{{DESCRIPTION}}` is replaced by wizard input and may be empty.
- Description content is rendered as markdown body content, not moved into YAML.

#### Navigation contract for creation wizards

- Up/Down moves between focusable rows.
- Enter moves to the next row.
- Enter on the last focusable row moves focus to `CONFIRM`.
- Escape once jumps focus to `CANCEL`.
- Escape again cancels (same as Enter on `CANCEL`).
- Left/Right is no-op on non-choice fields.
- Bottom action rows:
  - Standard: `CONFIRM` / `CANCEL`
  - Preview: `ADD TASKS TO PLAN` / `CONFIRM` / `CANCEL`
  - Left/Right cycles within the bottom action row.

#### Add task to plan behavior

- Keep tree navigation model for program/project/milestone/task traversal.
- Remove the extra gray first-program line in the header area.
- Remove the redundant "Programs (...)" line and ascii folder/file noise.
- Space has no behavior in add-task picker.
- Enter on a task opens the add-to-plan task editor wizard.
- In add-to-plan editor, user can edit YAML-backed task fields before adding.
- Confirm adds the task to the planning session and checks the task in picker state.
- Edits are persisted to the task markdown file (single source of truth).
- Planning session file stores only minimal references needed to build reports from source task files (no duplicate task metadata).

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
  └─ Wizard (duration, dates)
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

### Notes Navigation (PARA)

Uses the universal **Tree Navigation Model** defined above. Notes are stored at `{workspace}/notes/{category}/{optional-folder}/{note}.md`.

```
Notes sidebar:
  └─ Category (depth 1: Projects / Areas / Resources / Archive)
       └─ Folder (depth 2, optional subfolder)
            └─ Note entry (depth 3)  →  Enter opens in editor
```

- Category names are configurable via `notes.categories` in `config.toml` (defaults to PARA names).
- Notes can be at depth 2 (directly in category) or depth 3 (inside a subfolder).
- **New Note** command (`/new note`): wizard selects category → optionally enters folder name → template wizard fills title/description.
- **Move Note** command (`/move note`): filterable picker lists all categories and subfolders; selecting a destination moves the file on disk.
- Cross-section collapse: navigating away from the Notes section collapses all Notes expansions.

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
| `Space` | Add task picker | No action |
| `Enter` | Add task picker (task row) | Open add-to-plan task editor |
| `f` | Task picker | Show preview screen |
| Tab | Wizard | Next field |
| `Shift+Tab` | Wizard | Previous field |
| `j` / `↓` | Note category wizard | Move selection down |
| `k` / `↑` | Note category wizard | Move selection up |
| `Enter` | Note category wizard | Confirm selected category |
| `j` / `↓` | Move note picker | Move selection down |
| `k` / `↑` | Move note picker | Move selection up |
| `Enter` | Move note picker | Confirm move destination |

---

## Changelog

| Date | Event |
|------|-------|
| 2026-04-09 | Feature: Notes section with PARA methodology (Projects, Areas, Resources, Archive) — expand/collapse tree navigation consistent with Journal History |
| 2026-04-09 | Feature: `notes.categories` config key for overriding default PARA folder names |
| 2026-04-09 | Feature: New Note command — two-step wizard (category → folder) feeds into existing template wizard |
| 2026-04-09 | Feature: Move Note command — filterable picker lists all category/folder destinations; moves file on disk |
| 2026-04-09 | Feature: `NotesStorage` trait on `PathBuf` with `notes_dir()`, `scan_notes()`, `move_note()` |
| 2026-04-09 | Refactor: `SidebarNodeData` enum added for type-safe sidebar item dispatch (Notes vs Journal vs Programs) |
| 2026-04-09 | Refactor: `NoteNode` enum in cache.rs mirrors `JournalNode` for tree rendering |
| 2026-04-09 | Integration tests: 190 total tests (31 navigation, 9 command palette) |
| 2026-03-24 | Feature: Programs tree selection now renders a combined element view in the main window: YAML details table, markdown body content, and child status/count report |
| 2026-03-24 | UX tweak: Empty child-report state now shows only "No <child> detected" without repeating selected element status |
| 2026-03-24 | Feature: Binary now routes `init`, `jot`, `extract`, and `new-task` CLI commands through `src/commands`, removing the disconnected/dead-code path |
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

### Planning Reports

1. **Preview Plan report**: The preview screen shown when pressing `f` should use organized tables instead of plain text strings. Group tasks by Program/Project/Milestone hierarchy for readability.
2. **Notebook Expansion**: The tool should expand notebook capability to allow for general notetaking. Specifically I want to encorporate a PARA notebook style, and research the potential for adding more.
3. **Error Handling**: The user should be notified of errors. eg when launching the preferred editor, file naming conflicts/overwriting files, duplicate task names, etc.

### Bottom Panel and App Branding

The status bar and app identity need a coordinated overhaul:

**Bottom-left breadcrumb**: Show the current location with the top-level navigation category as the root segment, not just the raw tree path. Format: `Category > Level1 > Level2 > ...` where Category is one of: `Task Management`, `Journal`, `Planning`, `Notes`. Example: `Task Management > Acme Corp > Q2 Launch > Sprint 1`. The category name should be styled (bold or accent-colored) to anchor the user's context.

**App name ("chronicle")**: Display `chronicle` in the **top pane header** — left-aligned or centered — styled with the theme's accent color. This is app identity, not navigation context, so it belongs in the header rather than the status bar. The bottom bar should stay focused purely on location.

**Bottom middle / right**: Open for future use (e.g., mode indicator, clock, key hints).

### Theming

Support theme files (e.g., from Helix editor's `themes/` directory or Alacritty). Users should be able to select from a list of themes to customize the TUI color scheme. The `ayu_evolve` theme from Helix can be used as a reference or directly ported.
