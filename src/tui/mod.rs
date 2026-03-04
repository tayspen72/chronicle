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
                    // From CONFIRM, go to last field
                    if !state.fields.is_empty() {
                        state.focus = WizardFocus::Field(state.fields.len() - 1);
                        if let Some(field) = state.fields.last() {
                            self.input_buffer = field.value.clone();
                        }
                    }
                }
                WizardFocus::Field(idx) => {
                    if idx > 0 {
                        state.focus = WizardFocus::Field(idx - 1);
                        if let Some(field) = state.fields.get(idx - 1) {
                            self.input_buffer = field.value.clone();
                        }
                    }
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
                    if idx < state.fields.len() - 1 {
                        state.focus = WizardFocus::Field(idx + 1);
                        if let Some(field) = state.fields.get(idx + 1) {
                            self.input_buffer = field.value.clone();
                        }
                    } else {
                        // Last field, move to CONFIRM button
                        state.focus = WizardFocus::ConfirmButton;
                        self.input_buffer.clear();
                    }
                }
                WizardFocus::ConfirmButton => {
                    // From CONFIRM, go to CANCEL
                    state.focus = WizardFocus::CancelButton;
                }
                WizardFocus::CancelButton => {
                    // From CANCEL, wrap to first field
                    if !state.fields.is_empty() {
                        state.focus = WizardFocus::Field(0);
                        if let Some(field) = state.fields.first() {
                            self.input_buffer = field.value.clone();
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
                if let Some(ref state) = self.template_field_state {
                    if let WizardFocus::Field(idx) = state.focus {
                        if let Some(field) = state.fields.get(idx) {
                            if field.is_editable {
                                self.input_buffer.push(c);
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
                if let Some(ref state) = self.template_field_state {
                    if let WizardFocus::Field(idx) = state.focus {
                        if let Some(field) = state.fields.get(idx) {
                            if field.is_editable {
                                self.input_buffer.pop();
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
        self.input_buffer.clear();
        self.current_view = ViewType::InputProgram;
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
        // Now start the input for new project
        self.input_buffer.clear();
        self.current_view = ViewType::InputProject;
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
        self.input_buffer.clear();
        self.current_view = ViewType::InputMilestone;
    }

    fn start_new_task(&mut self) {
        self.input_buffer.clear();
        self.current_view = ViewType::InputTask;
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
        let keywords = ["NAME", "TODAY", "DEFAULT_STATUS", "OWNER"];

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
        let keywords = ["NAME", "TODAY", "DEFAULT_STATUS", "OWNER"];

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
        let keywords = ["NAME", "TODAY", "DEFAULT_STATUS", "OWNER"];

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
        let keywords = ["NAME", "TODAY", "DEFAULT_STATUS", "OWNER"];

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
                    let program_name = state.values.get("PROGRAM_NAME").cloned();
                    let project_name = state.values.get("PROJECT_NAME").cloned();
                    let milestone_name = state.values.get("MILESTONE_NAME").cloned();

                    if let Some(ref target) = state.target_path {
                        let _ = self.config.workspace.create_from_template(
                            &template_name,
                            target,
                            &state.values,
                            &state.strip_labels,
                        );
                    }

                    // Clear template state before calling load_tree_view_data
                    self.template_field_state = None;

                    // Refresh the tree view
                    self.load_tree_view_data();

                    // Navigate to the new item
                    match template_name.as_str() {
                        "program" => {
                            if let Some(name) = program_name {
                                self.current_program = Some(name.clone());
                                self.tree_state.path.push(name);
                            }
                        }
                        "project" => {
                            if let Some(name) = project_name {
                                self.current_project = Some(name.clone());
                                self.tree_state.path.push(name);
                            }
                        }
                        "milestone" => {
                            if let Some(name) = milestone_name {
                                self.current_milestone = Some(name.clone());
                                self.tree_state.path.push(name);
                            }
                        }
                        _ => {}
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
}
