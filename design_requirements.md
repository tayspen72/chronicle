# Purpose
The underlying goal of the project is to make a project management tool that unifies task management, planning, and daily journaling. The tool will be written in the Rust programming language, function as a TUI, and use a combination of markdown files and a strategic folder structure for storage.
 
# Config
The system will accept a config.toml stored at ~/.config/chronicle/config.toml for parameters and overrides. If the file does not exist when running the software but will be created and the user will be presented a wizard to autofil these fields:
* workspace - the path to where the file storage exists. Defaults to ~/chronical 
* editor - the name of the program that should be spawned to manually edit files. Defaults to "helix"
* workflow - the status fields for task management units in the system. Defaults to: New, Active, Blocked, Testing, Completed, Cancelled
* navigator_width - default width of the navigator panel, defaults to 60
* planning_duration - must be one of these values: weekly, biweekly, cycles (which is defined in docs to be a 6-week session)
 
# Layout
The tui will be split in two parts: navigator and main window. The navigator has the categories: Programs, Planning, Journal. The category name is not selectable. The planning section will show a list of "program" units detected in the workspace.

## Programs
The task management elements are as follows, and in this order: programs, projects, milestones, tasks, and subtasks.
 
When the program first launches, the topmost program is selected. Navigation will traverse the files through a tree structure (formatted similar to `eza-T`, except programs will never be indented, only the children notes will follow the tree notation, and don't use an arrow or triangle to show collapsed or expanded). Left will navigate up a level and collapse the level below, right will traverse deeper and expand the tree view, up/down will cycle through units on the same level (task - task) and not jump out or up a level when passing an extreme top or bottom. Note that program names, iterative planning, backlog, today, and History are considered to be the same level.

## Planning
Planning will have two entries: iteration planning and backlog. Ultimately these two will be created and show reports of statistics and statuses of tasks. For now, nothing. A simple placeholder can be shown (in development) until this is fully thought through.

## Journal
Journal will have two entries: Today and History. Today will show the journal entry for today, History will present a list of entries categorized in folders by year, then by month. The main window will typically show the markdown file selected in the navigator panel, or a relevant report, or both.

## Navigation
Navigation will occur by using the arrow keys (however the config file should allow for override by using the "navigation" tag, accepting a list of 4 values, ordered l r up down).

# Command Interface
The system allows for a command pallet (by pressing "/") to enter commands or quickly jump to a navigator anchor. Use an intelligent fuzzy finder to sort results. Escape will close the pallet.

## Known Commands
* exit - exit the program
* Programs - jump to the topmost program element
* New Program - Create a new program, launches the creation wizard at the program level
* Projects - infer the current program, jump to the topmost project element of this program. If no program can be inferred (ie navigator is in journal, etc) present a selection for the desired program
* New Project - Create a new program using the creation wizard, for the inferred program, or launch the selection tool to choose the desired program
* Milestones - Same as projects but for milestone elements
* New Milestone - same as new project but for milestone elements
* Tasks - Same as projects but for task elements
* New Task - same as new project but for task elements
* Subtasks - Same as projects but for subtask elements
* New Subtask - same as new project but for subtask elements
* Iteration Planning - jump to the summary report
* Start Planning Session - workflow doesn't exist yet, but this will be a command
* Today - jump to the journal entry for today

# Element Creation Wizard
When the user selects to create a new element, the system should load the template for the element and walk the user through a creation wizard with intelligent prompting pulled from the template. The idea is that the templates could be overidden (in the config folder) and the system would dynamically handle the new prompts in the creation wizard.

## Templates
A set of templates (found in `src/templates`) will be used for creating new elements. Each element type has its own template file: program.md, project.md, milestone.md, task.md, subtask.md, journal.md (future feature to include planning.md, not at this time). The templates are of this form:

````
---
uuid: {{UUID}}
title: {{NAME}}
status: {{DEFAULT_STATUS}}
creation_date: {{TODAY}}
created_by: {{OWNER}}
assigned_to: {{ASSIGNED_TO}}
due_date: {{DUE_DATE}}
type: task
---

# Description
{{DESCRIPTION}}
```
  
The top YAML section is used by the software for reporting and project management. Otherwise the file is fully markdown compliant and should be rendered as such. The tags in the YAML format are used in the creation wizard as the prompt name, cast to "Camel Case" (using heck to_title_case()). The placeholder {{VALUE}} should be replaced with the user input from the prompt.

A template file is required to have a DESCRIPTION field. The system should use the static prompt name "Description"

### Keyword Placeholders
Some placeholders are handled as keywords and should be auto populated by the system:
* NAME - the name of the element being created, should match the filename.
* UUID - a system generated uuid used for tracking and tagging tasks
* DEFAULT_STATUS - the first entry pulled from the config.toml workflow list
* TODAY - todays date, in format YYYY-MM-DD

