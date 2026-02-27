# Chronicle How-To Guide

## Creating New Items (Programs, Projects, Milestones, Tasks)

When you create a new item in the TUI, you'll be prompted to fill in template fields. Some fields are auto-populated, others require input.

### Auto-Populated Fields

These placeholders are automatically filled and won't prompt you:

| Placeholder | Value |
|-------------|-------|
| `{{NAME}}` | The name you entered for the item |
| `{{TODAY}}` | Current date (YYYY-MM-DD) |
| `{{DEFAULT_STATUS}}` | First entry from your workflow config |

### Manual Input Fields

All other placeholders will prompt you for input. For example, when creating a task you'll be prompted for:
- Description
- Due Date
- Assigned To
- etc.

## Template Syntax

### Field Types

There are two types of fields in templates:

1. **Labeled fields (keep label)**:
   ```
   #Title: {{NAME}}
   #Status: {{DEFAULT_STATUS}}
   ```
   Output: `Title: My Task Name`

2. **Content fields (strip label)**:
   ```
   #Description! {{DESCRIPTION}}
   ```
   Output: `My description text` (label removed)

The `!` after the field name tells chronicle to strip the label when creating the file.

### Prepopulated Keywords

Placeholders with these keywords are automatically populated:

- `NAME` - The item's name (from user input)
- `TODAY` - Current date (YYYY-MM-DD)
- `DEFAULT_STATUS` - First status from workflow config
- `OWNER` - (available but not auto-populated yet)

## Configuration

The config file is located at: `~/.config/chronicle/config.toml`

### Workflow Configuration

Define your project statuses in the config:

```toml
version = "1.0"
data_path = "~/chronicle"
editor = "hx"
workflow = ["New", "In Progress", "Done", "Blocked"]
```

The first entry (`"New"`) is used as `{{DEFAULT_STATUS}}`.

## Template Files

Templates are stored in the `templates/` folder:

- `program.md`
- `project.md`
- `milestone.md`
- `task.md`
- `subtask.md`
- `journal.md`

### Example: Task Template

```markdown
# Details
#Title: {{NAME}}
#Status: {{DEFAULT_STATUS}}
#Creation Date: {{TODAY}}
#Created By: {{OWNER}}
#Assigned To: {{ASSIGNED_TO}}
#Due Date: {{DUE_DATE}}

# Description
#Description! {{DESCRIPTION}}
```

When creating a task:
1. You enter the task name → becomes `{{NAME}}`
2. Status is auto-filled from workflow → becomes `{{DEFAULT_STATUS}}`
3. Creation date is today's date → becomes `{{TODAY}}`
4. You are prompted for: Description, Due Date, Assigned To
5. The Description field strips its label (`#Description! ` is removed)

## Markdown Conventions

Chronicle recognizes these metadata keys in markdown files:

```
#title: Task name
#status: todo|doing|done|blocked
#priority: low|med|high|urgent
#due: 2026-03-01
#assigned-to: username
#tags: #tag1 #tag2
```

## Command Palette

Press `/` to open the command palette and access all chronicle commands.
