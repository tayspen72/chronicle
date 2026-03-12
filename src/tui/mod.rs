pub mod command;
pub mod layout;
pub mod navigation;
pub mod tree;
pub mod views;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
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
use crate::storage::{
    validate_element_name, DirectoryEntry, JournalEntry, JournalStorage, WorkspaceStorage,
};
use chrono::Local;
use command::{get_command_list, CommandAction, CommandMatch};
use navigation::{SidebarItem, SidebarSection};
use tree::TreeModel;

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
    pub tree_model: TreeModel,
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
    // Planning session state
    pub planning_session_active: bool,
    pub planning_session_tasks: Vec<String>, // UUIDs or paths of selected tasks
    pub planning_session_start_date: Option<String>,
    pub planning_session_end_date: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let command_matches = get_command_list();

        let mut app = App {
            config,
            current_view: ViewType::TreeView,
            tree_model: TreeModel::default(),
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
            planning_session_active: false,
            planning_session_tasks: Vec::new(),
            planning_session_start_date: None,
            planning_session_end_date: None,
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

            if event::poll(std::time::Duration::from_millis(16))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                if matches!(self.mode, Mode::CommandPalette) {
                    self.handle_command_input(key.code);
                } else {
                    self.handle_key(key.code);
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
                        if let WizardFocus::Field(idx) = state.focus
                            && let Some(field) = state.fields.get_mut(idx)
                        {
                            field.value = self.input_buffer.clone();
                        }
                        state.focus = WizardFocus::CancelButton;
                        self.input_buffer.clear();
                    }
                } else if self.current_view == ViewType::TreeView {
                    // In TreeView, only navigate back if we're at root (empty path).
                    // When inside the tree (path not empty), do nothing because Left arrow
                    // should handle navigation up, and due to a crossterm bug, arrow keys
                    // can sometimes incorrectly trigger ESC first (the escape sequence parsing
                    // issue causes the ESC byte of the escape sequence to be interpreted as a
                    // separate keypress). By doing nothing, we prevent double-navigation.
                    if self.tree_model.selected_path().is_empty() {
                        self.current_view = ViewType::Journal;
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
            if let WizardFocus::Field(idx) = state.focus
                && let Some(field) = state.fields.get_mut(idx)
            {
                field.value = self.input_buffer.clone();
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
            if let WizardFocus::Field(idx) = state.focus
                && let Some(field) = state.fields.get_mut(idx)
            {
                field.value = self.input_buffer.clone();
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
            tracing::debug!(
                selected_index = self.selected_entry_index,
                path = ?self.tree_model.selected_path(),
                "navigate right"
            );
            self.open_tree_item_with_leaf_open(false);
        }
    }

    fn navigate_left(&mut self) {
        if self.current_view != ViewType::TreeView {
            return;
        }

        let Some(item) = self.sidebar_items.get(self.selected_entry_index) else {
            return;
        };
        let Some(selected_path) = item.tree_path.clone() else {
            return;
        };

        tracing::debug!(path = ?selected_path, "navigate left from selected path");
        if selected_path.is_empty() {
            return;
        }

        self.collapse_path(&selected_path);
        let mut parent = selected_path;
        parent.pop();
        self.collapse_path(&parent);
        self.set_selected_tree_path(parent);
        self.load_tree_view_data();
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
                if !self.tree_model.selected_path().is_empty() {
                    let mut parent = self.tree_model.selected_path_vec();
                    parent.pop();
                    self.set_selected_tree_path(parent);
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

    fn open_tree_item(&mut self) {
        self.open_tree_item_with_leaf_open(true);
    }

    fn open_tree_item_with_leaf_open(&mut self, open_leaf_content: bool) {
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
            let _ = plan_type;
            // Planning entries are placeholders for now; Enter should do nothing.
            return;
        }

        if let Some(journal_action) = &item.is_journal_item {
            match journal_action.as_str() {
                "Today" => {
                    let today_path = self.config.workspace.today_journal_path();
                    if today_path.exists() {
                        // Content preview is already rendered inline in TreeView.
                        self.current_view = ViewType::TreeView;
                    } else {
                        match self.create_today_journal_from_template() {
                            Ok(path) => self.launch_editor(&path),
                            Err(e) => tracing::error!("Failed to create today's journal: {}", e),
                        }
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
            let node_path = item
                .tree_path
                .clone()
                .unwrap_or_else(|| self.path_for_sidebar_item(item));
            let entry = DirectoryEntry {
                name: item.name.clone(),
                path: path.clone(),
                is_dir: false,
            };

            let has_children = item.has_children;
            tracing::debug!(
                item = %item.name,
                indent = item.indent,
                selected_index = idx,
                node_path = ?node_path,
                current_path = ?self.tree_model.selected_path(),
                has_children,
                "open tree item"
            );
            if has_children || self.tree_model.selected_path() != node_path {
                if has_children {
                    self.tree_model.expand_path(&node_path);
                    self.set_selected_tree_path(node_path.clone());
                    self.load_tree_view_data();
                    self.select_first_child_for_path(&node_path);
                } else {
                    self.set_selected_tree_path(node_path.clone());
                    self.load_tree_view_data();
                }
            } else if open_leaf_content {
                self.set_selected_tree_path(node_path);
                self.open_content(&entry);
            }
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
        self.programs = self.load_tree_level(&[]);
        self.current_program = self.tree_model.selected_path().first().cloned();
        self.current_project = self.tree_model.selected_path().get(1).cloned();
        self.current_milestone = self.tree_model.selected_path().get(2).cloned();
        self.current_task = self.tree_model.selected_path().get(3).cloned();

        self.projects = self.load_tree_level_for_selected_depth(1);
        self.milestones = self.load_tree_level_for_selected_depth(2);
        self.tasks = self.load_tree_level_for_selected_depth(3);
        self.subtasks = self.load_tree_level_for_selected_depth(4);

        tracing::debug!(
            path = ?self.tree_model.selected_path(),
            programs = self.programs.len(),
            projects = self.projects.len(),
            milestones = self.milestones.len(),
            tasks = self.tasks.len(),
            subtasks = self.subtasks.len(),
            "loaded tree view data"
        );
        self.build_sidebar_items();
        self.sync_selection_with_tree_path();
    }

    fn path_for_sidebar_item(&self, item: &SidebarItem) -> Vec<String> {
        let mut node_path = self.tree_model.selected_path_vec();
        let truncate_to = item.indent.min(node_path.len());
        node_path.truncate(truncate_to);
        node_path.push(item.name.clone());
        node_path
    }

    fn set_selected_tree_path(&mut self, path: Vec<String>) {
        self.tree_model.set_selected_path(path.clone());
        self.tree_model.expand_ancestors(&path);
    }

    fn collapse_path(&mut self, path: &[String]) {
        self.tree_model.collapse_path(path);
    }

    fn load_tree_level_for_selected_depth(&self, depth: usize) -> Vec<DirectoryEntry> {
        if self.tree_model.selected_depth() < depth {
            return Vec::new();
        }
        self.load_tree_level(&self.tree_model.selected_path()[..depth])
    }

    fn load_tree_level(&self, path: &[String]) -> Vec<DirectoryEntry> {
        if path.is_empty() {
            return match self.config.workspace.list_programs() {
                Ok(entries) => Self::dedupe_and_sort_entries(entries),
                Err(e) => {
                    tracing::warn!("Failed to list programs: {}", e);
                    Vec::new()
                }
            };
        }

        let Some(current_entry) = self.resolve_entry_at_path(path) else {
            tracing::warn!(requested_path = ?path, "failed to resolve tree path");
            return Vec::new();
        };
        self.list_children_for_entry(&current_entry)
    }

    fn resolve_entry_at_path(&self, path: &[String]) -> Option<DirectoryEntry> {
        if path.is_empty() {
            return None;
        }

        let mut entries = self.load_tree_level(&[]);
        let mut current: Option<DirectoryEntry> = None;
        for node in path {
            let found = entries.iter().find(|entry| &entry.name == node)?.clone();
            current = Some(found.clone());
            entries = self.list_children_for_entry(&found);
        }
        current
    }

    fn list_children_for_entry(&self, entry: &DirectoryEntry) -> Vec<DirectoryEntry> {
        let Some(parent_dir) = entry.path.parent() else {
            return Vec::new();
        };

        let mut base_dirs = Vec::new();
        if parent_dir
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == entry.name)
            .unwrap_or(false)
        {
            base_dirs.push(parent_dir.to_path_buf());
        }

        let sibling_named_dir = parent_dir.join(&entry.name);
        if sibling_named_dir.exists() {
            base_dirs.push(sibling_named_dir);
        }

        let mut all_children = Vec::new();
        for base in base_dirs {
            for child in Self::discover_children_from_base(&base, &entry.name) {
                all_children.push(child);
            }
        }
        let deduped = Self::dedupe_and_sort_entries(all_children);
        tracing::debug!(
            entry = entry.name,
            entry_path = %entry.path.display(),
            children = deduped.len(),
            "listed children for entry"
        );
        deduped
    }

    fn discover_children_from_base(base_dir: &Path, parent_name: &str) -> Vec<DirectoryEntry> {
        let mut entries = Vec::new();
        let container_dirs = ["projects", "milestones", "tasks", "subtasks"];
        let Ok(read_dir) = fs::read_dir(base_dir) else {
            return entries;
        };

        for candidate in read_dir.filter_map(|entry| entry.ok()) {
            let path = candidate.path();
            let Some(name) = path
                .file_name()
                .and_then(|file_name| file_name.to_str())
                .map(|name| name.to_string())
            else {
                continue;
            };

            if path.is_file() && name.ends_with(".md") {
                let stem = name.trim_end_matches(".md");
                if stem != parent_name {
                    entries.push(DirectoryEntry {
                        name: stem.to_string(),
                        path: path.clone(),
                        is_dir: false,
                    });
                }
                continue;
            }

            if !path.is_dir() {
                continue;
            }

            let own_md = path.join(format!("{name}.md"));
            if own_md.exists() {
                entries.push(DirectoryEntry {
                    name: name.clone(),
                    path: own_md,
                    is_dir: true,
                });
                continue;
            }

            if !container_dirs.contains(&name.as_str()) {
                continue;
            }

            let Ok(container_dir) = fs::read_dir(&path) else {
                continue;
            };
            for child in container_dir.filter_map(|entry| entry.ok()) {
                let child_path = child.path();
                let Some(child_name) = child_path
                    .file_name()
                    .and_then(|file_name| file_name.to_str())
                    .map(|name| name.to_string())
                else {
                    continue;
                };

                if child_path.is_file() && child_name.ends_with(".md") {
                    entries.push(DirectoryEntry {
                        name: child_name.trim_end_matches(".md").to_string(),
                        path: child_path,
                        is_dir: false,
                    });
                    continue;
                }

                if child_path.is_dir() {
                    let nested_md = child_path.join(format!("{child_name}.md"));
                    if nested_md.exists() {
                        entries.push(DirectoryEntry {
                            name: child_name,
                            path: nested_md,
                            is_dir: true,
                        });
                    }
                }
            }
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    fn dedupe_and_sort_entries(entries: Vec<DirectoryEntry>) -> Vec<DirectoryEntry> {
        let mut by_name: BTreeMap<String, DirectoryEntry> = BTreeMap::new();
        for entry in entries {
            match by_name.get(&entry.name) {
                Some(existing) => {
                    let replace = (entry.is_dir && !existing.is_dir)
                        || (entry.is_dir == existing.is_dir && entry.path < existing.path);
                    if replace {
                        by_name.insert(entry.name.clone(), entry);
                    }
                }
                None => {
                    by_name.insert(entry.name.clone(), entry);
                }
            }
        }
        by_name.into_values().collect()
    }

    fn first_selectable_sidebar_index(&self) -> usize {
        self.sidebar_items
            .iter()
            .position(|item| !item.is_header && !item.name.is_empty())
            .unwrap_or(0)
    }

    fn sync_selection_with_tree_path(&mut self) {
        let mut candidate = self.tree_model.selected_path_vec();
        while !candidate.is_empty() {
            if let Some(idx) = self
                .sidebar_items
                .iter()
                .position(|item| item.tree_path.as_ref() == Some(&candidate))
            {
                self.selected_entry_index = idx;
                if candidate != self.tree_model.selected_path() {
                    self.set_selected_tree_path(candidate.clone());
                }
                tracing::debug!(
                    selected_index = idx,
                    selected_name = ?self.tree_model.selected_path().last(),
                    "selection synced to tree path"
                );
                return;
            }
            candidate.pop();
        }
        tracing::warn!(
            path = ?self.tree_model.selected_path(),
            "selection sync fallback to first selectable item"
        );
        self.selected_entry_index = self.first_selectable_sidebar_index();
        if !self.tree_model.selected_path().is_empty() {
            if let Some(path) = self
                .sidebar_items
                .get(self.selected_entry_index)
                .and_then(|item| item.tree_path.clone())
            {
                self.set_selected_tree_path(path);
            } else {
                self.tree_model.set_selected_path(Vec::new());
            }
        }
    }

    fn select_first_child_for_path(&mut self, parent_path: &[String]) {
        if let Some(idx) = self.sidebar_items.iter().position(|item| {
            if item.is_header || item.name.is_empty() {
                return false;
            }
            let Some(path) = item.tree_path.as_ref() else {
                return false;
            };
            path.len() == parent_path.len() + 1 && path.starts_with(parent_path)
        }) {
            self.selected_entry_index = idx;
            if let Some(path) = self.sidebar_items[idx].tree_path.clone() {
                self.set_selected_tree_path(path);
                self.load_tree_view_data();
            }
        }
    }

    fn build_sidebar_items(&mut self) {
        self.sidebar_items.clear();
        self.sidebar_items
            .push(SidebarItem::new("Programs", SidebarSection::Programs).header());

        if self.programs.is_empty() {
            self.sidebar_items.push(
                SidebarItem::new("+ Create Program...", SidebarSection::Programs)
                    .indent(1)
                    .create_action(),
            );
        } else {
            self.push_tree_level_items(&[], 0);
        }

        self.sidebar_items
            .push(SidebarItem::new("", SidebarSection::Planning));
        self.sidebar_items
            .push(SidebarItem::new("Planning", SidebarSection::Planning).header());
        self.sidebar_items.push(
            SidebarItem::new("Weekly Planning", SidebarSection::Planning)
                .planning_item("WeeklyPlanning"),
        );
        self.sidebar_items
            .push(SidebarItem::new("Backlog", SidebarSection::Planning).planning_item("Backlog"));

        self.sidebar_items
            .push(SidebarItem::new("", SidebarSection::Journal));
        self.sidebar_items
            .push(SidebarItem::new("Journal", SidebarSection::Journal).header());
        self.sidebar_items
            .push(SidebarItem::new("Today", SidebarSection::Journal).journal_item("Today"));
        self.sidebar_items
            .push(SidebarItem::new("History", SidebarSection::Journal).journal_item("History"));
    }

    fn push_tree_level_items(&mut self, parent_path: &[String], depth: usize) {
        let entries = if depth == 0 {
            self.programs.clone()
        } else {
            self.load_tree_level(parent_path)
        };
        for entry in entries {
            let mut node_path = parent_path.to_vec();
            node_path.push(entry.name.clone());
            let has_children = !self.load_tree_level(&node_path).is_empty();
            self.sidebar_items.push(SidebarItem {
                name: entry.name.clone(),
                section: SidebarSection::Programs,
                is_header: false,
                is_planning_item: None,
                is_journal_item: None,
                indent: depth,
                path: Some(entry.path.clone()),
                tree_path: Some(node_path.clone()),
                has_children,
                is_create_action: false,
            });

            if self.tree_model.is_expanded(&node_path) {
                self.push_tree_level_items(&node_path, depth + 1);
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
                if let Some(ref mut state) = self.template_field_state
                    && let WizardFocus::Field(idx) = state.focus
                    && let Some(field) = state.fields.get_mut(idx)
                    && field.is_editable
                {
                    // Update both input_buffer and field.value for inline editing
                    self.input_buffer.push(c);
                    field.value.push(c);
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
                if let Some(ref mut state) = self.template_field_state
                    && let WizardFocus::Field(idx) = state.focus
                    && let Some(field) = state.fields.get_mut(idx)
                    && field.is_editable
                {
                    // Update both input_buffer and field.value for inline editing
                    self.input_buffer.pop();
                    field.value.pop();
                }
            }
            _ => {}
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
            Some(CommandAction::StartPlanningSession) => {
                self.start_planning_session();
            }
            Some(CommandAction::ClosePlanningSession) => {
                self.close_planning_session();
            }
            None => {
                self.current_view = cmd.view.clone();
            }
        }
    }

    fn open_today_journal(&mut self) {
        match self.create_today_journal_from_template() {
            Ok(path) => self.launch_editor(&path),
            Err(e) => eprintln!("Error opening today's journal: {}", e),
        }
    }

    fn create_today_journal_from_template(&self) -> Result<std::path::PathBuf> {
        let path = self.config.workspace.today_journal_path();
        if path.exists() {
            return Ok(path);
        }

        let mut values = std::collections::HashMap::new();
        values.insert(
            "TODAY".to_string(),
            Local::now().format("%Y-%m-%d").to_string(),
        );

        let strip_labels: std::collections::HashSet<String> = std::collections::HashSet::new();
        self.config
            .workspace
            .create_from_template("journal", &path, &values, &strip_labels)
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
        self.set_selected_tree_path(Vec::new());
        self.load_tree_view_data();
        self.current_view = ViewType::TreeView;
    }

    fn show_projects_list(&mut self) {
        if !self.tree_model.selected_path().is_empty() {
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        } else {
            self.set_selected_tree_path(Vec::new());
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        }
    }

    fn show_milestones_list(&mut self) {
        if self.tree_model.selected_depth() >= 2 {
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        } else {
            self.set_selected_tree_path(Vec::new());
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        }
    }

    fn show_tasks_list(&mut self) {
        if self.tree_model.selected_depth() >= 3 {
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        } else {
            self.set_selected_tree_path(Vec::new());
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        }
    }

    fn start_new_program(&mut self) {
        self.input_buffer.clear();
        self.open_template_wizard("program", None);
    }

    fn start_new_project(&mut self) {
        self.promote_selection_to_path_depth(1);
        self.input_buffer.clear();
        self.open_template_wizard("project", None);
    }

    fn start_new_milestone(&mut self) {
        self.promote_selection_to_path_depth(1);
        self.promote_selection_to_path_depth(2);
        self.input_buffer.clear();
        self.open_template_wizard("milestone", None);
    }

    fn start_new_task(&mut self) {
        self.promote_selection_to_path_depth(1);
        self.promote_selection_to_path_depth(2);
        self.promote_selection_to_path_depth(3);
        self.input_buffer.clear();
        self.open_template_wizard("task", None);
    }

    fn start_planning_session(&mut self) {
        if self.planning_session_active {
            println!("A planning session is already active. Please close it first.");
            return;
        }
        self.planning_session_active = true;
        self.planning_session_start_date =
            Some(chrono::Local::now().format("%Y-%m-%d").to_string());
        self.planning_session_end_date = Some(chrono::Local::now().format("%Y-%m-%d").to_string());
        self.planning_session_tasks.clear();
        self.current_view = ViewType::WeeklyPlanning;
    }

    fn close_planning_session(&mut self) {
        if !self.planning_session_active {
            println!("No active planning session to close.");
            return;
        }
        println!("Reviewing and closing planning session...");
        self.planning_session_active = false;
        self.planning_session_start_date = None;
        self.planning_session_end_date = None;
        self.planning_session_tasks.clear();
        self.current_view = ViewType::TreeView;
    }

    fn promote_selection_to_path_depth(&mut self, target_depth: usize) {
        if self.tree_model.selected_depth() >= target_depth {
            return;
        }

        let mut selected_path = match self.sidebar_items.get(self.selected_entry_index) {
            Some(item) if !item.is_header && !item.name.is_empty() => {
                self.path_for_sidebar_item(item)
            }
            _ => return,
        };

        if selected_path.len() < target_depth {
            let parent = self.tree_model.selected_path_vec();
            self.select_first_child_for_path(&parent);
            selected_path = match self.sidebar_items.get(self.selected_entry_index) {
                Some(item) if !item.is_header && !item.name.is_empty() => item
                    .tree_path
                    .clone()
                    .unwrap_or_else(|| self.path_for_sidebar_item(item)),
                _ => return,
            };
        }

        if selected_path.len() < target_depth {
            return;
        }

        selected_path.truncate(target_depth);
        self.set_selected_tree_path(selected_path);
        self.load_tree_view_data();
    }

    fn confirm_create_program(&mut self) {
        self.open_template_wizard("program", Some(self.input_buffer.clone()));
    }

    fn confirm_create_project(&mut self) {
        self.open_template_wizard("project", Some(self.input_buffer.clone()));
    }

    fn confirm_create_milestone(&mut self) {
        self.open_template_wizard("milestone", Some(self.input_buffer.clone()));
    }

    fn confirm_create_task(&mut self) {
        self.open_template_wizard("task", Some(self.input_buffer.clone()));
    }

    fn open_template_wizard(&mut self, template_name: &str, seeded_name: Option<String>) {
        if let Some(name) = seeded_name.as_deref()
            && let Err(e) = validate_element_name(name)
        {
            tracing::warn!("Invalid {} name '{}': {}", template_name, name, e);
            return;
        }

        let template = match template_name {
            "program" => include_str!("../../templates/program.md"),
            "project" => include_str!("../../templates/project.md"),
            "milestone" => include_str!("../../templates/milestone.md"),
            "task" => include_str!("../../templates/task.md"),
            "journal" => include_str!("../../templates/journal.md"),
            _ => {
                tracing::warn!("Unknown template requested: {}", template_name);
                return;
            }
        };
        let all_fields = crate::storage::parse_template_fields(template);
        tracing::debug!(
            template = template_name,
            seeded = seeded_name.is_some(),
            fields = all_fields.len(),
            "opening template wizard"
        );

        let mut values = std::collections::HashMap::new();
        values.insert(
            "TODAY".to_string(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );
        values.insert("OWNER".to_string(), self.config.owner.clone());
        if let Some(default_status) = self.config.workflow.first() {
            values.insert("DEFAULT_STATUS".to_string(), default_status.clone());
        }
        if let Some(name) = seeded_name.clone() {
            values.insert("NAME".to_string(), name.clone());
            match template_name {
                "program" => {
                    values.insert("PROGRAM_NAME".to_string(), name);
                }
                "project" => {
                    values.insert("PROJECT_NAME".to_string(), name);
                }
                "milestone" => {
                    values.insert("MILESTONE_NAME".to_string(), name);
                }
                "task" => {
                    values.insert("TASK_NAME".to_string(), name);
                }
                _ => {}
            }
        }

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|(_, _, strip)| *strip)
            .map(|(_, p, _)| p.clone())
            .collect();

        let seeded_keywords = ["NAME", "TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];
        let base_keywords = ["TODAY", "DEFAULT_STATUS", "OWNER", "UUID"];
        let keywords: &[&str] = if seeded_name.is_some() {
            &seeded_keywords
        } else {
            &base_keywords
        };

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
            template_name: template_name.to_string(),
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

    fn resolve_template_target_path(
        &self,
        template_name: &str,
        name: Option<&str>,
    ) -> Option<std::path::PathBuf> {
        match template_name {
            "program" => name.map(|program_name| {
                self.config
                    .workspace
                    .programs_dir()
                    .join(program_name)
                    .join(format!("{}.md", program_name))
            }),
            "project" => name.and_then(|project_name| {
                self.current_program.as_ref().map(|prog| {
                    self.config
                        .workspace
                        .programs_dir()
                        .join(prog)
                        .join("projects")
                        .join(project_name)
                        .join(format!("{}.md", project_name))
                })
            }),
            "milestone" => self
                .current_program
                .as_ref()
                .zip(self.current_project.as_ref())
                .and_then(|(prog, proj)| {
                    name.map(|milestone_name| {
                        self.config
                            .workspace
                            .programs_dir()
                            .join(prog)
                            .join("projects")
                            .join(proj)
                            .join("milestones")
                            .join(milestone_name)
                            .join(format!("{}.md", milestone_name))
                    })
                }),
            "task" => self
                .current_program
                .as_ref()
                .zip(self.current_project.as_ref())
                .zip(self.current_milestone.as_ref())
                .and_then(|((prog, proj), milestone)| {
                    name.map(|task_name| {
                        self.config
                            .workspace
                            .programs_dir()
                            .join(prog)
                            .join("projects")
                            .join(proj)
                            .join("milestones")
                            .join(milestone)
                            .join("tasks")
                            .join(task_name)
                            .join(format!("{}.md", task_name))
                    })
                }),
            "journal" => Some(self.config.workspace.today_journal_path()),
            _ => None,
        }
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
                    let values = state.values.clone();
                    let strip_labels = state.strip_labels.clone();
                    let name = values.get("NAME").cloned();
                    let candidate_name = name.as_deref();

                    if let Some(candidate_name) = candidate_name
                        && let Err(e) = validate_element_name(candidate_name)
                    {
                        tracing::warn!(
                            "Invalid {} name '{}': {}",
                            template_name,
                            candidate_name,
                            e
                        );
                        return;
                    }

                    let target_path =
                        self.resolve_template_target_path(&template_name, name.as_deref());

                    if let Some(target) = target_path {
                        if let Err(e) = self.config.workspace.create_from_template(
                            &template_name,
                            &target,
                            &values,
                            &strip_labels,
                        ) {
                            tracing::error!("Failed to create element: {}", e);
                        }
                    } else {
                        tracing::warn!(
                            "Could not compute target path for template: {}",
                            template_name
                        );
                    }

                    let new_element_name = name.clone();

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
        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };

        let mut app = App::new(config);

        // Verify we're at root level with programs loaded
        assert!(!app.programs.is_empty(), "Programs should be loaded");
        assert_eq!(
            app.tree_model.selected_depth(),
            0,
            "Should be at root level"
        );

        // Navigate into the program (select index 1 because index 0 is "Programs" header)
        app.selected_entry_index = 1;
        app.open_tree_item();

        // Verify we're now inside the program
        assert_eq!(
            app.tree_model.selected_depth(),
            1,
            "Should be inside program"
        );
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
            app.tree_model.selected_depth(),
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

        // Selected nodes keep children collapsed, so project children are not shown here.
        let has_new_project = app
            .sidebar_items
            .iter()
            .any(|item| !item.is_header && item.name == "NewProject");
        assert!(
            !has_new_project,
            "Selected program should keep project children collapsed"
        );
    }

    #[test]
    fn test_wizard_creates_program_file_on_disk() {
        // Create a temp workspace
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        // Set up config with temp workspace
        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };

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
        let program_path = workspace_path
            .join("programs")
            .join("MyNewProgram")
            .join("MyNewProgram.md");
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
        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };

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
            .join("projects")
            .join("NewProject")
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
        std::fs::create_dir_all(project_dir.join("projects").join("NewProject"))
            .expect("Failed to create project directories");
        std::fs::write(
            project_dir
                .join("projects")
                .join("NewProject")
                .join("NewProject.md"),
            "---\ntitle: NewProject\nstatus: New\n---\n",
        )
        .expect("Failed to create project file");

        // Set up config with temp workspace
        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };

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
            .join("projects")
            .join("NewProject")
            .join("milestones")
            .join("NewMilestone")
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
            .join("projects")
            .join("NewProject")
            .join("milestones")
            .join("NewMilestone");
        std::fs::create_dir_all(&milestone_dir).expect("Failed to create directories");

        // Create program file
        std::fs::write(
            workspace_path
                .join("programs")
                .join("TestProgram")
                .join("TestProgram.md"),
            "---\ntitle: TestProgram\nstatus: New\n---\n",
        )
        .expect("Failed to create program file");

        // Create project file
        std::fs::write(
            workspace_path
                .join("programs")
                .join("TestProgram")
                .join("projects")
                .join("NewProject")
                .join("NewProject.md"),
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
        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };

        let mut app = App::new(config);

        // Navigate into program, then project.
        // Right-navigation auto-selects first child, so after entering project
        // selection is already on the milestone.
        app.selected_entry_index = 1;
        app.open_tree_item();
        let project_idx = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "NewProject" && i.indent == 1)
            .expect("NewProject should be selectable");
        app.selected_entry_index = project_idx;
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
            .join("projects")
            .join("NewProject")
            .join("milestones")
            .join("NewMilestone")
            .join("tasks")
            .join("NewTask")
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
        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };

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
        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };

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
        let program_path = workspace_path
            .join("programs")
            .join("TestProgram")
            .join("TestProgram.md");
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

    #[test]
    fn test_navigate_left_from_milestone() {
        // Create temp workspace
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        // Create program: programs/TestProgram/TestProgram.md (nested structure)
        let program_dir = workspace_path.join("programs").join("TestProgram");
        std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
        std::fs::write(
            program_dir.join("TestProgram.md"),
            "---
title: TestProgram
---
# Test Program",
        )
        .expect("Failed to create program file");

        // Create project: programs/TestProgram/projects/TestProject/TestProject.md
        let project_dir = program_dir.join("projects").join("TestProject");
        std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
        std::fs::write(
            project_dir.join("TestProject.md"),
            "---
title: TestProject
---
# Test Project",
        )
        .expect("Failed to create project file");

        // Create milestone: programs/TestProgram/projects/TestProject/milestones/TestMilestone/TestMilestone.md
        let milestone_dir = project_dir.join("milestones").join("TestMilestone");
        std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
        std::fs::write(
            milestone_dir.join("TestMilestone.md"),
            "---
title: TestMilestone
---
# Test Milestone",
        )
        .expect("Failed to create milestone file");

        // Set up config with temp workspace
        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };

        let mut app = App::new(config);

        // Navigate into Program (select index 1 because index 0 is "Programs" header)
        app.selected_entry_index = 1;
        app.open_tree_item();

        // Navigate into Project
        let project_idx = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "TestProject")
            .expect("TestProject should be in sidebar");
        app.selected_entry_index = project_idx;
        app.open_tree_item();

        // Single right from project now expands and moves selection into milestone.
        assert_eq!(
            app.tree_model.selected_depth(),
            3,
            "Should be inside milestone after second navigation"
        );

        // Now press left arrow to navigate back
        app.navigate_left();

        // BUG: This should go to project level (path = ["TestProgram", "TestProject"])
        // but it jumps to program level (path = ["TestProgram"])
        assert_eq!(
            app.tree_model.selected_depth(),
            2,
            "Should go back to project level (depth 2), not program level (depth 1)"
        );
    }

    #[test]
    fn test_navigate_left_shows_correct_sidebar() {
        // Create temp workspace
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        // Create program: programs/TestProgram/TestProgram.md (nested structure)
        let program_dir = workspace_path.join("programs").join("TestProgram");
        std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
        std::fs::write(
            program_dir.join("TestProgram.md"),
            "---
title: TestProgram
---
# Test Program",
        )
        .expect("Failed to create program file");

        // Create project: programs/TestProgram/projects/TestProject/TestProject.md
        let project_dir = program_dir.join("projects").join("TestProject");
        std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
        std::fs::write(
            project_dir.join("TestProject.md"),
            "---
title: TestProject
---
# Test Project",
        )
        .expect("Failed to create project file");

        // Create milestone: programs/TestProgram/projects/TestProject/milestones/TestMilestone/TestMilestone.md
        let milestone_dir = project_dir.join("milestones").join("TestMilestone");
        std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
        std::fs::write(
            milestone_dir.join("TestMilestone.md"),
            "---
title: TestMilestone
---
# Test Milestone",
        )
        .expect("Failed to create milestone file");

        // Set up config with temp workspace
        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };

        let mut app = App::new(config);

        // Navigate into Program.
        // Right now expands and moves selection to the first project in one step.
        app.selected_entry_index = 1;
        app.open_tree_item();
        assert_eq!(app.tree_model.selected_depth(), 2);

        // Navigate into Project
        let project_idx = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "TestProject")
            .expect("TestProject should be in sidebar");
        app.selected_entry_index = project_idx;
        app.open_tree_item();
        assert_eq!(app.tree_model.selected_depth(), 3);

        // Now navigate LEFT - this should collapse back to project level
        app.navigate_left();

        // After collapsing, we should be at project level (depth 2)
        // path should be ["TestProgram", "TestProject"], not ["TestProgram"]
        assert_eq!(
            app.tree_model.selected_depth(),
            2,
            "After collapsing milestone, should be at project level (depth 2), not program level (depth 1)"
        );

        // The sidebar should keep milestones collapsed while project is selected.
        let has_milestones = app.sidebar_items.iter().any(|i| i.name == "TestMilestone");
        assert!(
            !has_milestones,
            "Sidebar should keep selected project's children collapsed"
        );

        // Selection should remain valid and on the project node after collapsing back.
        assert!(
            app.selected_entry_index < app.sidebar_items.len(),
            "Selected index should remain in bounds"
        );
        let selected = &app.sidebar_items[app.selected_entry_index];
        assert!(!selected.is_header, "Selection should not land on a header");
        assert_eq!(selected.name, "TestProject");
        assert_eq!(selected.indent, 1);
    }

    #[test]
    fn test_entering_program_does_not_auto_expand_first_project_children() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let program_dir = workspace_path.join("programs").join("TestProgram");
        std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
        std::fs::write(
            program_dir.join("TestProgram.md"),
            "---\ntitle: TestProgram\n---\n",
        )
        .expect("Failed to create program file");

        let alpha_project_dir = program_dir.join("projects").join("AlphaProject");
        std::fs::create_dir_all(&alpha_project_dir).expect("Failed to create alpha project dir");
        std::fs::write(
            alpha_project_dir.join("AlphaProject.md"),
            "---\ntitle: AlphaProject\n---\n",
        )
        .expect("Failed to create alpha project file");

        let beta_project_dir = program_dir.join("projects").join("BetaProject");
        std::fs::create_dir_all(&beta_project_dir).expect("Failed to create beta project dir");
        std::fs::write(
            beta_project_dir.join("BetaProject.md"),
            "---\ntitle: BetaProject\n---\n",
        )
        .expect("Failed to create beta project file");

        let milestone_dir = alpha_project_dir.join("milestones").join("M1");
        std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
        std::fs::write(milestone_dir.join("M1.md"), "---\ntitle: M1\n---\n")
            .expect("Failed to create milestone file");

        let config = crate::config::Config {
            workspace: workspace_path,
            ..crate::config::Config::default()
        };
        let mut app = App::new(config);

        app.selected_entry_index = 1;
        app.open_tree_item();

        assert_eq!(
            app.tree_model.selected_path().to_vec(),
            vec!["TestProgram".to_string(), "AlphaProject".to_string()]
        );
        assert!(
            !app.sidebar_items
                .iter()
                .any(|item| item.name == "M1" && item.indent == 2),
            "Milestones should not auto-expand when entering program level"
        );
    }

    #[test]
    fn test_navigate_left_collapses_children_of_newly_selected_parent() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let program_dir = workspace_path.join("programs").join("TestProgram");
        std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
        std::fs::write(
            program_dir.join("TestProgram.md"),
            "---\ntitle: TestProgram\n---\n",
        )
        .expect("Failed to create program file");

        let project_dir = program_dir.join("projects").join("TestProject");
        std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
        std::fs::write(
            project_dir.join("TestProject.md"),
            "---\ntitle: TestProject\n---\n",
        )
        .expect("Failed to create project file");

        let milestone_dir = project_dir.join("milestones").join("M1");
        std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
        std::fs::write(milestone_dir.join("M1.md"), "---\ntitle: M1\n---\n")
            .expect("Failed to create milestone file");

        let config = crate::config::Config {
            workspace: workspace_path,
            ..crate::config::Config::default()
        };
        let mut app = App::new(config);

        app.selected_entry_index = 1;
        app.open_tree_item_with_leaf_open(false);
        let project_idx = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "TestProject" && i.indent == 1)
            .expect("TestProject should be selectable");
        app.selected_entry_index = project_idx;
        app.open_tree_item_with_leaf_open(false);

        assert_eq!(
            app.tree_model.selected_path().to_vec(),
            vec![
                "TestProgram".to_string(),
                "TestProject".to_string(),
                "M1".to_string()
            ]
        );

        app.navigate_left();

        assert_eq!(
            app.tree_model.selected_path().to_vec(),
            vec!["TestProgram".to_string(), "TestProject".to_string()]
        );
        assert!(
            !app.sidebar_items
                .iter()
                .any(|item| item.name == "M1" && item.indent == 2),
            "Milestones should be collapsed when project is selected after left navigation"
        );
    }

    #[test]
    fn test_navigate_right_on_leaf_does_not_open_content_or_lose_selection() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let task_dir = workspace_path
            .join("programs")
            .join("TestProgram")
            .join("projects")
            .join("TestProject")
            .join("milestones")
            .join("M1")
            .join("tasks")
            .join("LeafTask");
        std::fs::create_dir_all(&task_dir).expect("Failed to create task dir");
        std::fs::write(
            workspace_path
                .join("programs")
                .join("TestProgram")
                .join("TestProgram.md"),
            "---\ntitle: TestProgram\n---\n",
        )
        .expect("Failed to create program file");
        std::fs::write(
            workspace_path
                .join("programs")
                .join("TestProgram")
                .join("projects")
                .join("TestProject")
                .join("TestProject.md"),
            "---\ntitle: TestProject\n---\n",
        )
        .expect("Failed to create project file");
        std::fs::write(
            workspace_path
                .join("programs")
                .join("TestProgram")
                .join("projects")
                .join("TestProject")
                .join("milestones")
                .join("M1")
                .join("M1.md"),
            "---\ntitle: M1\n---\n",
        )
        .expect("Failed to create milestone file");
        std::fs::write(task_dir.join("LeafTask.md"), "---\ntitle: LeafTask\n---\n")
            .expect("Failed to create task file");

        let config = crate::config::Config {
            workspace: workspace_path,
            ..crate::config::Config::default()
        };
        let mut app = App::new(config);

        app.selected_entry_index = 1;
        app.open_tree_item_with_leaf_open(false);
        let project_idx = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "TestProject" && i.indent == 1)
            .expect("TestProject should be selectable");
        app.selected_entry_index = project_idx;
        app.open_tree_item_with_leaf_open(false);
        let milestone_idx = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "M1" && i.indent == 2)
            .expect("M1 should be selectable");
        app.selected_entry_index = milestone_idx;
        app.open_tree_item_with_leaf_open(false);
        let task_idx = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "LeafTask" && i.indent == 3)
            .expect("LeafTask should be selectable after entering milestone");
        app.selected_entry_index = task_idx;
        app.open_tree_item_with_leaf_open(false);

        let selected_before = app.tree_model.selected_path().to_vec();
        app.navigate_right();
        app.navigate_right();

        assert_eq!(app.current_view, ViewType::TreeView);
        assert_eq!(app.tree_model.selected_path().to_vec(), selected_before);
        assert!(app.selected_entry_index < app.sidebar_items.len());
    }

    #[test]
    fn test_create_today_journal_from_template_populates_fields() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };
        let app = App::new(config);

        let today_path = app.config.workspace.today_journal_path();
        assert!(!today_path.exists());

        let created = app
            .create_today_journal_from_template()
            .expect("Journal creation should succeed");
        assert_eq!(created, today_path);
        assert!(today_path.exists());

        let content = std::fs::read_to_string(&today_path).expect("Should read created journal");
        assert!(
            !content.contains("{{UUID}}") && !content.contains("{{TODAY}}"),
            "Journal template placeholders should be resolved"
        );
    }

    #[test]
    fn test_today_entry_does_not_open_wizard_when_present() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let config = crate::config::Config {
            workspace: workspace_path.clone(),
            ..crate::config::Config::default()
        };
        let mut app = App::new(config);

        let today_path = app.config.workspace.today_journal_path();
        if let Some(parent) = today_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create journal directory");
        }
        std::fs::write(&today_path, "---\ntitle: today\n---\n").expect("Failed to create file");

        let today_idx = app
            .sidebar_items
            .iter()
            .position(|item| item.is_journal_item.as_deref() == Some("Today"))
            .expect("Today entry should exist");
        app.selected_entry_index = today_idx;
        app.open_tree_item();

        assert_eq!(app.current_view, ViewType::TreeView);
        assert!(app.template_field_state.is_none());
    }

    #[test]
    fn test_left_navigation_preserves_project_selection_after_project_milestone_roundtrip() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let program_dir = workspace_path.join("programs").join("TestProgram");
        std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
        std::fs::write(
            program_dir.join("TestProgram.md"),
            "---\ntitle: TestProgram\n---\n",
        )
        .expect("Failed to create program file");

        // Two projects to exercise up/down before navigating deeper.
        for project_name in ["AlphaProject", "BetaProject"] {
            let project_dir = program_dir.join("projects").join(project_name);
            std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
            std::fs::write(
                project_dir.join(format!("{project_name}.md")),
                format!("---\ntitle: {project_name}\n---\n"),
            )
            .expect("Failed to create project file");

            let milestone_dir = project_dir.join("milestones").join("M1");
            std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
            std::fs::write(milestone_dir.join("M1.md"), "---\ntitle: M1\n---\n")
                .expect("Failed to create milestone file");
        }

        let config = crate::config::Config {
            workspace: workspace_path,
            ..crate::config::Config::default()
        };
        let mut app = App::new(config);

        // Enter the top program.
        app.selected_entry_index = 1;
        app.open_tree_item();

        // Move selection in project list and enter BetaProject.
        if let Some(beta_idx) = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "BetaProject" && i.indent == 1)
        {
            app.selected_entry_index = beta_idx;
        } else {
            panic!("BetaProject should be selectable");
        }
        app.open_tree_item();

        // Single right from project expands and moves to milestone.
        assert_eq!(
            app.tree_model.selected_path().to_vec(),
            vec!["TestProgram", "BetaProject", "M1"]
        );

        // Collapse back one level.
        app.navigate_left();

        assert_eq!(
            app.tree_model.selected_path().to_vec(),
            vec!["TestProgram", "BetaProject"]
        );
        assert!(app.selected_entry_index < app.sidebar_items.len());
        let selected = &app.sidebar_items[app.selected_entry_index];
        assert_eq!(selected.name, "BetaProject");
        assert_eq!(selected.indent, 1);
        assert!(!selected.is_header);
    }

    #[test]
    fn test_tree_navigation_supports_program_with_direct_tasks() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let program_dir = workspace_path.join("programs").join("TestProgram");
        let tasks_dir = program_dir.join("tasks");
        std::fs::create_dir_all(&tasks_dir).expect("Failed to create directories");
        std::fs::write(
            workspace_path.join("programs").join("TestProgram.md"),
            "---\ntitle: TestProgram\n---\n",
        )
        .expect("Failed to create program file");
        std::fs::write(
            tasks_dir.join("DirectTask.md"),
            "---\ntitle: DirectTask\n---\n",
        )
        .expect("Failed to create direct task file");

        let config = crate::config::Config {
            workspace: workspace_path,
            ..crate::config::Config::default()
        };
        let mut app = App::new(config);

        app.selected_entry_index = 1;
        app.open_tree_item();
        assert_eq!(
            app.tree_model.selected_path().to_vec(),
            vec!["TestProgram", "DirectTask"]
        );

        app.navigate_left();
        assert_eq!(app.tree_model.selected_path().to_vec(), vec!["TestProgram"]);
        assert!(app.selected_entry_index < app.sidebar_items.len());
    }

    #[test]
    fn test_tree_navigation_prefers_expandable_variant_for_duplicate_names() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let program_dir = workspace_path.join("programs").join("TestProgram");
        let project_container = program_dir.join("projects");
        std::fs::create_dir_all(&project_container).expect("Failed to create directories");

        std::fs::write(
            workspace_path.join("programs").join("TestProgram.md"),
            "---\ntitle: TestProgram\n---\n",
        )
        .expect("Failed to create program file");

        // Duplicate project name in flat and nested forms.
        std::fs::write(program_dir.join("Common.md"), "---\ntitle: Common\n---\n")
            .expect("Failed to create flat duplicate project file");
        let common_nested_dir = project_container.join("Common");
        std::fs::create_dir_all(common_nested_dir.join("milestones").join("M1"))
            .expect("Failed to create nested duplicate hierarchy");
        std::fs::write(
            common_nested_dir.join("Common.md"),
            "---\ntitle: Common\n---\n",
        )
        .expect("Failed to create nested duplicate project file");
        std::fs::write(
            common_nested_dir
                .join("milestones")
                .join("M1")
                .join("M1.md"),
            "---\ntitle: M1\n---\n",
        )
        .expect("Failed to create milestone file");

        let config = crate::config::Config {
            workspace: workspace_path,
            ..crate::config::Config::default()
        };
        let mut app = App::new(config);

        app.selected_entry_index = 1;
        app.open_tree_item();
        let common_idx = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "Common" && i.indent == 1)
            .expect("Common should be selectable under program");
        app.selected_entry_index = common_idx;
        app.open_tree_item();

        assert_eq!(
            app.tree_model.selected_path().to_vec(),
            vec!["TestProgram", "Common", "M1"]
        );
        assert!(
            app.sidebar_items
                .iter()
                .any(|i| i.name == "M1" && i.indent == 2),
            "Expandable duplicate variant should be used, exposing milestones"
        );
    }

    #[test]
    fn test_tree_navigation_does_not_flatten_non_container_dirs() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let workspace_path = temp_dir.path().to_path_buf();

        let program_dir = workspace_path.join("programs").join("example program");
        let project_dir = program_dir.join("project 1");
        std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
        std::fs::write(
            workspace_path.join("programs").join("example program.md"),
            "---\ntitle: example program\n---\n",
        )
        .expect("Failed to create program file");
        std::fs::write(
            program_dir.join("project 1.md"),
            "---\ntitle: project 1\n---\n",
        )
        .expect("Failed to create project file");
        std::fs::write(
            project_dir.join("milestone 1.md"),
            "---\ntitle: milestone 1\n---\n",
        )
        .expect("Failed to create milestone file");
        std::fs::write(
            program_dir.join("project 2.md"),
            "---\ntitle: project 2\n---\n",
        )
        .expect("Failed to create second project file");

        let config = crate::config::Config {
            workspace: workspace_path,
            ..crate::config::Config::default()
        };
        let mut app = App::new(config);

        app.selected_entry_index = 1;
        app.open_tree_item();

        assert!(
            app.sidebar_items
                .iter()
                .any(|i| i.name == "project 1" && i.indent == 1),
            "project 1 should exist as a project under program"
        );
        assert!(
            app.sidebar_items
                .iter()
                .any(|i| i.name == "project 2" && i.indent == 1),
            "project 2 should exist as a project under program"
        );
        assert!(
            !app.sidebar_items
                .iter()
                .any(|i| i.name == "milestone 1" && i.indent == 1),
            "milestone 1 must not leak into program level"
        );

        let project_idx = app
            .sidebar_items
            .iter()
            .position(|i| i.name == "project 1" && i.indent == 1)
            .expect("project 1 should be selectable");
        app.selected_entry_index = project_idx;
        app.open_tree_item();

        assert!(
            app.sidebar_items
                .iter()
                .any(|i| i.name == "milestone 1" && i.indent == 2),
            "milestone 1 should appear only under project 1"
        );
    }
}
