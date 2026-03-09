# TUI Navigation

## Core Model

Navigation uses two separate states:
- selected node path (`selected_tree_path`)
- expanded node paths (`expanded_tree_paths`)

This avoids coupling selection and expansion into one global path.

## Tree Key Semantics

In tree view:
- `Right`:
  - if selected node has children: expand node and move selection to first child in one action
  - if selected node has no children and is newly selected context: enter/select node
  - if selected node has no children and is already selected: open content
- `Left`:
  - collapse current selected branch
  - move selection to parent in one action

This behavior applies at all depths (program/project/milestone/task/subtask).

## Other Keys

- `/`: command palette
- `Esc`: close palette; in tree root, returns to Journal view
- `Up` / `Down`: move selection
- `Enter`: activate selected item
- `Tab`: advance wizard focus

## Sidebar Sections

Sidebar always includes:
- Programs tree
- Planning (`Weekly Planning`, `Backlog`)
- Journal (`Today`, `History`)
