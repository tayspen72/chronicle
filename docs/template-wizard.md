# Template Wizard

## Overview

Wizard creation is template-driven.
Templates are embedded from `templates/*.md`.

Supported template names:
- `program`
- `project`
- `milestone`
- `task`

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
- type-specific aliases (`PROGRAM_NAME`, `PROJECT_NAME`, etc.)

## Write Targets

Wizard target paths are canonical:
- Program: `programs/<name>/<name>.md`
- Project: `programs/<program>/projects/<name>/<name>.md`
- Milestone: `programs/<program>/projects/<project>/milestones/<name>/<name>.md`
- Task: `programs/<program>/projects/<project>/milestones/<milestone>/tasks/<name>/<name>.md`

## Confirmation Flow

On confirm:
1. validate element name
2. resolve target path
3. call `workspace.create_from_template(...)`
4. reload tree view
5. keep user at parent level with new item selected
