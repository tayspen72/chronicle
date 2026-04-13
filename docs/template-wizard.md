# Template Wizard

## Overview

Creation is template-driven and templates are embedded from `templates/*.md`.

Template engine support (`create_from_template`):
- `program`
- `project`
- `milestone`
- `task`
- `subtask`
- `journal`
- `note`

Interactive template-field wizard support (`open_template_wizard`):
- `program`
- `project`
- `milestone`
- `task`
- `subtask`
- `note`

Non-interactive template usage:
- `journal` is created from template via the "Open Today's Journal" flow when missing.

## Field Parsing

`parse_template_fields` extracts placeholders from:
- YAML frontmatter lines containing `:` and `{{PLACEHOLDER}}`
- markdown body `{{DESCRIPTION}}` after YAML block

## Auto-Seeded Values

Wizard seeds values for common placeholders:
- `TODAY`
- `OWNER`
- `DEFAULT_STATUS`
- `NAME` (when pre-seeded)
- `IMPORTANCE` (default from config)
- `UUID`
- type-specific aliases (`PROGRAM_NAME`, `PROJECT_NAME`, etc.)

## Write Targets

Wizard target paths are canonical:
- Program: `programs/<name>/<name>.md`
- Project: `programs/<program>/projects/<name>/<name>.md`
- Milestone: `programs/<program>/projects/<project>/milestones/<name>/<name>.md`
- Task: `programs/<program>/projects/<project>/milestones/<milestone>/tasks/<name>/<name>.md`
- Note: `notes/<category>/<name>.md` (or `notes/<category>/<folder>/<name>.md`)
- Journal: `journal/YYYY/MM/YYYY-MM-DD.md`

## Confirmation Flow

On confirm:
1. validate element name
2. resolve target path
3. call `workspace.create_from_template(...)`
4. reload tree view
5. keep user at parent level with new item selected
