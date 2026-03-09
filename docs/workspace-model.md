# Workspace Model

## Canonical Structure

Chronicle now writes new elements in this canonical nested layout:

```text
workspace/
├── programs/
│   └── <program>/<program>.md
│       └── projects/<project>/<project>.md
│           └── milestones/<milestone>/<milestone>.md
│               └── tasks/<task>/<task>.md
│                   └── subtasks/<subtask>/<subtask>.md
├── planning/
│   ├── current/
│   └── history/
├── journal/
│   └── YYYY-MM-DD.md
└── .archive/
```

## Discovery Tolerance

The storage layer still discovers mixed/legacy layouts for compatibility:
- flat files (e.g. `<node>.md`)
- nested directories with same-name markdown
- container folders (`projects`, `milestones`, `tasks`, `subtasks`)

This tolerance is primarily for reading existing workspaces. New wizard-created elements follow canonical paths.

## Naming Constraints

Element names are validated by `validate_element_name`:
- non-empty
- no `/` or `\\`
- no control chars
- no `..`

## Path Safety

Template writes are constrained to remain inside workspace root.
