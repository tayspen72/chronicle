# Journal System

## Overview

Chronicle's journal system provides a daily markdown-based journal that integrates with the task management features. Each day's journal is stored as a separate markdown file following the canonical workspace layout.

## Journal File Structure

Journal entries are stored in:
```
workspace/
└── journal/
    └── YYYY/
        └── MM/
            └── YYYY-MM-DD.md
```

Each journal file uses a standard template with YAML frontmatter and predefined sections:

```markdown
---
uuid: {{UUID}}
title: {{TODAY}}
---

# To do

# Notes
```

### Template Fields

- `{{UUID}}`: Unique identifier for the journal entry
- `{{TODAY}}`: Current date in YYYY-MM-DD format

### Sections

1. **# To do**: Task checklist section that the system parses for interactive todo items
2. **# Notes**: Free-form notes section for daily reflections, ideas, or logs

## Todo Task Detection

The system automatically detects and makes interactive todo items from the "# To do" section of today's journal.

### Supported Formats

The system recognizes todo items in these formats:

**Unchecked items:**
- `- [ ] Task description`
- `* [ ] Task description`

**Checked items:**
- `- [x] Task description`
- `* [x] Task description`
- `- [X] Task description`
- `* [X] Task description`

### Section Recognition

The system identifies the todo section by looking for headers that match (case-insensitive):
- `# To do`
- `# todo`
- `# to-do`

Once a todo section is found, the system continues parsing items until it encounters another level-1 header (starting with `#`).

## Interaction in TUI

### Viewing Today's Todos

Today's todo items appear in the "My Tasks" view alongside any planning tasks assigned to you:

1. Press `/` to open the command palette
2. Select "My Tasks" from the planning reports
3. The view shows:
   - Assigned planning tasks from your current plan
   - Today's todo items from the journal with interactive checkboxes

### Toggling Todo Items

In the "My Tasks" view:
- Navigate to a todo item using arrow keys
- Press `Enter` or `Space` to toggle the checkbox state
- Changes are immediately saved to the journal file

### Editing Todo Items

To edit todo item text:
1. Open today's journal in your configured editor (typically via `Enter` on the journal entry in the sidebar)
2. Modify the todo item text directly
3. Save the file - changes will be reflected in the TUI

## Journal Navigation

### Browsing History

1. In the TUI sidebar, navigate to the "History" section
2. Use `Right`/`Left` or `l`/`h` to expand/collapse year and month nodes
3. Press `Enter` on a specific date to open that journal entry in the viewer
4. Press `Enter` again to open the entry in your external editor

### Today's Journal

Quick access to today's journal:
1. Navigate to the "History" section in the sidebar
2. Today's entry should be pre-expanded and selected
3. Press `Enter` to view in the TUI
4. Press `Enter` again to open in your external editor

## Command Line Usage

### Adding Journal Entries

Use the `jot` command to append timestamped entries to today's journal:

```bash
# Add a simple journal entry
chronicle jot "Completed the user authentication feature"

# Add an entry with more context
chronicle jot "Had productive discussion with team about API design"
```

Entries are appended to the "# Notes" section of today's journal with a timestamp.

### Extracting Todos

Extract todo items from all journal files into your planning backlog:

```bash
chronicle extract
```

This command:
1. Scans all journal markdown files
2. Finds lines containing `/todo` (different from the interactive checkbox format)
3. Converts each to a backlog task in `planning/current/`

Note: This is a separate mechanism from the interactive todo detection and uses the `/todo` pattern rather than checkbox syntax.

## Best Practices

### Daily Planning Workflow

1. **Morning**: Review yesterday's journal and migrate unfinished todos
2. **During Day**: Use the interactive todo checkboxes for task tracking
3. **Evening**: Use `jot` to document accomplishments and reflections
4. **Weekly Review**: Use planning reports to assess progress

### Todo Item Guidelines

- Keep todo items actionable and specific
- Use the checkboxes for visual progress tracking
- Migrate long-running tasks to the planning system
- Use the notes section for context and detailed updates

### Integration with Planning

- Use journal todos for daily, tactical tasks
- Use planning system for strategic, multi-day work
- Extract valuable journal todos to planning backlog when needed
- Link related work using UUIDs or descriptive references

## Customization

### Modifying the Template

To change the default journal template:
1. Edit `templates/journal.md`
2. Modify sections or add new ones as needed
3. The system will use your updated template for new journal entries

### Changing Todo Detection

The todo detection logic is in `src/tui/mod.rs` in the `parse_todo_items` function. To modify:
- Adjust the header recognition logic
- Change the checkbox pattern matching
- Modify the section parsing boundaries

## Troubleshooting

### Todos Not Appearing in My Tasks

1. Verify you're viewing today's journal (not a historical date)
2. Check that the todo section uses one of the recognized headers
3. Ensure todo items use the correct checkbox format
4. Confirm there are no syntax errors in the journal markdown

### Changes Not Saving

1. Verify file permissions on the journal directory
2. Check that Chronicle has write access to the workspace
3. Look for error messages in diagnostics (if enabled)
4. Ensure the journal file isn't open in another editor with conflicting locks

## Related Systems

- **Planning System**: For strategic task management and scheduling
- **Template System**: Controls default journal structure
- **Storage Layer**: Handles reading/writing journal files
- **Command System**: Provides `jot` and `extract` CLI commands

---

*Last updated: March 2026*