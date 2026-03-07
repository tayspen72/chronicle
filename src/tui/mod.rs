pub mod command;
pub mod layout;
pub mod navigation;
pub mod views;

use std::process::Command;

use crate::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use std::io::{self, Write};

use crate::config::Config;
use crate::storage::{DirectoryEntry, JournalEntry, JournalStorage, WorkspaceStorage};
use command::{get_command_list, CommandAction, CommandMatch};
use navigation::{SidebarItem, SidebarSection, TreeState};

/// Application interaction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal navigation and interaction
    Normal,
    /// Command palette is open, typing filters commands
    CommandPalette,
    /// User is inputting data (e.g., creating element)
    #[allow(dead_code)]
    Input, // TODO: Will be used for input mode in future sprint
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub label: String,
    pub placeholder: String,
    pub value: String,
    pub is_focused: bool,
    /// true for user input fields, false for prepopulated keyword fields
    pub is_editable: bool,
    /// Position in template (0-based) to preserve order
    pub display_order: usize,
}

/// Focus state for the template field wizard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardFocus {
    /// Focused on a field at the given index
    Field(usize),
    /// Focused on the CONFIRM button
    ConfirmButton,
    /// Focused on the CANCEL button
    CancelButton,
}

impl Default for WizardFocus {
    fn default() -> Self {
        WizardFocus::Field(0)
    }
}

#[derive(Debug, Clone)]
pub struct TemplateFieldState {
    pub template_name: String,
    pub target_path: Option<std::path::PathBuf>,
    pub fields: Vec<FieldInfo>,
    pub focus: WizardFocus,
    pub values: std::collections::HashMap<String, String>,
    pub strip_labels: std::collections::HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    TreeView,
    Journal,
    JournalArchiveList,
    #[allow(dead_code)]
    JournalToday, // TODO: Reserved for future inline journal editing
    Backlog,
    WeeklyPlanning,
    ViewingContent,
    InputProgram,
    InputProject,
    InputMilestone,
    InputTask,
    InputTemplateField,
}

pub struct App {
    pub config: Config,
    pub current_view: ViewType,
    pub tree_state: TreeState,
    pub mode: Mode,
    pub command_input: String,
    pub command_matches: Vec<CommandMatch>,
    pub should_exit: bool,
    pub journal_entries: Vec<JournalEntry>,
    pub selected_entry_index: usize,
    pub command_selection_index: usize,
    pub needs_terminal_reinit: bool,
    pub current_program: Option<String>,
    pub current_project: Option<String>,
    pub current_milestone: Option<String>,
    pub current_task: Option<String>,
    pub programs: Vec<DirectoryEntry>,
    pub projects: Vec<DirectoryEntry>,
    pub milestones: Vec<DirectoryEntry>,
    pub tasks: Vec<DirectoryEntry>,
    pub subtasks: Vec<DirectoryEntry>,
    pub input_buffer: String,
    pub selected_content: Option<DirectoryEntry>,
    pub current_content_text: Option<String>,
    pub sidebar_items: Vec<SidebarItem>,
    pub template_field_state: Option<TemplateFieldState>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let command_matches = get_command_list();

        let mut app = App {
            config,
            current_view: ViewType::TreeView,
            tree_state: TreeState::default(),
            mode: Mode::Normal,
            command_input: String::new(),
            command_matches,
            should_exit: false,
            journal_entries: Vec::new(),
            selected_entry_index: 0,
            command_selection_index: 0,
            needs_terminal_reinit: false,
            current_program: None,
            current_project: None,
            current_milestone: None,
            current_task: None,
            programs: Vec::new(),
            projects: Vec::new(),
            milestones: Vec::new(),
            tasks: Vec::new(),
            subtasks: Vec::new(),
            input_buffer: String::new(),
            selected_content: None,
            current_content_text: None,
            sidebar_items: Vec::new(),
            template_field_state: None,
        };

        app.load_tree_view_data();
        app
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let mut backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            // Reinitialize terminal if needed (after returning from external editor)
            if self.needs_terminal_reinit {
                drop(terminal);
                execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).ok();
                disable_raw_mode().ok();

                print!("\x1b[2J\x1b[H");
                io::stdout().flush().ok();

                enable_raw_mode()?;
                execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).ok();
                backend = CrosstermBackend::new(io::stdout());
                terminal = Terminal::new(backend)?;

                self.needs_terminal_reinit = false;
            }

            terminal.draw(|f| self.draw(f))?;

            if self.should_exit {
                break;
            }

            if event::poll(std::time::Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if matches!(self.mode, Mode::CommandPalette) {
                            self.handle_command_input(key.code);
                        } else {
                            self.handle_key(key.code);
                        }
                    }
                }
            }

            // Only filter commands when input changes, not every frame
        }

        drop(terminal);
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;

        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('/') => {
                self.mode = Mode::CommandPalette;
                self.command_input.clear();
                self.filter_commands();
            }
            KeyCode::Esc => {
                if matches!(self.mode, Mode::CommandPalette) {
                    self.mode = Mode::Normal;
                    self.command_input.clear();
                    self.command_selection_index = 0;
                } else if self.current_view == ViewType::InputTemplateField {
                    // Escape jumps to CANCEL button
                    if let Some(ref mut state) = self.template_field_state {
                        // Save current field value first
                        if let WizardFocus::Field(idx) = state.focus {
                            if let Some(field) = state.fields.get_mut(idx) {
                                field.value = self.input_buffer.clone();
                            }
                        }
                        state.focus = WizardFocus::CancelButton;
                        self.input_buffer.clear();
                    }
                } else {
                    self.return_from_view();
                }
            }
            KeyCode::Right => {
                self.navigate_right();
            }
            KeyCode::Left => {
                self.navigate_left();
            }
            KeyCode::Up => {
                if self.current_view == ViewType::InputTemplateField {
                    self.navigate_template_field_up();
                } else {
                    self.navigate_up();
                }
            }
            KeyCode::Down => {
                if self.current_view == ViewType::InputTemplateField {
                    self.navigate_template_field_down();
                } else {
                    self.navigate_down();
                }
            }
            KeyCode::Enter => {
                self.handle_enter();
            }
            KeyCode::Tab => {
                if self.current_view == ViewType::InputTemplateField {
                    self.navigate_template_field_down();
                }
            }
            KeyCode::Char(c) => {
                self.handle_input_char(c);
            }
            KeyCode::Backspace => {
                self.handle_input_backspace();
            }
            _ => {}
        }
    }

    fn navigate_template_field_up(&mut self) {
        if let Some(ref mut state) = self.template_field_state {
            // Save current value if on a field
            if let WizardFocus::Field(idx) = state.focus {
                if let Some(field) = state.fields.get_mut(idx) {
                    field.value = self.input_buffer.clone();
                }
            }

            match state.focus {
                WizardFocus::CancelButton => {
                    // From CANCEL, go to CONFIRM
                    state.focus = WizardFocus::ConfirmButton;
                }
                WizardFocus::ConfirmButton => {
                    // From CONFIRM, go to last EDITABLE field
                    if !state.fields.is_empty() {
                        // Find last editable field
                        let last_editable = state.fields.iter().rposition(|f| f.is_editable);
                        if let Some(idx) = last_editable {
                            state.focus = WizardFocus::Field(idx);
                            if let Some(field) = state.fields.get(idx) {
                                self.input_buffer = field.value.clone();
                            }
                        }
                    }
                }
                WizardFocus::Field(idx) => {
                    // Find previous editable field
                    let mut prev_idx = idx;
                    while prev_idx > 0 {
                        prev_idx -= 1;
                        if state.fields[prev_idx].is_editable {
                            state.focus = WizardFocus::Field(prev_idx);
                            if let Some(field) = state.fields.get(prev_idx) {
                                self.input_buffer = field.value.clone();
                            }
                            return;
                        }
                    }
                    // No previous editable field, stay on current
                }
            }
        }
    }

    fn navigate_template_field_down(&mut self) {
        if let Some(ref mut state) = self.template_field_state {
            // Save current value if on a field
            if let WizardFocus::Field(idx) = state.focus {
                if let Some(field) = state.fields.get_mut(idx) {
                    field.value = self.input_buffer.clone();
                }
            }

            match state.focus {
                WizardFocus::Field(idx) => {
                    // Find next editable field
                    let mut next_idx = idx;
                    while next_idx < state.fields.len() - 1 {
                        next_idx += 1;
                        if state.fields[next_idx].is_editable {
                            state.focus = WizardFocus::Field(next_idx);
                            if let Some(field) = state.fields.get(next_idx) {
                                self.input_buffer = field.value.clone();
                            }
                            return;
                        }
                    }
                    // No more editable fields, move to CONFIRM button
                    state.focus = WizardFocus::ConfirmButton;
                    self.input_buffer.clear();
                }
                WizardFocus::ConfirmButton => {
                    // From CONFIRM, go to CANCEL
                    state.focus = WizardFocus::CancelButton;
                }
                WizardFocus::CancelButton => {
                    // From CANCEL, wrap to first EDITABLE field
                    if !state.fields.is_empty() {
                        // Find first editable field
                        let first_editable = state.fields.iter().position(|f| f.is_editable);
                        if let Some(idx) = first_editable {
                            state.focus = WizardFocus::Field(idx);
                            if let Some(field) = state.fields.get(idx) {
                                self.input_buffer = field.value.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    fn navigate_right(&mut self) {
        if self.current_view == ViewType::TreeView {
            self.open_tree_item();
        }
    }

    fn navigate_left(&mut self) {
        if self.current_view == ViewType::TreeView && !self.tree_state.path.is_empty() {
            // Remember the parent we're navigating back to (the one that will remain in path after pop)
            // If path is ["Program", "Project"], after pop it's ["Program"], so "Program" is the target
            let target_name = if self.tree_state.path.len() > 1 {
                // After pop, the target is the second-to-last element (which becomes the last)
                self.tree_state.path[self.tree_state.path.len() - 2].clone()
            } else {
                // We're at depth 1, navigating to root - select first program after header
                String::new() // Empty string means "select first non-header item"
            };

            self.tree_state.path.pop();
            let new_depth = self.tree_state.path.len();
            match new_depth {
                0 => {
                    self.current_program = None;
                    self.current_project = None;
                    self.current_milestone = None;
                    self.current_task = None;
                }
                1 => {
                    self.current_program = Some(self.tree_state.path[0].clone());
                    self.current_project = None;
                    self.current_milestone = None;
                    self.current_task = None;
                }
                2 => {
                    self.current_program = Some(self.tree_state.path[0].clone());
                    self.current_project = Some(self.tree_state.path[1].clone());
                    self.current_milestone = None;
                    self.current_task = None;
                }
                3 => {
                    self.current_program = Some(self.tree_state.path[0].clone());
                    self.current_project = Some(self.tree_state.path[1].clone());
                    self.current_milestone = Some(self.tree_state.path[2].clone());
                    self.current_task = None;
                }
                4 => {
                    self.current_program = Some(self.tree_state.path[0].clone());
                    self.current_project = Some(self.tree_state.path[1].clone());
                    self.current_milestone = Some(self.tree_state.path[2].clone());
                    self.current_task = Some(self.tree_state.path[3].clone());
                }
                _ => {}
            }

            self.load_tree_view_data();

            // Find and select the target item
            if target_name.is_empty() {
                // At root level, select first non-header item
                self.selected_entry_index = if self.sidebar_items.len() > 1 { 1 } else { 0 };
            } else {
                // Find the item with matching name
                self.selected_entry_index = self
                    .sidebar_items
                    .iter()
                    .position(|item| item.name == target_name)
                    .unwrap_or(if self.sidebar_items.len() > 1 { 1 } else { 0 });
            }
        }
    }

    fn return_from_view(&mut self) {
        match self.current_view {
            ViewType::JournalArchiveList => {
                self.current_view = ViewType::Journal;
            }
            ViewType::ViewingContent => {
                self.current_view = ViewType::TreeView;
                self.selected_content = None;
                self.current_content_text = None;
            }
            ViewType::TreeView => {
                if !self.tree_state.path.is_empty() {
                    self.tree_state.path.pop();
                    let new_depth = self.tree_state.path.len();
                    match new_depth {
                        0 => {
                            self.current_program = None;
                            self.current_project = None;
                            self.current_milestone = None;
                            self.current_task = None;
                        }
                        1 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = None;
                            self.current_milestone = None;
                            self.current_task = None;
                        }
                        2 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = Some(self.tree_state.path[1].clone());
                            self.current_milestone = None;
                            self.current_task = None;
                        }
                        3 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = Some(self.tree_state.path[1].clone());
                            self.current_milestone = Some(self.tree_state.path[2].clone());
                            self.current_task = None;
                        }
                        4 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = Some(self.tree_state.path[1].clone());
                            self.current_milestone = Some(self.tree_state.path[2].clone());
                            self.current_task = Some(self.tree_state.path[3].clone());
                        }
                        _ => {}
                    }
                    self.selected_entry_index = 0;
                    self.load_tree_view_data();
                } else {
                    self.current_view = ViewType::Journal;
                }
            }
            ViewType::InputProgram
            | ViewType::InputProject
            | ViewType::InputMilestone
            | ViewType::InputTask => {
                self.current_view = ViewType::TreeView;
            }
            _ => {}
        }
    }

    fn navigate_up(&mut self) {
        self.selected_entry_index =
            navigation::navigate_up(&self.sidebar_items, self.selected_entry_index);
    }

    fn navigate_down(&mut self) {
        self.selected_entry_index =
            navigation::navigate_down(&self.sidebar_items, self.selected_entry_index);
    }

    // TODO: These helper methods are extracted for future use in pagination/scrolling.
    #[allow(dead_code)]
    fn get_current_tier_start_index(&self) -> usize {
        0
    }

    #[allow(dead_code)]
    fn get_current_tier_item_count(&self) -> usize {
        self.sidebar_items
            .iter()
            .filter(|i| !i.is_header && !i.name.is_empty())
            .count()
    }

    #[allow(dead_code)]
    fn get_visible_item_count(&self) -> usize {
        self.sidebar_items
            .iter()
            .filter(|i| !i.is_header && !i.name.is_empty())
            .count()
    }

    fn open_tree_item(&mut self) {
        let idx = self.selected_entry_index;

        if idx >= self.sidebar_items.len() {
            return;
        }

        let item = &self.sidebar_items[idx];

        if item.is_header || item.name.is_empty() {
            return;
        }

        // Handle create action items
        if item.is_create_action && item.name == "+ Create Program..." {
            self.start_new_program();
            return;
        }

        if let Some(plan_type) = &item.is_planning_item {
            match plan_type.as_str() {
                "WeeklyPlanning" => {
                    self.current_view = ViewType::WeeklyPlanning;
                }
                "Backlog" => {
                    self.current_view = ViewType::Backlog;
                }
                _ => {}
            }
            return;
        }

        if let Some(journal_action) = &item.is_journal_item {
            match journal_action.as_str() {
                "Today" => {
                    if let Ok((path, _)) = self.config.workspace.open_or_create_today_journal() {
                        self.launch_editor(&path);
                    }
                }
                "History" => {
                    match self.config.workspace.list_journal_entries() {
                        Ok(entries) => self.journal_entries = entries,
                        Err(e) => {
                            eprintln!("Failed to list journal entries: {}", e);
                            self.journal_entries.clear();
                        }
                    }
                    self.current_view = ViewType::JournalArchiveList;
                }
                _ => {}
            }
            return;
        }

        if let Some(path) = &item.path {
            let entry = DirectoryEntry {
                name: item.name.clone(),
                path: path.clone(),
                is_dir: false,
            };

            let depth = self.tree_state.path.len();
            let is_dir = match depth {
                0 => true,
                1 => self.projects.iter().any(|p| p.name == item.name),
                2 => self.milestones.iter().any(|m| m.name == item.name),
                3 => {
                    // A task is expandable if we can find subtasks for it
                    // This is true whether the task is a directory or a flat .md file
                    self.tasks.iter().any(|t| t.name == item.name)
                }
                4 => self
                    .subtasks
                    .iter()
                    .any(|s| s.name == item.name && s.is_dir),
                _ => false,
            };

            if is_dir {
                let current_idx = self.selected_entry_index;
                self.tree_state.path.push(item.name.clone());
                self.load_tree_view_data();
                // Select the first child item (one position after the expanded parent)
                // After rebuild, parent is at same index, children follow immediately
                self.selected_entry_index = current_idx + 1;
                // Clamp to valid range
                if self.selected_entry_index >= self.sidebar_items.len() {
                    self.selected_entry_index = self.sidebar_items.len().saturating_sub(1);
                }
            } else {
                self.open_content(&entry);
            }
        }
    }

    // TODO: Helper for getting current tier's items, useful for future keyboard shortcuts
    #[allow(dead_code)]
    fn get_current_tree_items(&self) -> &Vec<DirectoryEntry> {
        match self.tree_state.path.len() {
            0 => &self.programs,
            1 => &self.projects,
            2 => &self.milestones,
            3 => &self.tasks,
            4 => &self.subtasks,
            _ => &self.programs,
        }
    }

    fn open_content(&mut self, entry: &DirectoryEntry) {
        if let Ok(content) = self.config.workspace.read_md_file(&entry.path) {
            self.current_content_text = Some(content);
            self.current_view = ViewType::ViewingContent;
            self.selected_content = Some(entry.clone());
        }
    }

    fn load_tree_view_data(&mut self) {
        match self.tree_state.path.len() {
            0 => {
                match self.config.workspace.list_programs() {
                    Ok(entries) => self.programs = entries,
                    Err(e) => {
                        tracing::warn!("Failed to list programs: {}", e);
                        self.programs.clear();
                    }
                }
                self.current_program = None;
                self.current_project = None;
                self.current_milestone = None;
                self.current_task = None;
            }
            1 => {
                let program = &self.tree_state.path[0];
                self.current_program = Some(program.clone());
                match self.config.workspace.list_projects(program) {
                    Ok(entries) => self.projects = entries,
                    Err(e) => {
                        tracing::warn!("Failed to list projects: {}", e);
                        self.projects.clear();
                    }
                }
            }
            2 => {
                let program = &self.tree_state.path[0];
                let project = &self.tree_state.path[1];
                self.current_program = Some(program.clone());
                self.current_project = Some(project.clone());
                match self.config.workspace.list_milestones(program, project) {
                    Ok(entries) => self.milestones = entries,
                    Err(e) => {
                        tracing::warn!("Failed to list milestones: {}", e);
                        self.milestones.clear();
                    }
                }
            }
            3 => {
                let program = &self.tree_state.path[0];
                let project = &self.tree_state.path[1];
                let milestone = &self.tree_state.path[2];
                self.current_program = Some(program.clone());
                self.current_project = Some(project.clone());
                self.current_milestone = Some(milestone.clone());
                match self
                    .config
                    .workspace
                    .list_tasks(program, project, milestone)
                {
                    Ok(entries) => self.tasks = entries,
                    Err(e) => {
                        tracing::warn!("Failed to list tasks: {}", e);
                        self.tasks.clear();
                    }
                }
            }
            4 => {
                let program = &self.tree_state.path[0];
                let project = &self.tree_state.path[1];
                let milestone = &self.tree_state.path[2];
                let task = &self.tree_state.path[3];
                self.current_program = Some(program.clone());
                self.current_project = Some(project.clone());
                self.current_milestone = Some(milestone.clone());
                self.current_task = Some(task.clone());
                match self
                    .config
                    .workspace
                    .list_subtasks(program, project, milestone, task)
                {
                    Ok(entries) => self.subtasks = entries,
                    Err(e) => {
                        tracing::warn!("Failed to list subtasks: {}", e);
                        self.subtasks.clear();
                    }
                }
            }
            _ => {}
        }
        self.build_sidebar_items();
        // Select first non-header item (skip "Programs" header at index 0)
        self.selected_entry_index = if self.sidebar_items.len() > 1 { 1 } else { 0 };
    }

    fn build_sidebar_items(&mut self) {
        self.sidebar_items = navigation::build_sidebar_items(
            &self.programs,
            &self.projects,
            &self.milestones,
            &self.tasks,
            self.current_program.as_deref(),
            self.current_project.as_deref(),
            self.current_milestone.as_deref(),
        );

        // Handle subtasks separately since the extracted function doesn't include them
        // TODO: Architect to decide if subtasks should be added to navigation::build_sidebar_items
        if let (Some(_current_milestone), Some(current_task)) =
            (self.current_milestone.as_ref(), self.current_task.as_ref())
        {
            // Find the index of the current task and add subtasks after it
            let task_idx = self
                .sidebar_items
                .iter()
                .position(|i| i.name == *current_task && i.indent == 3);
            if let Some(idx) = task_idx {
                // Insert subtasks after the task
                let subtask_items: Vec<SidebarItem> = self
                    .subtasks
                    .iter()
                    .map(|subtask| SidebarItem {
                        name: subtask.name.clone(),
                        section: SidebarSection::Programs,
                        is_header: false,
                        is_planning_item: None,
                        is_journal_item: None,
                        indent: 4,
                        path: Some(subtask.path.clone()),
                        is_create_action: false,
                    })
                    .collect();

                for (offset, item) in subtask_items.into_iter().enumerate() {
                    self.sidebar_items.insert(idx + 1 + offset, item);
                }
            }
        }
    }

    fn handle_enter(&mut self) {
        match &self.current_view {
            ViewType::TreeView => {
                self.open_tree_item();
            }
            ViewType::JournalArchiveList => {
                self.open_selected_archive_entry();
            }
            ViewType::InputProgram => {
                self.confirm_create_program();
            }
            ViewType::InputProject => {
                self.confirm_create_project();
            }
            ViewType::InputMilestone => {
                self.confirm_create_milestone();
            }
            ViewType::InputTask => {
                self.confirm_create_task();
            }
            ViewType::InputTemplateField => {
                self.confirm_template_field();
            }
            _ => {}
        }
    }

    fn handle_input_char(&mut self, c: char) {
        match &self.current_view {
            ViewType::InputProgram
            | ViewType::InputProject
            | ViewType::InputMilestone
            | ViewType::InputTask => {
                self.input_buffer.push(c);
            }
            ViewType::InputTemplateField => {
                // Only allow input when focused on an editable field
                if let Some(ref mut state) = self.template_field_state {
                    if let WizardFocus::Field(idx) = state.focus {
                        if let Some(field) = state.fields.get_mut(idx) {
                            if field.is_editable {
                                // Update both input_buffer and field.value for inline editing
                                self.input_buffer.push(c);
                                field.value.push(c);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_input_backspace(&mut self) {
        match &self.current_view {
            ViewType::InputProgram
            | ViewType::InputProject
            | ViewType::InputMilestone
            | ViewType::InputTask => {
                self.input_buffer.pop();
            }
            ViewType::InputTemplateField => {
                // Only allow input when focused on an editable field
                if let Some(ref mut state) = self.template_field_state {
                    if let WizardFocus::Field(idx) = state.focus {
                        if let Some(field) = state.fields.get_mut(idx) {
                            if field.is_editable {
                                // Update both input_buffer and field.value for inline editing
                                self.input_buffer.pop();
                                field.value.pop();
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // TODO: These methods are helpers for future keyboard shortcuts for quick element creation
    #[allow(dead_code)]
    fn select_program_for_new_project(&mut self) {
        if let Some(entry) = self.programs.get(self.selected_entry_index) {
            self.current_program = Some(entry.name.clone());
            self.input_buffer.clear();
            self.current_view = ViewType::InputProject;
        }
    }

    #[allow(dead_code)]
    fn select_project_for_new_milestone(&mut self) {
        if let Some(entry) = self.projects.get(self.selected_entry_index) {
            self.current_project = Some(entry.name.clone());
            self.input_buffer.clear();
            self.current_view = ViewType::InputMilestone;
        }
    }

    #[allow(dead_code)]
    fn select_milestone_for_new_task(&mut self) {
        if let Some(entry) = self.milestones.get(self.selected_entry_index) {
            self.current_milestone = Some(entry.name.clone());
            self.input_buffer.clear();
            self.current_view = ViewType::InputTask;
        }
    }

    fn handle_command_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.command_input.push(c);
                self.command_selection_index = 0;
                self.filter_commands();
            }
            KeyCode::Backspace => {
                self.command_input.pop();
                self.command_selection_index = 0;
                self.filter_commands();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_input.clear();
                self.command_selection_index = 0;
            }
            KeyCode::Enter => {
                if let Some(cmd) = self
                    .command_matches
                    .get(self.command_selection_index)
                    .cloned()
                {
                    self.execute_command(&cmd);
                }
                self.mode = Mode::Normal;
                self.command_input.clear();
                self.command_selection_index = 0;
            }
            KeyCode::Up => {
                if self.command_selection_index > 0 {
                    self.command_selection_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.command_selection_index < self.command_matches.len().saturating_sub(1) {
                    self.command_selection_index += 1;
                }
            }
            _ => {}
        }
    }

    fn execute_command(&mut self, cmd: &CommandMatch) {
        if cmd.exit {
            self.should_exit = true;
            return;
        }

        match &cmd.action {
            Some(CommandAction::OpenTodayJournal) => {
                self.open_today_journal();
            }
            Some(CommandAction::ShowArchiveList) => {
                self.show_archive_list();
            }
            Some(CommandAction::ShowProgramsList) => {
                self.show_programs_list();
            }
            Some(CommandAction::ShowProjectsList) => {
                self.show_projects_list();
            }
            Some(CommandAction::ShowMilestonesList) => {
                self.show_milestones_list();
            }
            Some(CommandAction::ShowTasksList) => {
                self.show_tasks_list();
            }
            Some(CommandAction::NewProgram) => {
                self.start_new_program();
            }
            Some(CommandAction::NewProject) => {
                self.start_new_project();
            }
            Some(CommandAction::NewMilestone) => {
                self.start_new_milestone();
            }
            Some(CommandAction::NewTask) => {
                self.start_new_task();
            }
            Some(CommandAction::Refresh) => {
                self.load_tree_view_data();
            }
            None => {
                self.current_view = cmd.view.clone();
            }
        }
    }

    fn open_today_journal(&mut self) {
        let workspace = &self.config.workspace;
        match workspace.open_or_create_today_journal() {
            Ok((path, _)) => {
                self.launch_editor(&path);
            }
            Err(e) => {
                eprintln!("Error opening today's journal: {}", e);
            }
        }
    }

    fn show_archive_list(&mut self) {
        let workspace = &self.config.workspace;
        match workspace.list_journal_entries() {
            Ok(entries) => {
                self.journal_entries = entries;
                self.selected_entry_index = 0;
                self.current_view = ViewType::JournalArchiveList;
            }
            Err(e) => {
                eprintln!("Error loading archive list: {}", e);
            }
        }
    }

    fn open_archive_entry(&mut self, index: usize) {
        if let Some(entry) = self.journal_entries.get(index) {
            let path = entry.path.clone();
            self.launch_editor(&path);
        }
    }

    fn open_selected_archive_entry(&mut self) {
        self.open_archive_entry(self.selected_entry_index);
    }

    fn launch_editor(&mut self, path: &std::path::Path) {
        let editor = &self.config.editor;

        // Leave alternate screen and disable raw mode
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).ok();
        disable_raw_mode().ok();

        // Clear terminal
        print!("\x1b[2J\x1b[H");
        io::stdout().flush().ok();

        let result = Command::new(editor).arg(path).status();

        match result {
            Ok(status) => {
                if !status.success() {
                    eprintln!("Editor exited with error: {}", status);
                }
            }
            Err(e) => {
                eprintln!("Failed to launch editor '{}': {}", editor, e);
            }
        }

        // Mark that we need to reinitialize the terminal
        self.needs_terminal_reinit = true;
    }

    // Workspace functions: Programs, Projects, Milestones, Tasks

    fn show_programs_list(&mut self) {
        self.tree_state.path.clear();
        self.load_tree_view_data();
        self.current_view = ViewType::TreeView;
    }

    fn show_projects_list(&mut self) {
        if !self.tree_state.path.is_empty() {
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        } else {
            self.tree_state.path.clear();
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        }
    }

    fn show_milestones_list(&mut self) {
        if self.tree_state.path.len() >= 2 {
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        } else {
            self.tree_state.path.clear();
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        }
    }

    fn show_tasks_list(&mut self) {
        if self.tree_state.path.len() >= 3 {
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        } else {
            self.tree_state.path.clear();
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        }
    }

    fn start_new_program(&mut self) {
        // Go directly to template field wizard - name will be the first editable field
        self.input_buffer.clear();

        let template = include_str!("../../templates/program.md");
        let all_fields = crate::storage::parse_template_fields(template);

        let mut values = std::collections::HashMap::new();
        values.insert(
            "TODAY".to_string(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );
        values.insert("OWNER".to_string(), self.config.owner.clone());
        if let Some(default_status) = self.config.workflow.first() {
            values.insert("DEFAULT_STATUS".to_string(), default_status.clone());
        }

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|(_, _, strip)| *strip)
            .map(|(_, p, _)| p.clone())
            .collect();

        // Keywords that are prepopulated and not editable
        let keywords = ["TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];

        // Convert to FieldInfo structures - include ALL fields, mark as editable or not
        let fields: Vec<FieldInfo> = all_fields
            .into_iter()
            .enumerate()
            .map(|(i, (label, placeholder, _))| {
                let is_keyword = keywords.contains(&placeholder.as_str());
                let value = if is_keyword {
                    values.get(&placeholder).cloned().unwrap_or_default()
                } else {
                    String::new()
                };
                FieldInfo {
                    label,
                    placeholder,
                    value,
                    is_focused: i == 0 && !is_keyword,
                    is_editable: !is_keyword,
                    display_order: i,
                }
            })
            .collect();

        // Find first editable field for initial focus (this will be NAME/Title)
        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        // Target path will be determined later after name is entered
        self.template_field_state = Some(TemplateFieldState {
            template_name: "program".to_string(),
            target_path: None, // Will be set when confirmed
            fields,
            focus: initial_focus,
            values,
            strip_labels,
        });

        // Load initial field value into buffer
        if let WizardFocus::Field(idx) = initial_focus {
            if let Some(field) = self
                .template_field_state
                .as_ref()
                .and_then(|s| s.fields.get(idx))
            {
                self.input_buffer = field.value.clone();
            }
        } else {
            self.input_buffer.clear();
        }
        self.current_view = ViewType::InputTemplateField;
    }

    fn start_new_project(&mut self) {
        // If at depth 0 (programs level), navigate into selected program first
        if self.tree_state.path.is_empty() {
            if let Some(entry) = self.programs.get(self.selected_entry_index) {
                self.tree_state.path.push(entry.name.clone());
                self.current_program = Some(entry.name.clone());
                self.selected_entry_index = 0;
                self.load_tree_view_data();
            }
        }

        // Go directly to template field wizard
        self.input_buffer.clear();

        let template = include_str!("../../templates/project.md");
        let all_fields = crate::storage::parse_template_fields(template);

        let mut values = std::collections::HashMap::new();
        values.insert(
            "TODAY".to_string(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );
        values.insert("OWNER".to_string(), self.config.owner.clone());
        if let Some(default_status) = self.config.workflow.first() {
            values.insert("DEFAULT_STATUS".to_string(), default_status.clone());
        }

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|(_, _, strip)| *strip)
            .map(|(_, p, _)| p.clone())
            .collect();

        // Keywords that are prepopulated and not editable
        let keywords = ["TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];

        let fields: Vec<FieldInfo> = all_fields
            .into_iter()
            .enumerate()
            .map(|(i, (label, placeholder, _))| {
                let is_keyword = keywords.contains(&placeholder.as_str());
                let value = if is_keyword {
                    values.get(&placeholder).cloned().unwrap_or_default()
                } else {
                    String::new()
                };
                FieldInfo {
                    label,
                    placeholder,
                    value,
                    is_focused: i == 0 && !is_keyword,
                    is_editable: !is_keyword,
                    display_order: i,
                }
            })
            .collect();

        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        self.template_field_state = Some(TemplateFieldState {
            template_name: "project".to_string(),
            target_path: None,
            fields,
            focus: initial_focus,
            values,
            strip_labels,
        });

        if let WizardFocus::Field(idx) = initial_focus {
            if let Some(field) = self
                .template_field_state
                .as_ref()
                .and_then(|s| s.fields.get(idx))
            {
                self.input_buffer = field.value.clone();
            }
        } else {
            self.input_buffer.clear();
        }
        self.current_view = ViewType::InputTemplateField;
    }

    fn start_new_milestone(&mut self) {
        // If at depth 0, navigate into selected program first
        if self.tree_state.path.is_empty() {
            if let Some(entry) = self.programs.get(self.selected_entry_index) {
                self.tree_state.path.push(entry.name.clone());
                self.current_program = Some(entry.name.clone());
                self.selected_entry_index = 0;
                self.load_tree_view_data();
            }
        }
        // If at depth 1, navigate into selected project first
        if self.tree_state.path.len() == 1 {
            if let Some(entry) = self.projects.get(
                self.selected_entry_index
                    .saturating_sub(self.programs.len()),
            ) {
                self.tree_state.path.push(entry.name.clone());
                self.current_project = Some(entry.name.clone());
                self.selected_entry_index = 0;
                self.load_tree_view_data();
            }
        }

        // Go directly to template field wizard
        self.input_buffer.clear();

        let template = include_str!("../../templates/milestone.md");
        let all_fields = crate::storage::parse_template_fields(template);

        let mut values = std::collections::HashMap::new();
        values.insert(
            "TODAY".to_string(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );
        values.insert("OWNER".to_string(), self.config.owner.clone());
        if let Some(default_status) = self.config.workflow.first() {
            values.insert("DEFAULT_STATUS".to_string(), default_status.clone());
        }

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|(_, _, strip)| *strip)
            .map(|(_, p, _)| p.clone())
            .collect();

        // Keywords that are prepopulated and not editable
        let keywords = ["TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];

        let fields: Vec<FieldInfo> = all_fields
            .into_iter()
            .enumerate()
            .map(|(i, (label, placeholder, _))| {
                let is_keyword = keywords.contains(&placeholder.as_str());
                let value = if is_keyword {
                    values.get(&placeholder).cloned().unwrap_or_default()
                } else {
                    String::new()
                };
                FieldInfo {
                    label,
                    placeholder,
                    value,
                    is_focused: i == 0 && !is_keyword,
                    is_editable: !is_keyword,
                    display_order: i,
                }
            })
            .collect();

        // Find first editable field for initial focus (this will be NAME/Title)
        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        // Target path will be determined later after name is entered
        self.template_field_state = Some(TemplateFieldState {
            template_name: "milestone".to_string(),
            target_path: None, // Will be set when confirmed
            fields,
            focus: initial_focus,
            values,
            strip_labels,
        });

        // Load initial field value into buffer
        if let WizardFocus::Field(idx) = initial_focus {
            if let Some(field) = self
                .template_field_state
                .as_ref()
                .and_then(|s| s.fields.get(idx))
            {
                self.input_buffer = field.value.clone();
            }
        } else {
            self.input_buffer.clear();
        }
        self.current_view = ViewType::InputTemplateField;
    }

    fn start_new_task(&mut self) {
        // If at depth 0, navigate into selected program first
        if self.tree_state.path.is_empty() {
            if let Some(entry) = self.programs.get(self.selected_entry_index) {
                self.tree_state.path.push(entry.name.clone());
                self.current_program = Some(entry.name.clone());
                self.selected_entry_index = 0;
                self.load_tree_view_data();
            }
        }
        // If at depth 1, navigate into selected project first
        if self.tree_state.path.len() == 1 {
            if let Some(entry) = self.projects.get(
                self.selected_entry_index
                    .saturating_sub(self.programs.len()),
            ) {
                self.tree_state.path.push(entry.name.clone());
                self.current_project = Some(entry.name.clone());
                self.selected_entry_index = 0;
                self.load_tree_view_data();
            }
        }
        // If at depth 2, navigate into selected milestone first
        if self.tree_state.path.len() == 2 {
            if let Some(entry) = self.milestones.get(
                self.selected_entry_index
                    .saturating_sub(self.programs.len() + self.projects.len()),
            ) {
                self.tree_state.path.push(entry.name.clone());
                self.current_milestone = Some(entry.name.clone());
                self.selected_entry_index = 0;
                self.load_tree_view_data();
            }
        }

        // Go directly to template field wizard
        self.input_buffer.clear();

        let template = include_str!("../../templates/task.md");
        let all_fields = crate::storage::parse_template_fields(template);

        let mut values = std::collections::HashMap::new();
        values.insert(
            "TODAY".to_string(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );
        values.insert("OWNER".to_string(), self.config.owner.clone());
        if let Some(default_status) = self.config.workflow.first() {
            values.insert("DEFAULT_STATUS".to_string(), default_status.clone());
        }

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|(_, _, strip)| *strip)
            .map(|(_, p, _)| p.clone())
            .collect();

        // Keywords that are prepopulated and not editable
        let keywords = ["TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];

        let fields: Vec<FieldInfo> = all_fields
            .into_iter()
            .enumerate()
            .map(|(i, (label, placeholder, _))| {
                let is_keyword = keywords.contains(&placeholder.as_str());
                let value = if is_keyword {
                    values.get(&placeholder).cloned().unwrap_or_default()
                } else {
                    String::new()
                };
                FieldInfo {
                    label,
                    placeholder,
                    value,
                    is_focused: i == 0 && !is_keyword,
                    is_editable: !is_keyword,
                    display_order: i,
                }
            })
            .collect();

        // Find first editable field for initial focus (this will be NAME/Title)
        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        // Target path will be determined later after name is entered
        self.template_field_state = Some(TemplateFieldState {
            template_name: "task".to_string(),
            target_path: None, // Will be set when confirmed
            fields,
            focus: initial_focus,
            values,
            strip_labels,
        });

        // Load initial field value into buffer
        if let WizardFocus::Field(idx) = initial_focus {
            if let Some(field) = self
                .template_field_state
                .as_ref()
                .and_then(|s| s.fields.get(idx))
            {
                self.input_buffer = field.value.clone();
            }
        } else {
            self.input_buffer.clear();
        }
        self.current_view = ViewType::InputTemplateField;
    }

    fn confirm_create_program(&mut self) {
        let template = include_str!("../../templates/program.md");
        let all_fields = crate::storage::parse_template_fields(template);
        let target_path = self
            .config
            .workspace
            .programs_dir()
            .join(format!("{}.md", self.input_buffer));

        let mut values = std::collections::HashMap::new();
        values.insert("PROGRAM_NAME".to_string(), self.input_buffer.clone());
        values.insert("NAME".to_string(), self.input_buffer.clone());
        values.insert(
            "TODAY".to_string(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );
        values.insert("OWNER".to_string(), self.config.owner.clone());
        if let Some(default_status) = self.config.workflow.first() {
            values.insert("DEFAULT_STATUS".to_string(), default_status.clone());
        }

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|(_, _, strip)| *strip)
            .map(|(_, p, _)| p.clone())
            .collect();

        // Keywords that are prepopulated and not editable
        let keywords = ["NAME", "TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];

        // Convert to FieldInfo structures - include ALL fields, mark as editable or not
        let fields: Vec<FieldInfo> = all_fields
            .into_iter()
            .enumerate()
            .map(|(i, (label, placeholder, _))| {
                let is_keyword = keywords.contains(&placeholder.as_str());
                let value = if is_keyword {
                    values.get(&placeholder).cloned().unwrap_or_default()
                } else {
                    String::new()
                };
                FieldInfo {
                    label,
                    placeholder,
                    value,
                    is_focused: i == 0 && !is_keyword,
                    is_editable: !is_keyword,
                    display_order: i,
                }
            })
            .collect();

        // Find first editable field for initial focus
        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        self.template_field_state = Some(TemplateFieldState {
            template_name: "program".to_string(),
            target_path: Some(target_path),
            fields,
            focus: initial_focus,
            values,
            strip_labels,
        });

        // Load initial field value into buffer
        if let WizardFocus::Field(idx) = initial_focus {
            if let Some(field) = self
                .template_field_state
                .as_ref()
                .and_then(|s| s.fields.get(idx))
            {
                self.input_buffer = field.value.clone();
            }
        } else {
            self.input_buffer.clear();
        }
        self.current_view = ViewType::InputTemplateField;
    }

    fn confirm_create_project(&mut self) {
        let template = include_str!("../../templates/project.md");
        let all_fields = crate::storage::parse_template_fields(template);

        let target_path = self
            .config
            .workspace
            .programs_dir()
            .join(self.current_program.as_ref().unwrap())
            .join(format!("{}.md", self.input_buffer));

        let mut values = std::collections::HashMap::new();
        values.insert("PROJECT_NAME".to_string(), self.input_buffer.clone());
        values.insert("NAME".to_string(), self.input_buffer.clone());
        values.insert(
            "TODAY".to_string(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );
        values.insert("OWNER".to_string(), self.config.owner.clone());
        if let Some(default_status) = self.config.workflow.first() {
            values.insert("DEFAULT_STATUS".to_string(), default_status.clone());
        }

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|(_, _, strip)| *strip)
            .map(|(_, p, _)| p.clone())
            .collect();

        // Keywords that are prepopulated and not editable
        let keywords = ["NAME", "TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];

        // Convert to FieldInfo structures - include ALL fields, mark as editable or not
        let fields: Vec<FieldInfo> = all_fields
            .into_iter()
            .enumerate()
            .map(|(i, (label, placeholder, _))| {
                let is_keyword = keywords.contains(&placeholder.as_str());
                let value = if is_keyword {
                    values.get(&placeholder).cloned().unwrap_or_default()
                } else {
                    String::new()
                };
                FieldInfo {
                    label,
                    placeholder,
                    value,
                    is_focused: i == 0 && !is_keyword,
                    is_editable: !is_keyword,
                    display_order: i,
                }
            })
            .collect();

        // Find first editable field for initial focus
        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        self.template_field_state = Some(TemplateFieldState {
            template_name: "project".to_string(),
            target_path: Some(target_path),
            fields,
            focus: initial_focus,
            values,
            strip_labels,
        });

        // Load initial field value into buffer
        if let WizardFocus::Field(idx) = initial_focus {
            if let Some(field) = self
                .template_field_state
                .as_ref()
                .and_then(|s| s.fields.get(idx))
            {
                self.input_buffer = field.value.clone();
            }
        } else {
            self.input_buffer.clear();
        }
        self.current_view = ViewType::InputTemplateField;
    }

    fn confirm_create_milestone(&mut self) {
        let template = include_str!("../../templates/milestone.md");
        let all_fields = crate::storage::parse_template_fields(template);

        let target_path = self
            .config
            .workspace
            .programs_dir()
            .join(self.current_program.as_ref().unwrap())
            .join(self.current_project.as_ref().unwrap())
            .join(format!("{}.md", self.input_buffer));

        let mut values = std::collections::HashMap::new();
        values.insert("MILESTONE_NAME".to_string(), self.input_buffer.clone());
        values.insert("NAME".to_string(), self.input_buffer.clone());
        values.insert(
            "TODAY".to_string(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );
        values.insert("OWNER".to_string(), self.config.owner.clone());
        if let Some(default_status) = self.config.workflow.first() {
            values.insert("DEFAULT_STATUS".to_string(), default_status.clone());
        }

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|(_, _, strip)| *strip)
            .map(|(_, p, _)| p.clone())
            .collect();

        // Keywords that are prepopulated and not editable
        let keywords = ["NAME", "TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];

        // Convert to FieldInfo structures - include ALL fields, mark as editable or not
        let fields: Vec<FieldInfo> = all_fields
            .into_iter()
            .enumerate()
            .map(|(i, (label, placeholder, _))| {
                let is_keyword = keywords.contains(&placeholder.as_str());
                let value = if is_keyword {
                    values.get(&placeholder).cloned().unwrap_or_default()
                } else {
                    String::new()
                };
                FieldInfo {
                    label,
                    placeholder,
                    value,
                    is_focused: i == 0 && !is_keyword,
                    is_editable: !is_keyword,
                    display_order: i,
                }
            })
            .collect();

        // Find first editable field for initial focus
        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        self.template_field_state = Some(TemplateFieldState {
            template_name: "milestone".to_string(),
            target_path: Some(target_path),
            fields,
            focus: initial_focus,
            values,
            strip_labels,
        });

        // Load initial field value into buffer
        if let WizardFocus::Field(idx) = initial_focus {
            if let Some(field) = self
                .template_field_state
                .as_ref()
                .and_then(|s| s.fields.get(idx))
            {
                self.input_buffer = field.value.clone();
            }
        } else {
            self.input_buffer.clear();
        }
        self.current_view = ViewType::InputTemplateField;
    }

    fn confirm_create_task(&mut self) {
        let template = include_str!("../../templates/task.md");
        let all_fields = crate::storage::parse_template_fields(template);

        let target_path = self
            .config
            .workspace
            .programs_dir()
            .join(self.current_program.as_ref().unwrap())
            .join(self.current_project.as_ref().unwrap())
            .join(self.current_milestone.as_ref().unwrap())
            .join(format!("{}.md", self.input_buffer));

        let mut values = std::collections::HashMap::new();
        values.insert("TASK_NAME".to_string(), self.input_buffer.clone());
        values.insert("NAME".to_string(), self.input_buffer.clone());
        values.insert(
            "TODAY".to_string(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );
        values.insert("OWNER".to_string(), self.config.owner.clone());
        if let Some(default_status) = self.config.workflow.first() {
            values.insert("DEFAULT_STATUS".to_string(), default_status.clone());
        }

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|(_, _, strip)| *strip)
            .map(|(_, p, _)| p.clone())
            .collect();

        // Keywords that are prepopulated and not editable
        let keywords = ["NAME", "TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];

        // Convert to FieldInfo structures - include ALL fields, mark as editable or not
        let fields: Vec<FieldInfo> = all_fields
            .into_iter()
            .enumerate()
            .map(|(i, (label, placeholder, _))| {
                let is_keyword = keywords.contains(&placeholder.as_str());
                let value = if is_keyword {
                    values.get(&placeholder).cloned().unwrap_or_default()
                } else {
                    String::new()
                };
                FieldInfo {
                    label,
                    placeholder,
                    value,
                    is_focused: i == 0 && !is_keyword,
                    is_editable: !is_keyword,
                    display_order: i,
                }
            })
            .collect();

        // Find first editable field for initial focus
        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        self.template_field_state = Some(TemplateFieldState {
            template_name: "task".to_string(),
            target_path: Some(target_path),
            fields,
            focus: initial_focus,
            values,
            strip_labels,
        });

        // Load initial field value into buffer
        if let WizardFocus::Field(idx) = initial_focus {
            if let Some(field) = self
                .template_field_state
                .as_ref()
                .and_then(|s| s.fields.get(idx))
            {
                self.input_buffer = field.value.clone();
            }
        } else {
            self.input_buffer.clear();
        }
        self.current_view = ViewType::InputTemplateField;
    }

    fn confirm_template_field(&mut self) {
        if let Some(ref mut state) = self.template_field_state {
            match state.focus {
                WizardFocus::CancelButton => {
                    // Cancel - return to tree view without creating
                    self.template_field_state = None;
                    self.current_view = ViewType::TreeView;
                }
                WizardFocus::ConfirmButton => {
                    // Confirm - collect all field values and create the element
                    // Save any current field value first
                    for field in &state.fields {
                        state
                            .values
                            .insert(field.placeholder.clone(), field.value.clone());
                    }

                    let template_name = state.template_name.clone();
                    // Templates use "NAME" as the placeholder for the element name
                    // (not PROGRAM_NAME, PROJECT_NAME, or MILESTONE_NAME)
                    let name = state.values.get("NAME").cloned();

                    // Also check for type-specific names for backwards compatibility
                    let program_name = state.values.get("PROGRAM_NAME").or(name.as_ref()).cloned();
                    let project_name = state.values.get("PROJECT_NAME").or(name.as_ref()).cloned();
                    let milestone_name = state
                        .values
                        .get("MILESTONE_NAME")
                        .or(name.as_ref())
                        .cloned();

                    // Compute target_path based on template_name and current context
                    let target_path = match template_name.as_str() {
                        "program" => program_name.as_ref().map(|n| {
                            self.config
                                .workspace
                                .programs_dir()
                                .join(format!("{}.md", n))
                        }),
                        "project" => {
                            if let (Some(prog), Some(proj)) = (&self.current_program, &project_name)
                            {
                                Some(
                                    self.config
                                        .workspace
                                        .programs_dir()
                                        .join(prog)
                                        .join(format!("{}.md", proj)),
                                )
                            } else {
                                None
                            }
                        }
                        "milestone" => {
                            if let (Some(prog), Some(proj), Some(mil)) = (
                                &self.current_program,
                                &self.current_project,
                                &milestone_name,
                            ) {
                                Some(
                                    self.config
                                        .workspace
                                        .programs_dir()
                                        .join(prog)
                                        .join(proj)
                                        .join(format!("{}.md", mil)),
                                )
                            } else {
                                None
                            }
                        }
                        "task" => {
                            if let (Some(prog), Some(proj), Some(mil), Some(t)) = (
                                &self.current_program,
                                &self.current_project,
                                &self.current_milestone,
                                &name,
                            ) {
                                Some(
                                    self.config
                                        .workspace
                                        .programs_dir()
                                        .join(prog)
                                        .join(proj)
                                        .join(mil)
                                        .join(format!("{}.md", t)),
                                )
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };

                    if let Some(target) = target_path {
                        if let Err(e) = self.config.workspace.create_from_template(
                            &template_name,
                            &target,
                            &state.values,
                            &state.strip_labels,
                        ) {
                            tracing::error!("Failed to create element: {}", e);
                        }
                    } else {
                        tracing::warn!(
                            "Could not compute target path for template: {}",
                            template_name
                        );
                    }

                    // Determine the name of the newly created element
                    let new_element_name: Option<String> = match template_name.as_str() {
                        "program" => program_name.clone(),
                        "project" => project_name.clone(),
                        "milestone" => milestone_name.clone(),
                        "task" => name.clone(),
                        _ => None,
                    };

                    // Clear template state before calling load_tree_view_data
                    self.template_field_state = None;

                    // Refresh the tree view to show the newly created element at current level
                    self.load_tree_view_data();

                    // Find and select the newly created element in the sidebar
                    // Stay at parent level (don't auto-navigate into new element)
                    if let Some(ref element_name) = new_element_name {
                        // Find the newly created element in sidebar_items
                        if let Some(pos) = self
                            .sidebar_items
                            .iter()
                            .position(|item| !item.is_header && item.name == *element_name)
                        {
                            self.selected_entry_index = pos;
                        }
                    }

                    self.current_view = ViewType::TreeView;
                }
                WizardFocus::Field(idx) => {
                    // Save current field value
                    if let Some(field) = state.fields.get_mut(idx) {
                        field.value = self.input_buffer.clone();
                    }

                    // Find next editable field or move to CONFIRM button
                    let mut next_idx = idx + 1;
                    while next_idx < state.fields.len() {
                        if state.fields[next_idx].is_editable {
                            state.focus = WizardFocus::Field(next_idx);
                            if let Some(field) = state.fields.get(next_idx) {
                                self.input_buffer = field.value.clone();
                            }
                            return;
                        }
                        next_idx += 1;
                    }
                    // No more editable fields, move to CONFIRM button
                    state.focus = WizardFocus::ConfirmButton;
                    self.input_buffer.clear();
                }
            }
        }
    }

    // TODO: These methods are helpers for future keyboard shortcuts for quick navigation
    #[allow(dead_code)]
    fn select_program(&mut self) {
        if let Some(entry) = self.programs.get(self.selected_entry_index) {
            self.current_program = Some(entry.name.clone());
            self.current_project = None;
            self.current_milestone = None;
            self.show_projects_list();
        }
    }

    #[allow(dead_code)]
    fn select_project(&mut self) {
        if let Some(entry) = self.projects.get(self.selected_entry_index) {
            self.current_project = Some(entry.name.clone());
            self.current_milestone = None;
            self.show_milestones_list();
        }
    }

    #[allow(dead_code)]
    fn select_milestone(&mut self) {
        if let Some(entry) = self.milestones.get(self.selected_entry_index) {
            self.current_milestone = Some(entry.name.clone());
            self.show_tasks_list();
        }
    }

    #[allow(dead_code)]
    fn open_selected_task(&mut self) {
        if let Some(entry) = self.tasks.get(self.selected_entry_index) {
            let path = entry.path.clone();
            self.launch_editor(&path);
        }
    }

    fn filter_commands(&mut self) {
        self.command_matches = command::filter_commands(
            &self.command_input,
            self.current_program.as_deref(),
            self.current_project.as_deref(),
            self.current_milestone.as_deref(),
            !self.programs.is_empty(),
        );
        self.command_selection_index = 0;
    }

    fn draw(&self, f: &mut Frame) {
        layout::render(f, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_mode_is_normal() {
        let config = crate::config::Config::default();
        let app = App::new(config);
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn test_slash_enters_command_palette_mode() {
        let config = crate::config::Config::default();
        let mut app = App::new(config);

        // Simulate pressing '/'
        app.handle_key(KeyCode::Char('/'));

        assert!(matches!(app.mode, Mode::CommandPalette));
    }

    #[test]
    fn test_esc_exits_command_palette_mode() {
        let config = crate::config::Config::default();
        let mut app = App::new(config);

        // First enter command palette mode
        app.handle_key(KeyCode::Char('/'));
        assert!(matches!(app.mode, Mode::CommandPalette));

        // Then press Esc to exit (from command input handler since mode is CommandPalette)
        app.handle_command_input(KeyCode::Esc);

        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn test_enter_returns_to_normal_mode() {
        let config = crate::config::Config::default();
        let mut app = App::new(config);

        // Enter command palette mode
        app.handle_key(KeyCode::Char('/'));
        assert!(matches!(app.mode, Mode::CommandPalette));

        // Press Enter (should execute command and return to normal)
        app.handle_command_input(KeyCode::Enter);

        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn test_command_palette_has_new_program_with_empty_workspace() {
        // Simulate an empty workspace scenario
        let mut config = crate::config::Config::default();
        // Set workspace to a temp directory (simulating empty workspace)
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        config.workspace = temp_dir.path().to_path_buf();

        let mut app = App::new(config);

        // Verify programs list is empty
        assert!(
            app.programs.is_empty(),
            "Programs should be empty in new workspace"
        );

        // Verify sidebar has items (Planning and Journal sections should exist)
        assert!(
            !app.sidebar_items.is_empty(),
            "Sidebar should have items even with empty programs"
        );

        // Open command palette
        app.handle_key(KeyCode::Char('/'));
        assert!(matches!(app.mode, Mode::CommandPalette));

        // Verify "New Program" is in the command list
        assert!(
            app.command_matches.iter().any(|c| c.label == "New Program"),
            "New Program command should be available even with empty workspace"
        );

        // Verify we can navigate the command list
        assert!(
            !app.command_matches.is_empty(),
            "Command list should not be empty"
        );

        // Verify we can select "New Program" command
        let new_program_idx = app
            .command_matches
            .iter()
            .position(|c| c.label == "New Program");
        assert!(
            new_program_idx.is_some(),
            "Should be able to find New Program command index"
        );

        // Navigate to New program command
        if let Some(idx) = new_program_idx {
            app.command_selection_index = idx;
            assert_eq!(app.command_matches[idx].label, "New Program");
        }
    }

    #[test]
    fn test_inline_editing_updates_field_value() {
        let config = crate::config::Config::default();
        let mut app = App::new(config);

        // Set up template field state for testing
        let fields = vec![
            FieldInfo {
                label: "Title".to_string(),
                placeholder: "TITLE".to_string(),
                value: String::new(),
                is_focused: true,
                is_editable: true,
                display_order: 0,
            },
            FieldInfo {
                label: "Status".to_string(),
                placeholder: "DEFAULT_STATUS".to_string(),
                value: "New".to_string(),
                is_focused: false,
                is_editable: false,
                display_order: 1,
            },
        ];

        app.template_field_state = Some(TemplateFieldState {
            template_name: "test".to_string(),
            target_path: None,
            fields,
            focus: WizardFocus::Field(0),
            values: std::collections::HashMap::new(),
            strip_labels: std::collections::HashSet::new(),
        });
        app.current_view = ViewType::InputTemplateField;
        app.input_buffer.clear();

        // Type some characters
        app.handle_key(KeyCode::Char('H'));
        app.handle_key(KeyCode::Char('e'));
        app.handle_key(KeyCode::Char('l'));
        app.handle_key(KeyCode::Char('l'));
        app.handle_key(KeyCode::Char('o'));

        // Verify both input_buffer and field.value are updated
        assert_eq!(app.input_buffer, "Hello");
        if let Some(state) = &app.template_field_state {
            assert_eq!(state.fields[0].value, "Hello");
        }

        // Test backspace
        app.handle_key(KeyCode::Backspace);
        assert_eq!(app.input_buffer, "Hell");
        if let Some(state) = &app.template_field_state {
            assert_eq!(state.fields[0].value, "Hell");
        }
    }

    #[test]
    fn test_navigator_refreshes_after_creating_element() {
        // Create a temp workspace
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        // Create a program directory manually so we have something to work with
        let programs_dir = workspace_path.join("programs");
        std::fs::create_dir_all(&programs_dir).expect("Failed to create programs dir");

        // Create a simple program file
        let program_content = r#"---
uuid: test-uuid
title: TestProgram
status: New
tags: program
---

# DESCRIPTION
Test description
"#;
        std::fs::write(programs_dir.join("TestProgram.md"), program_content)
            .expect("Failed to create program file");

        // Set up config with temp workspace
        let mut config = crate::config::Config::default();
        config.workspace = workspace_path.clone();

        let mut app = App::new(config);

        // Verify we're at root level with programs loaded
        assert!(!app.programs.is_empty(), "Programs should be loaded");
        assert_eq!(app.tree_state.path.len(), 0, "Should be at root level");

        // Navigate into the program (select index 1 because index 0 is "Programs" header)
        app.selected_entry_index = 1;
        app.open_tree_item();

        // Verify we're now inside the program
        assert_eq!(app.tree_state.path.len(), 1, "Should be inside program");
        assert!(
            app.current_program.is_some(),
            "Current program should be set"
        );

        // Now simulate creating a new project via the wizard
        // First, start the new project wizard
        app.start_new_project();

        // Verify we're in template field input mode
        assert_eq!(app.current_view, ViewType::InputTemplateField);
        assert!(app.template_field_state.is_some());

        // Fill in the project name in the first editable field
        if let Some(ref mut state) = app.template_field_state {
            // Find first editable field and set its value
            for field in &mut state.fields {
                if field.is_editable {
                    field.value = "NewProject".to_string();
                    break;
                }
            }
            // Set focus to ConfirmButton to simulate pressing tab through all fields
            state.focus = WizardFocus::ConfirmButton;
        }
        app.input_buffer = "NewProject".to_string();

        // Confirm the creation
        app.confirm_template_field();

        // After creation, verify the navigator was refreshed
        // NEW BEHAVIOR: Stay at parent level (don't auto-navigate into new element)
        // The user can manually navigate into it with arrow key
        assert_eq!(
            app.tree_state.path.len(),
            1,
            "Should stay at parent level (program) after creation"
        );
        assert!(
            app.current_project.is_none(),
            "Should NOT auto-navigate into project - current_project should be None"
        );

        // Verify we're back in TreeView
        assert_eq!(app.current_view, ViewType::TreeView);

        // The key test: verify that sidebar_items reflects the current state
        // After creating a project, we should still be in the program, showing projects
        assert!(
            !app.sidebar_items.is_empty(),
            "Sidebar should have items after creation"
        );

        // Verify the new project is in the sidebar and selected
        let has_new_project = app
            .sidebar_items
            .iter()
            .any(|item| !item.is_header && item.name == "NewProject");
        assert!(
            has_new_project,
            "Newly created project should appear in sidebar"
        );
    }

    #[test]
    fn test_wizard_creates_program_file_on_disk() {
        // Create a temp workspace
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        // Set up config with temp workspace
        let mut config = crate::config::Config::default();
        config.workspace = workspace_path.clone();

        let mut app = App::new(config);

        // Start the new program wizard
        app.start_new_program();

        // Verify we're in template field input mode
        assert_eq!(app.current_view, ViewType::InputTemplateField);
        assert!(app.template_field_state.is_some());

        // Fill in the program name in the first editable field
        if let Some(ref mut state) = app.template_field_state {
            // Find first editable field and set its value
            for field in &mut state.fields {
                if field.is_editable {
                    field.value = "MyNewProgram".to_string();
                    break;
                }
            }
            // Set focus to ConfirmButton
            state.focus = WizardFocus::ConfirmButton;
        }
        app.input_buffer = "MyNewProgram".to_string();

        // Confirm the creation
        app.confirm_template_field();

        // Verify the file was created on disk
        let program_path = workspace_path.join("programs").join("MyNewProgram.md");
        assert!(
            program_path.exists(),
            "Program file should be created at {:?}",
            program_path
        );

        // Verify the file has content from the template
        let content = std::fs::read_to_string(&program_path).expect("Should be able to read file");
        assert!(
            content.contains("MyNewProgram"),
            "Program file should contain the name"
        );
    }

    #[test]
    fn test_wizard_creates_project_file_on_disk() {
        // Create a temp workspace
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        // Create a program directory manually so we have something to work with
        let programs_dir = workspace_path.join("programs");
        std::fs::create_dir_all(&programs_dir).expect("Failed to create programs dir");

        // Create a simple program file
        let program_content = r#"---
uuid: test-uuid
title: TestProgram
status: New
tags: program
---

# DESCRIPTION
Test description
"#;
        std::fs::write(programs_dir.join("TestProgram.md"), program_content)
            .expect("Failed to create program file");

        // Set up config with temp workspace
        let mut config = crate::config::Config::default();
        config.workspace = workspace_path.clone();

        let mut app = App::new(config);

        // Navigate into the program
        app.selected_entry_index = 1;
        app.open_tree_item();

        // Start the new project wizard
        app.start_new_project();

        // Fill in the project name
        if let Some(ref mut state) = app.template_field_state {
            for field in &mut state.fields {
                if field.is_editable {
                    field.value = "NewProject".to_string();
                    break;
                }
            }
            state.focus = WizardFocus::ConfirmButton;
        }
        app.input_buffer = "NewProject".to_string();

        // Confirm the creation
        app.confirm_template_field();

        // Verify the file was created on disk
        let project_path = workspace_path
            .join("programs")
            .join("TestProgram")
            .join("NewProject.md");
        assert!(
            project_path.exists(),
            "Project file should be created at {:?}",
            project_path
        );

        // Verify the file has content from the template
        let content = std::fs::read_to_string(&project_path).expect("Should be able to read file");
        assert!(
            content.contains("NewProject"),
            "Project file should contain the name"
        );
    }

    #[test]
    fn test_wizard_creates_milestone_file_on_disk() {
        // Create a temp workspace with program and project
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let programs_dir = workspace_path.join("programs");
        let project_dir = programs_dir.join("TestProgram");
        std::fs::create_dir_all(&project_dir).expect("Failed to create directories");

        // Create program file
        std::fs::write(
            programs_dir.join("TestProgram.md"),
            "---\ntitle: TestProgram\nstatus: New\n---\n",
        )
        .expect("Failed to create program file");

        // Create project file
        std::fs::write(
            project_dir.join("NewProject.md"),
            "---\ntitle: NewProject\nstatus: New\n---\n",
        )
        .expect("Failed to create project file");

        // Set up config with temp workspace
        let mut config = crate::config::Config::default();
        config.workspace = workspace_path.clone();

        let mut app = App::new(config);

        // Navigate into program, then project
        app.selected_entry_index = 1;
        app.open_tree_item();
        app.selected_entry_index = 1;
        app.open_tree_item();

        // Start the new milestone wizard
        app.start_new_milestone();

        // Fill in the milestone name
        if let Some(ref mut state) = app.template_field_state {
            for field in &mut state.fields {
                if field.is_editable {
                    field.value = "NewMilestone".to_string();
                    break;
                }
            }
            state.focus = WizardFocus::ConfirmButton;
        }
        app.input_buffer = "NewMilestone".to_string();

        // Confirm the creation
        app.confirm_template_field();

        // Verify the file was created on disk
        let milestone_path = workspace_path
            .join("programs")
            .join("TestProgram")
            .join("NewProject")
            .join("NewMilestone.md");
        assert!(
            milestone_path.exists(),
            "Milestone file should be created at {:?}",
            milestone_path
        );

        // Verify the file has content
        let content =
            std::fs::read_to_string(&milestone_path).expect("Should be able to read file");
        assert!(
            content.contains("NewMilestone"),
            "Milestone file should contain the name"
        );
    }

    #[test]
    fn test_wizard_creates_task_file_on_disk() {
        // Create a temp workspace with program, project, and milestone
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let milestone_dir = workspace_path
            .join("programs")
            .join("TestProgram")
            .join("NewProject")
            .join("NewMilestone");
        std::fs::create_dir_all(&milestone_dir).expect("Failed to create directories");

        // Create program file
        std::fs::write(
            workspace_path.join("programs").join("TestProgram.md"),
            "---\ntitle: TestProgram\nstatus: New\n---\n",
        )
        .expect("Failed to create program file");

        // Create project file
        std::fs::write(
            milestone_dir.parent().unwrap().join("NewProject.md"),
            "---\ntitle: NewProject\nstatus: New\n---\n",
        )
        .expect("Failed to create project file");

        // Create milestone file
        std::fs::write(
            milestone_dir.join("NewMilestone.md"),
            "---\ntitle: NewMilestone\nstatus: New\n---\n",
        )
        .expect("Failed to create milestone file");

        // Set up config with temp workspace
        let mut config = crate::config::Config::default();
        config.workspace = workspace_path.clone();

        let mut app = App::new(config);

        // Navigate into program, then project, then milestone
        app.selected_entry_index = 1;
        app.open_tree_item();
        app.selected_entry_index = 1;
        app.open_tree_item();
        app.selected_entry_index = 1;
        app.open_tree_item();

        // Start the new task wizard
        app.start_new_task();

        // Fill in the task name
        if let Some(ref mut state) = app.template_field_state {
            for field in &mut state.fields {
                if field.is_editable {
                    field.value = "NewTask".to_string();
                    break;
                }
            }
            state.focus = WizardFocus::ConfirmButton;
        }
        app.input_buffer = "NewTask".to_string();

        // Confirm the creation
        app.confirm_template_field();

        // Verify the file was created on disk
        let task_path = workspace_path
            .join("programs")
            .join("TestProgram")
            .join("NewProject")
            .join("NewMilestone")
            .join("NewTask.md");
        assert!(
            task_path.exists(),
            "Task file should be created at {:?}",
            task_path
        );

        // Verify the file has content
        let content = std::fs::read_to_string(&task_path).expect("Should be able to read file");
        assert!(
            content.contains("NewTask"),
            "Task file should contain the name"
        );
    }

    #[test]
    fn test_navigator_selection_stays_on_newly_created_element() {
        // Test that after creating an element, the navigator selects the new element
        // instead of resetting to the first item

        // Create a temp workspace
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        // Create a program directory manually so we have something to work with
        let programs_dir = workspace_path.join("programs");
        std::fs::create_dir_all(&programs_dir).expect("Failed to create programs dir");

        // Create a simple program file
        let program_content = r#"---
uuid: test-uuid
title: TestProgram
status: New
tags: program
---

# DESCRIPTION
Test description
"#;
        std::fs::write(programs_dir.join("TestProgram.md"), program_content)
            .expect("Failed to create program file");

        // Set up config with temp workspace
        let mut config = crate::config::Config::default();
        config.workspace = workspace_path.clone();

        let mut app = App::new(config);

        // We're at root level - verify there are programs
        assert!(!app.programs.is_empty(), "Programs should be loaded");

        // Record the initial selection position (before creating new element)
        let _initial_selected_index = app.selected_entry_index;

        // Start the new program wizard
        app.start_new_program();

        // Fill in the program name in the first editable field
        if let Some(ref mut state) = app.template_field_state {
            for field in &mut state.fields {
                if field.is_editable && field.placeholder == "NAME" {
                    field.value = "NewProgram".to_string();
                    break;
                }
            }
            state.focus = WizardFocus::ConfirmButton;
        }
        app.input_buffer = "NewProgram".to_string();

        // Confirm the creation
        app.confirm_template_field();

        // Now verify selection is on the newly created element, NOT reset to first item
        // The new element should be in the sidebar
        let new_element_in_sidebar = app
            .sidebar_items
            .iter()
            .any(|item| item.name == "NewProgram");

        assert!(
            new_element_in_sidebar,
            "Newly created element should be in sidebar"
        );

        // The key assertion: selected_entry_index should point to the new element
        let selected_item = &app.sidebar_items[app.selected_entry_index];
        assert_eq!(
            selected_item.name, "NewProgram",
            "Selected item should be the newly created program, but got '{}' (index {})",
            selected_item.name, app.selected_entry_index
        );

        // Also verify we didn't just reset to initial position (index 1)
        // The new element should NOT be at index 1 if there's an existing program
        // (index 1 should be the first existing element "TestProgram", not "NewProgram")
    }

    #[test]
    fn test_wizard_description_in_markdown_body() {
        // Test that DESCRIPTION field appears in wizard and is placed in markdown body

        // Create a temp workspace
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        // Set up config with temp workspace
        let mut config = crate::config::Config::default();
        config.workspace = workspace_path.clone();

        let mut app = App::new(config);

        // Start the new program wizard
        app.start_new_program();

        // Verify we're in template field input mode
        assert_eq!(app.current_view, ViewType::InputTemplateField);
        assert!(app.template_field_state.is_some());

        // Check that DESCRIPTION is in the wizard fields
        let has_description_field = app
            .template_field_state
            .as_ref()
            .map(|state| state.fields.iter().any(|f| f.placeholder == "DESCRIPTION"))
            .unwrap_or(false);

        assert!(
            has_description_field,
            "Wizard should have DESCRIPTION field"
        );

        // Fill in the program name and description
        if let Some(ref mut state) = app.template_field_state {
            for field in &mut state.fields {
                if field.placeholder == "NAME" {
                    field.value = "TestProgram".to_string();
                } else if field.placeholder == "DESCRIPTION" {
                    field.value = "This is a test description".to_string();
                }
            }
            state.focus = WizardFocus::ConfirmButton;
        }
        app.input_buffer = "TestProgram".to_string();

        // Confirm the creation
        app.confirm_template_field();

        // Verify the file was created on disk
        let program_path = workspace_path.join("programs").join("TestProgram.md");
        assert!(program_path.exists(), "Program file should be created");

        // Read the content and verify DESCRIPTION is in the markdown body
        let content = std::fs::read_to_string(&program_path).expect("Should be able to read file");

        // Verify description appears in markdown body (after YAML separator)
        assert!(
            content.contains("This is a test description"),
            "Description should appear in markdown body, got: {}",
            content
        );

        // Verify description is NOT in YAML frontmatter
        // The YAML section is between the two --- markers
        let yaml_section = content.split("---").nth(1).unwrap_or("");
        assert!(
            !yaml_section.to_lowercase().contains("description:"),
            "Description should NOT be in YAML frontmatter, YAML section was: {}",
            yaml_section
        );
    }
}
