pub mod cache;
pub mod command;
pub mod hierarchical_picker;
pub mod layout;
pub mod navigation;
pub mod planning_session;
pub mod planning_wizard;
pub mod review;
pub mod sidebar_tree;
pub mod task_wizard;
pub mod test_utils;
pub mod views;
pub mod wizard;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use std::io::{self, Write};

use crate::config::Config;
use crate::model::{PlanningSession, SelectedTask, SessionStatus};
use crate::storage::md::parse_element;
use crate::storage::planning::{
    archive_planning_session, create_planning_session, generate_session_uuid, list_active_sessions,
    load_planning_session, save_planning_session,
};
use crate::storage::{
    DirectoryEntry, JournalEntry, JournalStorage, WorkspaceStorage, validate_element_name,
};
use cache::{TaskMetadata, TreeData};
use chrono::Local;
use command::{CommandAction, CommandMatch, CommandPalette};
use hierarchical_picker::HierarchicalPickerState;
use navigation::{JournalTreeState, NavigationState, SidebarItem, SidebarNodeData, SidebarSection};
use planning_session::PlanningSessionState;
use planning_wizard::PlanningDateFocus;
use review::ReviewState;
use wizard::{FieldInfo, TemplateFieldState, WizardFocus, WizardState};

/// Application interaction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal navigation and interaction
    Normal,
    /// Command palette is open, typing filters commands
    CommandPalette,
    /// User is inputting data (e.g., creating element)
    #[allow(dead_code)]
    Input,
    /// User is selecting tasks for a planning session
    TaskSelection,
    /// User is reviewing tasks in a planning session
    ReviewSession,
    /// User is navigating hierarchical task picker
    HierarchicalSelection,
    /// User is previewing tasks before confirming plan
    PlanningPreview,
    /// User is editing task details before adding to plan
    TaskDetailWizard,
    /// User is inputting text for a task detail field
    InputTaskDetailField,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    TreeView,
    Journal,
    JournalArchiveList,
    #[allow(dead_code)]
    JournalToday,
    Backlog,
    WeeklyPlanning,
    ViewingContent,
    InputProgram,
    InputProject,
    InputMilestone,
    InputTask,
    InputTemplateField,
    InputPlanningSessionDates,
    PlanningTaskPicker,
    HierarchicalTaskPicker,
    PlanningPreview,
    TaskDetailWizard,
    InputTaskDetailField,
}

pub struct App {
    pub config: Config,
    pub current_view: ViewType,
    pub navigation_state: NavigationState,
    pub mode: Mode,
    pub command_palette: CommandPalette,
    pub should_exit: bool,
    pub journal_entries: Vec<JournalEntry>,
    pub needs_terminal_reinit: bool,
    pub tree_data: TreeData,
    pub input_buffer: String,
    pub selected_content: Option<DirectoryEntry>,
    pub current_content_text: Option<String>,
    pub wizard_state: WizardState,
    pub planning_session: PlanningSessionState,
    pub review_state: ReviewState,
    // Hierarchical task picker state
    pub hierarchical_picker: HierarchicalPickerState,
    // Extracted planning wizard state (replaces fields above)
    pub planning_wizard: Option<planning_wizard::PlanningWizardState>,
    // Task wizard state
    pub task_wizard: Option<task_wizard::TaskWizardState>,
    // Planning preview confirmed flag
    pub show_confirmation_message: bool,
    // Archive list tree structure (maps tree index -> journal entry index)
    pub archive_tree_mapping: Vec<Option<usize>>,
    // Journal tree state for sidebar expansion
    pub journal_tree_state: JournalTreeState,
}

impl App {
    const FOCUS_ASSIGNED_TO: usize = 100;
    const FOCUS_START_DATE: usize = 101;
    const FOCUS_DUE_DATE: usize = 102;

    pub fn new(config: Config) -> Self {
        let mut app = App {
            config,
            current_view: ViewType::TreeView,
            navigation_state: NavigationState::new(),
            mode: Mode::Normal,
            command_palette: CommandPalette::new(),
            should_exit: false,
            journal_entries: Vec::new(),
            needs_terminal_reinit: false,
            tree_data: TreeData::new(),
            input_buffer: String::new(),
            selected_content: None,
            current_content_text: None,
            wizard_state: WizardState::new(),
            planning_session: PlanningSessionState::new(),
            review_state: ReviewState::new(),
            hierarchical_picker: HierarchicalPickerState::new(),
            planning_wizard: None,
            task_wizard: None,
            show_confirmation_message: false,
            archive_tree_mapping: Vec::new(),
            journal_tree_state: JournalTreeState::new(),
        };

        app.load_tree_view_data();
        app.resume_planning_session();
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

    pub fn handle_key(&mut self, code: KeyCode) {
        // Handle TaskSelection mode specially
        if self.mode == Mode::TaskSelection {
            match code {
                KeyCode::Char(' ') => {
                    self.toggle_task_selection();
                }
                KeyCode::Enter => {
                    if self.planning_session.has_tasks() {
                        self.mode = Mode::Normal;
                        self.current_view = ViewType::WeeklyPlanning;
                    }
                }
                KeyCode::Esc => {
                    self.cancel_task_selection();
                }
                KeyCode::Right => {
                    self.navigate_right();
                }
                KeyCode::Left => {
                    self.navigate_left();
                }
                KeyCode::Up => {
                    self.navigate_up();
                }
                KeyCode::Down => {
                    self.navigate_down();
                }
                _ => {}
            }
            return;
        }

        // Handle HierarchicalSelection mode specially
        if self.mode == Mode::HierarchicalSelection {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.hierarchical_picker.navigate_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.hierarchical_picker.navigate_down();
                }
                KeyCode::Left => {
                    // Go back up hierarchy, but don't cancel if tasks are selected
                    if !self.hierarchical_picker.go_back() {
                        // At root level - only cancel if no tasks selected or in session
                        let has_picker_tasks = !self.hierarchical_picker.selected_tasks.is_empty();
                        let has_session_tasks = self.planning_session.has_tasks();
                        if !has_picker_tasks && !has_session_tasks {
                            self.cancel_planning_wizard();
                        }
                    } else {
                        self.load_hierarchical_picker_level(self.hierarchical_picker.level);
                    }
                }
                KeyCode::Right => {
                    // Drill down (same as Enter for navigation)
                    if self.hierarchical_picker.level != hierarchical_picker::PickerLevel::Tasks
                        && let Some((new_level, _name)) = self.hierarchical_picker.select_current()
                    {
                        self.load_hierarchical_picker_level(new_level);
                    }
                }
                KeyCode::Enter => {
                    let at_tasks_level =
                        self.hierarchical_picker.level == hierarchical_picker::PickerLevel::Tasks;

                    if at_tasks_level {
                        // Open task detail wizard for the selected task
                        self.open_task_detail_wizard();
                    } else if let Some((new_level, _name)) =
                        self.hierarchical_picker.select_current()
                    {
                        self.load_hierarchical_picker_level(new_level);
                    }
                }

                KeyCode::Char('f') => {
                    // 'f' for "Finish plan" - finalize if in wizard mode with tasks
                    let has_picker_selections = !self.hierarchical_picker.selected_tasks.is_empty();
                    let has_session_tasks = self.planning_session.has_tasks();
                    if (has_picker_selections || has_session_tasks)
                        && self.hierarchical_picker.is_wizard_mode
                    {
                        self.finalize_planning_session_from_picker();
                    }
                }
                KeyCode::Backspace => {
                    if !self.hierarchical_picker.go_back() {
                        // At root level - only cancel if no tasks selected or in session
                        let has_picker_tasks = !self.hierarchical_picker.selected_tasks.is_empty();
                        let has_session_tasks = self.planning_session.has_tasks();
                        if !has_picker_tasks && !has_session_tasks {
                            self.cancel_planning_wizard();
                        }
                    } else {
                        self.load_hierarchical_picker_level(self.hierarchical_picker.level);
                    }
                }
                KeyCode::Esc => {
                    if !self.hierarchical_picker.go_back() {
                        // At root level - only cancel if no tasks selected or in session
                        let has_picker_tasks = !self.hierarchical_picker.selected_tasks.is_empty();
                        let has_session_tasks = self.planning_session.has_tasks();
                        if !has_picker_tasks && !has_session_tasks {
                            self.cancel_planning_wizard();
                        }
                    } else {
                        self.load_hierarchical_picker_level(self.hierarchical_picker.level);
                    }
                }
                KeyCode::Char('q') => {
                    self.cancel_planning_wizard();
                }
                _ => {}
            }
            return;
        }

        // Handle ReviewSession mode specially
        if self.mode == Mode::ReviewSession {
            match code {
                KeyCode::Char('s') => {
                    self.cycle_task_status();
                }
                KeyCode::Char('r') => {
                    self.toggle_rollover();
                }
                KeyCode::Char('d') => {
                    self.mark_task_done();
                }
                KeyCode::Char('x') => {
                    self.remove_task_from_session();
                }
                KeyCode::Char('m') => {
                    // Add more tasks - go back to picker
                    self.add_more_tasks_to_session();
                }
                KeyCode::Char('c') => {
                    self.close_planning_session();
                }
                KeyCode::Char('a') => {
                    // Set assigned to - use input mode
                    self.mode = Mode::Input;
                    self.input_buffer = self
                        .planning_session
                        .tasks
                        .get(self.review_state.selection_index)
                        .and_then(|t| t.assigned_to.clone())
                        .unwrap_or_default();
                }
                KeyCode::Char('b') => {
                    // Set start date - use input mode
                    self.mode = Mode::Input;
                    self.input_buffer = self
                        .planning_session
                        .tasks
                        .get(self.review_state.selection_index)
                        .and_then(|t| t.start_date.clone())
                        .unwrap_or_default();
                }
                KeyCode::Char('e') => {
                    // Set due date - use input mode
                    self.mode = Mode::Input;
                    self.input_buffer = self
                        .planning_session
                        .tasks
                        .get(self.review_state.selection_index)
                        .and_then(|t| t.due_date.clone())
                        .unwrap_or_default();
                }
                KeyCode::Enter => {
                    // Confirm and finalize session
                    self.finalize_planning_session();
                }
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                    self.current_view = ViewType::WeeklyPlanning;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.navigate_review(-1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.navigate_review(1);
                }
                _ => {}
            }
            return;
        }

        // Handle PlanningPreview mode specially
        if self.mode == Mode::PlanningPreview {
            match code {
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.review_state.preview_focus > 0 {
                        self.review_state.preview_focus -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.review_state.preview_focus < 2 {
                        self.review_state.preview_focus += 1;
                    }
                }
                KeyCode::Enter => {
                    match self.review_state.preview_focus {
                        0 => self.add_more_tasks_to_session(),
                        1 => {
                            // Save the planning session and return to Normal mode
                            self.planning_session.active = true;
                            if self.planning_session.uuid.is_none() {
                                self.planning_session.uuid =
                                    Some(crate::storage::planning::generate_session_uuid());
                            }
                            self.save_current_planning_session();
                            self.show_confirmation_message = true;
                            self.return_from_view();
                        }
                        2 => self.cancel_planning_wizard(),
                        _ => {}
                    }
                }
                KeyCode::Esc => {
                    self.cancel_planning_wizard();
                }
                _ => {}
            }
            return;
        }

        // Handle TaskDetailWizard mode specially
        if self.mode == Mode::TaskDetailWizard {
            const TASK_WIZARD_FIELD_COUNT: usize = 7; // task name, status, assigned_to, start_date, due_date, priority, description
            if let Some(ref mut wizard) = self.task_wizard {
                match code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if wizard.field_index > 0 {
                            wizard.field_index -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if wizard.field_index < TASK_WIZARD_FIELD_COUNT + 1 {
                            // +1 for buttons row
                            wizard.field_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if wizard.field_index < TASK_WIZARD_FIELD_COUNT {
                            // On a field - cycle to next field (like template wizard)
                            self.cycle_task_wizard_field_or_next();
                        } else if wizard.field_index == TASK_WIZARD_FIELD_COUNT {
                            // ADD TO PLAN button
                            self.confirm_task_detail_wizard();
                        } else {
                            // CANCEL button
                            self.cancel_task_detail_wizard();
                        }
                    }
                    KeyCode::Esc => {
                        // Escape jumps to CANCEL button
                        if let Some(ref mut wizard) = self.task_wizard {
                            wizard.field_index = 8; // CancelButton index
                        }
                    }
                    KeyCode::Char(c) => {
                        // Inline editing - type directly into the focused field
                        self.handle_task_wizard_char(c);
                    }
                    KeyCode::Backspace => {
                        // Inline editing - backspace in the focused field
                        self.handle_task_wizard_backspace();
                    }
                    _ => {}
                }
            }
            return;
        }

        match code {
            KeyCode::Char('/') => {
                self.mode = Mode::CommandPalette;
                self.command_palette.input.clear();
                self.filter_commands();
            }
            KeyCode::Esc => {
                if matches!(self.mode, Mode::CommandPalette) {
                    self.mode = Mode::Normal;
                    self.command_palette.input.clear();
                    self.command_palette.selection_index = 0;
                } else if self.current_view == ViewType::InputTemplateField {
                    // Escape jumps to CANCEL button
                    if let Some(ref mut state) = self.wizard_state.template {
                        // Save current field value first
                        if let WizardFocus::Field(idx) = state.focus
                            && let Some(field) = state.fields.get_mut(idx)
                        {
                            field.value = self.input_buffer.clone();
                        }
                        state.focus = WizardFocus::CancelButton;
                        self.input_buffer.clear();
                    }
                } else if self.current_view == ViewType::InputPlanningSessionDates {
                    // ESC jumps to CancelButton instead of canceling
                    if let Some(ref mut wizard) = self.planning_wizard {
                        wizard.focus = PlanningDateFocus::CancelButton;
                    }
                } else if self.current_view == ViewType::PlanningTaskPicker {
                    // ESC cancels the planning wizard
                    self.cancel_planning_wizard();
                } else if self.current_view == ViewType::TaskDetailWizard {
                    // Cancel task detail wizard and return to picker
                    self.cancel_task_detail_wizard();
                } else if self.mode == Mode::Input
                    && matches!(
                        self.review_state.input_focus,
                        Some(Self::FOCUS_ASSIGNED_TO)
                            | Some(Self::FOCUS_START_DATE)
                            | Some(Self::FOCUS_DUE_DATE)
                    )
                {
                    // Cancel task metadata input and return to ReviewSession
                    self.input_buffer.clear();
                    self.mode = Mode::ReviewSession;
                } else if self.current_view == ViewType::TreeView {
                    // In TreeView, only navigate back if we're at root (empty path).
                    // When inside the tree (path not empty), do nothing because Left arrow
                    // should handle navigation up, and due to a crossterm bug, arrow keys
                    // can sometimes incorrectly trigger ESC first (the escape sequence parsing
                    // issue causes the ESC byte of the escape sequence to be interpreted as a
                    // separate keypress). By doing nothing, we prevent double-navigation.
                    if self
                        .navigation_state
                        .sidebar_tree
                        .selected_path()
                        .is_empty()
                    {
                        self.current_view = ViewType::Journal;
                    }
                } else {
                    self.return_from_view();
                }
            }
            KeyCode::Right => {
                if self.current_view == ViewType::InputPlanningSessionDates {
                    if let Some(ref mut wizard) = self.planning_wizard {
                        let focus_idx = wizard.focus.index();
                        if focus_idx == 1 {
                            // Duration field - cycle options
                            self.cycle_duration_right();
                        } else if focus_idx == 3 {
                            // ConfirmButton - cycle to CancelButton
                            wizard.focus = PlanningDateFocus::CancelButton;
                        } else if focus_idx == 4 {
                            // CancelButton - cycle back to StartDate
                            wizard.focus = PlanningDateFocus::StartDate;
                        } else {
                            self.navigate_right();
                        }
                    } else {
                        self.navigate_right();
                    }
                } else {
                    self.navigate_right();
                }
            }
            KeyCode::Left => {
                if self.current_view == ViewType::InputPlanningSessionDates {
                    if let Some(ref mut wizard) = self.planning_wizard {
                        let focus_idx = wizard.focus.index();
                        if focus_idx == 1 {
                            // Duration field - cycle options
                            self.cycle_duration_left();
                        } else if focus_idx == 4 {
                            // CancelButton - cycle to ConfirmButton
                            wizard.focus = PlanningDateFocus::ConfirmButton;
                        } else if focus_idx == 3 {
                            // ConfirmButton - cycle back to CancelButton
                            wizard.focus = PlanningDateFocus::CancelButton;
                        } else {
                            self.navigate_left();
                        }
                    } else {
                        self.navigate_left();
                    }
                } else {
                    self.navigate_left();
                }
            }
            KeyCode::Up => {
                if self.current_view == ViewType::InputTemplateField {
                    self.navigate_template_field_up();
                } else if self.current_view == ViewType::InputPlanningSessionDates {
                    self.navigate_planning_dates_up();
                } else if self.current_view == ViewType::PlanningTaskPicker {
                    self.navigate_planning_tasks_up();
                } else {
                    self.navigate_up();
                }
            }
            KeyCode::Down => {
                if self.current_view == ViewType::InputTemplateField {
                    self.navigate_template_field_down();
                } else if self.current_view == ViewType::InputPlanningSessionDates {
                    self.navigate_planning_dates_down();
                } else if self.current_view == ViewType::PlanningTaskPicker {
                    self.navigate_planning_tasks_down();
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
                } else if self.current_view == ViewType::InputPlanningSessionDates {
                    self.navigate_planning_dates_down();
                }
            }
            KeyCode::Char(' ') => {
                if self.current_view == ViewType::PlanningTaskPicker {
                    self.toggle_planning_task_selection();
                } else if matches!(
                    self.current_view,
                    ViewType::InputProgram
                        | ViewType::InputProject
                        | ViewType::InputMilestone
                        | ViewType::InputTask
                        | ViewType::InputTemplateField
                        | ViewType::InputPlanningSessionDates
                        | ViewType::InputTaskDetailField
                ) {
                    self.handle_input_char(' ');
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
        if let Some(ref mut state) = self.wizard_state.template {
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
        if let Some(ref mut state) = self.wizard_state.template {
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
            let idx = self.navigation_state.selected_entry_index;
            if idx < self.navigation_state.sidebar_items.len() {
                let item = &self.navigation_state.sidebar_items[idx];

                let is_journal = item.section == SidebarSection::Journal;
                let is_history = item.name == "History" && item.is_journal_item.is_some();
                let is_journal_item = item.is_journal_item.is_some();
                let journal_path = item.journal_path.clone();

                if is_journal {
                    // Handle journal history expansion
                    if is_history {
                        self.expand_history();
                        return;
                    }

                    // Handle journal tree navigation (month/year headers)
                    if let Some(ref jpath) = journal_path
                        && !is_journal_item
                    {
                        // This is a journal header (year/month) - expand it
                        self.expand_journal_item(jpath);
                        return;
                    }
                }

                tracing::debug!(
                    selected_index = self.navigation_state.selected_entry_index,
                    path = ?self.navigation_state.sidebar_tree.selected_path(),
                    "navigate right"
                );
                self.open_tree_item_with_leaf_open(false);
            }
        }
    }

    /// Expands the History item: loads journal entries if needed, expands the tree,
    /// reloads program data, rebuilds sidebar with programs + journal tree, and selects the first child.
    fn expand_history(&mut self) {
        if self.journal_entries.is_empty()
            && let Ok(entries) = self.config.workspace.list_journal_entries()
        {
            self.journal_entries = entries;
        }

        if !self.journal_entries.is_empty() {
            self.journal_tree_state
                .set_entries(self.journal_entries.clone());
        }

        self.navigation_state.expand_path(&[]);
        self.navigation_state.set_selected_path(vec![]);
        self.load_tree_view_data();
        self.select_first_journal_child(&[]);
    }

    fn navigate_left(&mut self) {
        if self.current_view != ViewType::TreeView {
            return;
        }

        let idx = self.navigation_state.selected_entry_index;
        if idx >= self.navigation_state.sidebar_items.len() {
            return;
        }

        let item = &self.navigation_state.sidebar_items[idx];

        let is_journal = item.section == SidebarSection::Journal;
        let journal_path = item.journal_path.clone();

        // Handle journal history navigation
        if is_journal {
            // Only collapse if this is a header (year/month) with visible children
            if item.is_journal_header {
                if let Some(ref jpath) = journal_path {
                    // Collapse current level
                    self.collapse_journal_item(jpath);

                    // Determine what to select after collapse
                    let target_path = if jpath.len() > 1 {
                        let parent_path: Vec<String> = jpath[..jpath.len() - 1].to_vec();
                        // Check if parent exists as sidebar item (in single-year case,
                        // year header doesn't exist, so we go to History instead)
                        if self
                            .navigation_state
                            .sidebar_items
                            .iter()
                            .any(|item| item.journal_path.as_ref() == Some(&parent_path))
                        {
                            Some(parent_path)
                        } else {
                            // Parent doesn't exist - collapse History level
                            self.collapse_journal_item(&[]);
                            None
                        }
                    } else {
                        None
                    };

                    // Rebuild sidebar
                    if let Some(ref path) = target_path {
                        self.navigation_state
                            .sidebar_tree
                            .set_selected_path(path.clone());
                    }
                    self.load_tree_view_data();

                    // Select the target after sidebar is rebuilt
                    if let Some(ref path) = target_path {
                        self.select_journal_item_by_path(path);
                    } else {
                        self.select_history();
                    }
                    return;
                }
                // At History level, don't collapse further
                return;
            }
            // Journal entry (leaf) - collapse to parent month/year header
            if let Some(ref jpath) = journal_path {
                // Collapse current entry in journal tree state
                self.collapse_journal_item(jpath);
                // Select parent month/year header
                if jpath.len() > 1 {
                    // Parent is month/year header (first len-1 elements)
                    let parent_path: Vec<String> = jpath[..jpath.len() - 1].to_vec();
                    // Collapse the parent path so children (including this entry) are hidden
                    self.collapse_journal_item(&parent_path);

                    // Rebuild sidebar first, then select
                    self.load_tree_view_data();
                    // Select the parent header (this overrides sync_selection result)
                    self.select_journal_item_by_path(&parent_path);
                } else {
                    // Edge case: single-level entry, collapse and go to History
                    self.load_tree_view_data();
                    self.select_history();
                }
                return;
            }
            // Fallback for journal entries without path (e.g., History, Today) - do nothing
            return;
        }

        // Program tree navigation
        let Some(selected_path) = item.tree_path.clone() else {
            return;
        };

        tracing::debug!(path = ?selected_path, "navigate left from selected path");
        if selected_path.is_empty() {
            return;
        }

        // For root-level items (programs), only collapse if they have children
        if selected_path.len() == 1 && !item.has_children {
            return;
        }

        self.collapse_path(&selected_path);
        let mut parent = selected_path;
        parent.pop();
        self.collapse_path(&parent);
        self.set_selected_tree_path(parent);
        self.load_tree_view_data();
    }

    fn add_journal_months_and_entries(&mut self, year: &str) {
        use crate::tui::cache::JournalNode;

        let months = self.journal_tree_state.months_for_year(year);

        for month in &months {
            let month_path = vec![year.to_string(), month.clone()];
            let node = JournalNode::Month {
                year: year.to_string(),
                month: month.clone(),
            };
            let mut item = SidebarItem::journal(node, month_path.clone());
            item.indent = 2;
            self.navigation_state.sidebar_items.push(item);

            if self.navigation_state.is_expanded(&month_path) {
                self.add_journal_entries_for_month(year, month);
            }
        }
    }

    fn add_journal_entries_for_month(&mut self, year: &str, month: &str) {
        self.add_journal_entries_with_indent(year, month, 3);
    }

    fn add_journal_entries_with_indent(&mut self, year: &str, month: &str, indent_level: usize) {
        use crate::tui::cache::JournalNode;

        let entries = self.journal_tree_state.entries_for_month(year, month);
        for entry in entries {
            let label = entry.filename.trim_end_matches(".md").to_string();
            let entry_path = if month.is_empty() {
                vec![year.to_string(), label.clone()]
            } else {
                vec![year.to_string(), month.to_string(), label.clone()]
            };
            let node = JournalNode::Entry {
                year: year.to_string(),
                month: month.to_string(),
                entry: entry.clone(),
            };
            let mut item = SidebarItem::journal(node, entry_path);
            item.indent = indent_level;
            self.navigation_state.sidebar_items.push(item);
        }
    }

    fn expand_journal_item(&mut self, path: &[String]) {
        self.navigation_state.expand_path(path);
        self.navigation_state.set_selected_path(path.to_vec());
        self.load_tree_view_data();
        self.select_first_journal_child(path);
    }

    fn collapse_journal_item(&mut self, path: &[String]) {
        self.navigation_state.collapse_path(path);
    }

    fn select_first_journal_child(&mut self, parent_path: &[String]) {
        let child_depth = parent_path.len() + 1;

        // First try to find children at the expected depth
        if let Some(idx) = self.navigation_state.sidebar_items.iter().position(|item| {
            item.journal_path
                .as_ref()
                .is_some_and(|p| p.len() == child_depth && p.starts_with(parent_path))
        }) {
            self.navigation_state.selected_entry_index = idx;
            return;
        }

        // For single-year case: when expanding root (empty parent_path),
        // years aren't created, so look for months at depth 2 instead
        if parent_path.is_empty() {
            let month_depth = 2;
            if let Some(idx) = self.navigation_state.sidebar_items.iter().position(|item| {
                item.journal_path
                    .as_ref()
                    .is_some_and(|p| p.len() == month_depth && p.starts_with(parent_path))
            }) {
                self.navigation_state.selected_entry_index = idx;
            }
        }
    }

    fn select_journal_item_by_path(&mut self, path: &[String]) {
        if let Some(idx) = self
            .navigation_state
            .sidebar_items
            .iter()
            .position(|item| item.journal_path.as_ref() == Some(&path.to_vec()))
        {
            self.navigation_state.selected_entry_index = idx;
        }
    }

    fn select_history(&mut self) {
        self.navigation_state.selected_entry_index = self
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "History" && i.is_journal_item.is_some())
            .unwrap_or(0);
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
                if !self
                    .navigation_state
                    .sidebar_tree
                    .selected_path()
                    .is_empty()
                {
                    let mut parent = self.navigation_state.sidebar_tree.selected_path_vec();
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
        let prev_section = self
            .navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)
            .map(|i| i.section.clone());

        self.navigation_state.navigate_up();
        self.sync_scope_from_sidebar_selection();

        // Cross-section navigation: collapse previous section
        if let Some(prev_section) = prev_section {
            let new_idx = self.navigation_state.selected_entry_index;
            if let Some(new_item) = self.navigation_state.sidebar_items.get(new_idx) {
                let new_section = new_item.section.clone();
                // If leaving Programs section, collapse all expanded programs
                if prev_section == SidebarSection::Programs && new_section != prev_section {
                    self.collapse_all_programs();
                    self.load_tree_view_data();
                    // After rebuild, select the LAST selectable item in the NEW section
                    // (for upward navigation, we want to land at the end of the section)
                    self.navigation_state.selected_entry_index = self
                        .navigation_state
                        .sidebar_items
                        .iter()
                        .rposition(|i| {
                            !i.is_header && !i.name.is_empty() && i.section == new_section
                        })
                        .unwrap_or(0);
                }
                // If leaving Journal section, clear journal expansion
                else if prev_section == SidebarSection::Journal && new_section != prev_section {
                    self.navigation_state.sidebar_tree.clear_expanded();
                    self.load_tree_view_data();
                    // After rebuild, select the LAST selectable item in the NEW section
                    // (for upward navigation, we want to land at the end of the section)
                    self.navigation_state.selected_entry_index = self
                        .navigation_state
                        .sidebar_items
                        .iter()
                        .rposition(|i| {
                            !i.is_header && !i.name.is_empty() && i.section == new_section
                        })
                        .unwrap_or(0);
                }
            }
        }
    }

    fn navigate_down(&mut self) {
        let prev_section = self
            .navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)
            .map(|i| i.section.clone());

        self.navigation_state.navigate_down();
        self.sync_scope_from_sidebar_selection();

        // Cross-section navigation: collapse previous section
        if let Some(prev_section) = prev_section {
            let new_idx = self.navigation_state.selected_entry_index;
            if let Some(new_item) = self.navigation_state.sidebar_items.get(new_idx) {
                let new_section = new_item.section.clone();
                // If leaving Programs section, collapse all expanded programs
                if prev_section == SidebarSection::Programs && new_section != prev_section {
                    self.collapse_all_programs();
                    self.load_tree_view_data();
                    // After rebuild, select the first selectable item in the NEW section
                    self.navigation_state.selected_entry_index = self
                        .navigation_state
                        .sidebar_items
                        .iter()
                        .position(|i| {
                            !i.is_header && !i.name.is_empty() && i.section == new_section
                        })
                        .unwrap_or(0);
                }
                // If leaving Journal section, clear journal expansion
                else if prev_section == SidebarSection::Journal && new_section != prev_section {
                    self.navigation_state.sidebar_tree.clear_expanded();
                    self.load_tree_view_data();
                    // After rebuild, select the first selectable item in the NEW section
                    self.navigation_state.selected_entry_index = self
                        .navigation_state
                        .sidebar_items
                        .iter()
                        .position(|i| {
                            !i.is_header && !i.name.is_empty() && i.section == new_section
                        })
                        .unwrap_or(0);
                }
            }
        }
    }

    fn collapse_all_programs(&mut self) {
        let paths_to_remove: Vec<Vec<String>> = self
            .navigation_state
            .sidebar_tree
            .expanded_paths()
            .iter()
            .cloned()
            .collect();
        for path in paths_to_remove {
            self.navigation_state.sidebar_tree.collapse_path(&path);
        }
    }

    pub fn sync_scope_from_sidebar_selection(&mut self) {
        let idx = self.navigation_state.selected_entry_index;
        if idx >= self.navigation_state.sidebar_items.len() {
            return;
        }
        let item = &self.navigation_state.sidebar_items[idx];

        // Don't sync scope for spacer items - they have no tree_path and shouldn't
        // trigger fallback behavior that would override navigation
        if item.name.is_empty() {
            return;
        }

        let Some(path) = item.tree_path.clone() else {
            return;
        };
        if path != self.navigation_state.sidebar_tree.selected_path() {
            self.set_selected_tree_path(path.clone());
        }
        // Also update current_* fields so wizard scope is accurate
        self.navigation_state.set_scope_from_path(&path);
    }

    fn open_tree_item(&mut self) {
        self.open_tree_item_with_leaf_open(true);
    }

    fn open_tree_item_with_leaf_open(&mut self, open_leaf_content: bool) {
        let idx = self.navigation_state.selected_entry_index;

        if idx >= self.navigation_state.sidebar_items.len() {
            return;
        }

        let item = &self.navigation_state.sidebar_items[idx];

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
                    self.expand_history();
                }
                _ => {}
            }
            return;
        }

        // Handle journal history entries (when journal_path is set but not a header)
        if let Some(ref jpath) = item.journal_path
            && !item.is_journal_header
        {
            // This is a journal entry - open it in viewer
            let label = navigation::journal_entry_label(jpath).unwrap_or(&item.name);
            if let Some(entry) = self
                .journal_entries
                .iter()
                .find(|e| *e.filename.trim_end_matches(".md") == *label)
                && let Ok(content) = self.config.workspace.read_journal_entry(&entry.path)
            {
                self.current_content_text = Some(content);
                self.selected_content = Some(DirectoryEntry {
                    name: entry.filename.clone(),
                    path: entry.path.clone(),
                    is_dir: false,
                });
                self.current_view = ViewType::ViewingContent;
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
                current_path = ?self.navigation_state.sidebar_tree.selected_path(),
                has_children,
                "open tree item"
            );
            if has_children || self.navigation_state.sidebar_tree.selected_path() != node_path {
                if has_children {
                    self.navigation_state.sidebar_tree.expand_path(&node_path);
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
        self.tree_data.programs = self.load_tree_level(&[]);
        self.navigation_state.update_scope_from_tree();

        self.tree_data.projects = self.load_tree_level_for_selected_depth(1);
        self.tree_data.milestones = self.load_tree_level_for_selected_depth(2);
        self.tree_data.tasks = self.load_tree_level_for_selected_depth(3);
        self.tree_data.subtasks = self.load_tree_level_for_selected_depth(4);

        tracing::debug!(
            path = ?self.navigation_state.sidebar_tree.selected_path(),
            programs = self.tree_data.programs.len(),
            projects = self.tree_data.projects.len(),
            milestones = self.tree_data.milestones.len(),
            tasks = self.tree_data.tasks.len(),
            subtasks = self.tree_data.subtasks.len(),
            "loaded tree view data"
        );
        self.build_sidebar_items();
        self.sync_selection_with_tree_path();
    }

    fn path_for_sidebar_item(&self, item: &SidebarItem) -> Vec<String> {
        let mut node_path = self.navigation_state.sidebar_tree.selected_path_vec();
        let truncate_to = item.indent.min(node_path.len());
        node_path.truncate(truncate_to);
        node_path.push(item.name.clone());
        node_path
    }

    fn set_selected_tree_path(&mut self, path: Vec<String>) {
        self.navigation_state
            .sidebar_tree
            .set_selected_path(path.clone());
        self.navigation_state.sidebar_tree.expand_ancestors(&path);
    }

    fn collapse_path(&mut self, path: &[String]) {
        self.navigation_state.sidebar_tree.collapse_path(path);
    }

    fn load_tree_level_for_selected_depth(&self, depth: usize) -> Vec<DirectoryEntry> {
        if self.navigation_state.sidebar_tree.selected_depth() < depth {
            return Vec::new();
        }
        self.load_tree_level(&self.navigation_state.sidebar_tree.selected_path()[..depth])
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
        self.navigation_state
            .sidebar_items
            .iter()
            .position(|item| !item.is_header && !item.name.is_empty())
            .unwrap_or(0)
    }

    fn sync_selection_with_tree_path(&mut self) {
        let mut candidate = self.navigation_state.sidebar_tree.selected_path_vec();
        while !candidate.is_empty() {
            if let Some(idx) = self
                .navigation_state
                .sidebar_items
                .iter()
                .position(|item| item.tree_path.as_ref() == Some(&candidate))
            {
                self.navigation_state.selected_entry_index = idx;
                if candidate != self.navigation_state.sidebar_tree.selected_path() {
                    self.set_selected_tree_path(candidate.clone());
                }
                tracing::debug!(
                    selected_index = idx,
                    selected_name = ?self.navigation_state.sidebar_tree.selected_path().last(),
                    "selection synced to tree path"
                );
                return;
            }
            candidate.pop();
        }

        // Fall back to first selectable
        self.navigation_state.selected_entry_index = self.first_selectable_sidebar_index();
        if !self
            .navigation_state
            .sidebar_tree
            .selected_path()
            .is_empty()
        {
            if let Some(path) = self
                .navigation_state
                .sidebar_items
                .get(self.navigation_state.selected_entry_index)
                .and_then(|item| item.tree_path.clone())
            {
                self.set_selected_tree_path(path);
            } else {
                self.navigation_state
                    .sidebar_tree
                    .set_selected_path(Vec::new());
            }
        }
    }

    fn select_first_child_for_path(&mut self, parent_path: &[String]) {
        if let Some(idx) = self.navigation_state.sidebar_items.iter().position(|item| {
            if item.is_header || item.name.is_empty() {
                return false;
            }
            let Some(path) = item.tree_path.as_ref() else {
                return false;
            };
            path.len() == parent_path.len() + 1 && path.starts_with(parent_path)
        }) {
            self.navigation_state.selected_entry_index = idx;
            if let Some(path) = self.navigation_state.sidebar_items[idx].tree_path.clone() {
                self.set_selected_tree_path(path);
                self.load_tree_view_data();
            }
        }
    }

    fn build_sidebar_items(&mut self) {
        self.navigation_state.sidebar_items.clear();
        self.navigation_state
            .sidebar_items
            .push(SidebarItem::new("Programs", SidebarSection::Programs).header());

        if self.tree_data.programs.is_empty() {
            self.navigation_state.sidebar_items.push(
                SidebarItem::new("+ Create Program...", SidebarSection::Programs)
                    .indent(1)
                    .create_action()
                    .node_data(SidebarNodeData::Action),
            );
        } else {
            self.push_tree_level_items(&[], 0);
        }

        self.navigation_state
            .sidebar_items
            .push(SidebarItem::new("", SidebarSection::Planning));
        self.navigation_state
            .sidebar_items
            .push(SidebarItem::new("Planning", SidebarSection::Planning).header());
        self.navigation_state.sidebar_items.push(
            SidebarItem::new("Current Plan", SidebarSection::Planning)
                .planning_item("WeeklyPlanning")
                .node_data(SidebarNodeData::Planning),
        );
        self.navigation_state.sidebar_items.push(
            SidebarItem::new("Backlog", SidebarSection::Planning)
                .planning_item("Backlog")
                .node_data(SidebarNodeData::Planning),
        );

        self.navigation_state
            .sidebar_items
            .push(SidebarItem::new("", SidebarSection::Journal));
        self.navigation_state
            .sidebar_items
            .push(SidebarItem::new("Journal", SidebarSection::Journal).header());
        self.navigation_state.sidebar_items.push(
            SidebarItem::new("Today", SidebarSection::Journal)
                .journal_item("Today")
                .node_data(SidebarNodeData::JournalAction),
        );
        self.navigation_state.sidebar_items.push(
            SidebarItem::new("History", SidebarSection::Journal)
                .journal_item("History")
                .node_data(SidebarNodeData::JournalAction),
        );

        // If journal history is expanded, add the tree structure
        if self.navigation_state.is_expanded(&[]) {
            self.add_journal_tree_items();
        }
    }

    fn add_journal_tree_items(&mut self) {
        use crate::tui::cache::JournalNode;

        let years = self.journal_tree_state.years();

        for year in &years {
            let year_path = vec![year.clone()];
            let node = JournalNode::Year { year: year.clone() };
            let mut item = SidebarItem::journal(node, year_path.clone());
            item.indent = 1;
            self.navigation_state.sidebar_items.push(item);

            if self.navigation_state.is_expanded(&year_path) {
                self.add_journal_months_and_entries(year);
            }
        }
    }

    fn push_tree_level_items(&mut self, parent_path: &[String], depth: usize) {
        let entries = if depth == 0 {
            self.tree_data.programs.clone()
        } else {
            self.load_tree_level(parent_path)
        };
        for entry in entries {
            let mut node_path = parent_path.to_vec();
            node_path.push(entry.name.clone());
            let has_children = !self.load_tree_level(&node_path).is_empty();
            self.navigation_state.sidebar_items.push(SidebarItem {
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
                journal_path: None,
                is_journal_header: false,
                node_data: SidebarNodeData::Program(entry),
            });

            if self.navigation_state.sidebar_tree.is_expanded(&node_path) {
                self.push_tree_level_items(&node_path, depth + 1);
            }
        }
    }

    fn handle_enter(&mut self) {
        // Handle input mode for review session task metadata
        if self.mode == Mode::Input {
            if let Some(focus) = self.review_state.input_focus {
                match focus {
                    Self::FOCUS_ASSIGNED_TO => {
                        // assigned_to
                        let name = self.input_buffer.clone();
                        self.set_task_assigned_to(name);
                        self.input_buffer.clear();
                        self.mode = Mode::ReviewSession;
                    }
                    Self::FOCUS_START_DATE => {
                        // start_date
                        let date = self.input_buffer.clone();
                        self.set_task_start_date(date);
                        self.input_buffer.clear();
                        self.mode = Mode::ReviewSession;
                    }
                    Self::FOCUS_DUE_DATE => {
                        // due_date
                        let date = self.input_buffer.clone();
                        self.set_task_due_date(date);
                        self.input_buffer.clear();
                        self.mode = Mode::ReviewSession;
                    }
                    _ => {}
                }
            }
            return;
        }

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
            ViewType::InputPlanningSessionDates => {
                self.confirm_planning_dates();
            }
            ViewType::PlanningTaskPicker => {
                self.confirm_planning_tasks();
            }
            ViewType::InputTaskDetailField => {
                self.confirm_task_detail_field_input();
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
                if let Some(ref mut state) = self.wizard_state.template
                    && let WizardFocus::Field(idx) = state.focus
                    && let Some(field) = state.fields.get_mut(idx)
                    && field.is_editable
                {
                    // Update both input_buffer and field.value for inline editing
                    self.input_buffer.push(c);
                    field.value.push(c);
                }
            }
            ViewType::InputTaskDetailField => {
                self.input_buffer.push(c);
            }
            ViewType::InputPlanningSessionDates => {
                // Use input_buffer for date fields (0=start_date, 2=end_date)
                if let Some(ref mut wizard) = self.planning_wizard {
                    let focus_idx = wizard.focus.index();
                    if focus_idx == 0 || focus_idx == 2 {
                        wizard.date_error = None;
                        self.input_buffer.push(c);
                        wizard.input_buffer.push(c); // Sync to wizard's field immediately
                    }
                }
            }
            ViewType::PlanningTaskPicker => {
                if let Some(ref mut wizard) = self.planning_wizard {
                    wizard.task_filter.push(c);
                    wizard.task_index = 0;
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
                if let Some(ref mut state) = self.wizard_state.template
                    && let WizardFocus::Field(idx) = state.focus
                    && let Some(field) = state.fields.get_mut(idx)
                    && field.is_editable
                {
                    // Update both input_buffer and field.value for inline editing
                    self.input_buffer.pop();
                    field.value.pop();
                }
            }
            ViewType::InputTaskDetailField => {
                self.input_buffer.pop();
            }
            ViewType::InputPlanningSessionDates => {
                // Use input_buffer for date fields (0=start_date, 2=end_date)
                if let Some(ref mut wizard) = self.planning_wizard {
                    let focus_idx = wizard.focus.index();
                    if focus_idx == 0 || focus_idx == 2 {
                        wizard.date_error = None;
                        self.input_buffer.pop();
                    }
                }
            }
            ViewType::PlanningTaskPicker => {
                if let Some(ref mut wizard) = self.planning_wizard {
                    wizard.task_filter.pop();
                }
            }
            _ => {}
        }
    }

    fn handle_command_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.command_palette.input.push(c);
                self.command_palette.selection_index = 0;
                self.filter_commands();
            }
            KeyCode::Backspace => {
                self.command_palette.input.pop();
                self.command_palette.selection_index = 0;
                self.filter_commands();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_palette.input.clear();
                self.command_palette.selection_index = 0;
            }
            KeyCode::Enter => {
                if let Some(cmd) = self
                    .command_palette
                    .matches
                    .get(self.command_palette.selection_index)
                    .cloned()
                {
                    self.execute_command(&cmd);
                }
                // Only reset mode if we're not entering a special mode that should persist
                if !matches!(
                    self.mode,
                    Mode::TaskSelection | Mode::ReviewSession | Mode::HierarchicalSelection
                ) {
                    self.mode = Mode::Normal;
                }
                self.command_palette.input.clear();
                self.command_palette.selection_index = 0;
            }
            KeyCode::Up => {
                if self.command_palette.selection_index > 0 {
                    self.command_palette.selection_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.command_palette.selection_index
                    < self.command_palette.matches.len().saturating_sub(1)
                {
                    self.command_palette.selection_index += 1;
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
            Some(CommandAction::ReviewSession) => {
                self.start_review_session();
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
                self.navigation_state.selected_entry_index = 0;
                self.current_view = ViewType::JournalArchiveList;
            }
            Err(e) => {
                eprintln!("Error loading archive list: {}", e);
            }
        }
    }

    fn open_archive_entry(&mut self, tree_index: usize) {
        if let Some(Some(entry_idx)) = self.archive_tree_mapping.get(tree_index)
            && let Some(entry) = self.journal_entries.get(*entry_idx)
        {
            let path = entry.path.clone();
            self.launch_editor(&path);
        }
    }

    fn open_selected_archive_entry(&mut self) {
        self.open_archive_entry(self.navigation_state.selected_entry_index);
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
        if !self
            .navigation_state
            .sidebar_tree
            .selected_path()
            .is_empty()
        {
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        } else {
            self.set_selected_tree_path(Vec::new());
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        }
    }

    fn show_milestones_list(&mut self) {
        if self.navigation_state.sidebar_tree.selected_depth() >= 2 {
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        } else {
            self.set_selected_tree_path(Vec::new());
            self.load_tree_view_data();
            self.current_view = ViewType::TreeView;
        }
    }

    fn show_tasks_list(&mut self) {
        if self.navigation_state.sidebar_tree.selected_depth() >= 3 {
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
        // If a session already exists, go directly to the task picker to edit it
        if self.planning_session.active {
            self.open_hierarchical_task_picker_for_existing_session();
            return;
        }
        self.open_planning_wizard();
    }

    fn open_hierarchical_task_picker_for_existing_session(&mut self) {
        // Open task picker for an existing session
        self.hierarchical_picker = hierarchical_picker::HierarchicalPickerState::new_wizard();
        self.load_hierarchical_picker_level(hierarchical_picker::PickerLevel::Programs);
        self.mode = Mode::HierarchicalSelection;
        self.current_view = ViewType::HierarchicalTaskPicker;
    }

    fn open_planning_wizard(&mut self) {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        // Initialize new state struct
        let mut wizard_state =
            planning_wizard::PlanningWizardState::new(&self.config.planning_duration);
        wizard_state.tasks = self.load_all_tasks();
        wizard_state.input_buffer = today.clone();
        self.planning_wizard = Some(wizard_state);

        // Initialize input_buffer with the start date so user can edit it
        self.input_buffer = today;
        self.current_view = ViewType::InputPlanningSessionDates;
    }

    fn load_all_tasks(&self) -> Vec<TaskMetadata> {
        let mut tasks = Vec::new();

        if let Ok(programs) = self.config.workspace.list_programs() {
            for program_entry in programs {
                let program = &program_entry.name;
                if let Ok(projects) = self.config.workspace.list_projects(program) {
                    for project_entry in projects {
                        let project = &project_entry.name;
                        if let Ok(milestones) =
                            self.config.workspace.list_milestones(program, project)
                        {
                            for milestone_entry in milestones {
                                let milestone = &milestone_entry.name;
                                if let Ok(task_entries) = self
                                    .config
                                    .workspace
                                    .list_tasks(program, project, milestone)
                                {
                                    for task_entry in task_entries {
                                        let task_path = task_entry.path.clone();
                                        if let Ok(content) = std::fs::read_to_string(&task_path)
                                            && let Some(parsed) =
                                                parse_element(&content).ok().flatten()
                                            && let crate::model::Element::Task(t) = parsed
                                        {
                                            tasks.push(TaskMetadata {
                                                uuid: t.uuid,
                                                path: task_path,
                                                program: program.clone(),
                                                project: project.clone(),
                                                milestone: milestone.clone(),
                                                task_name: t.title,
                                                status: t.status,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        tasks
    }

    pub fn get_filtered_tasks(&self) -> Vec<&TaskMetadata> {
        let Some(ref wizard) = self.planning_wizard else {
            return Vec::new();
        };

        let filter_lower = wizard.task_filter.to_lowercase();

        let mut tasks: Vec<_> = if filter_lower.is_empty() {
            wizard.tasks.iter().collect()
        } else {
            wizard
                .tasks
                .iter()
                .filter(|t| {
                    t.task_name.to_lowercase().contains(&filter_lower)
                        || t.program.to_lowercase().contains(&filter_lower)
                        || t.project.to_lowercase().contains(&filter_lower)
                        || t.milestone.to_lowercase().contains(&filter_lower)
                })
                .collect()
        };

        tasks.sort_by(|a, b| {
            a.program
                .cmp(&b.program)
                .then_with(|| a.project.cmp(&b.project))
                .then_with(|| a.milestone.cmp(&b.milestone))
                .then_with(|| a.task_name.cmp(&b.task_name))
        });

        tasks
    }

    fn finalize_planning_wizard_dates(&mut self) {
        // Use wizard state for dates
        let Some(ref mut wizard) = self.planning_wizard else {
            return;
        };

        // Validate start date using wizard state
        let start_date = match chrono::NaiveDate::parse_from_str(&wizard.start_date, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => {
                wizard.date_error = Some("Invalid date format. Use YYYY-MM-DD".to_string());
                return;
            }
        };

        // Calculate due date from wizard state
        let due_date = if !wizard.end_date.is_empty() {
            match chrono::NaiveDate::parse_from_str(&wizard.end_date, "%Y-%m-%d") {
                Ok(d) => d.format("%Y-%m-%d").to_string(),
                Err(_) => {
                    wizard.date_error = Some("Invalid end date format. Use YYYY-MM-DD".to_string());
                    return;
                }
            }
        } else {
            let days = match wizard.duration.as_str() {
                "biweekly" => 14,
                "6weekly" => 42,
                _ => 7,
            };
            start_date
                .checked_add_days(chrono::Days::new(days))
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| start_date.format("%Y-%m-%d").to_string())
        };

        // Clear any errors
        wizard.date_error = None;

        // Set session dates
        self.planning_session.start_date = Some(start_date.format("%Y-%m-%d").to_string());
        self.planning_session.due_date = Some(due_date);
        self.hierarchical_picker = hierarchical_picker::HierarchicalPickerState::new_wizard();
        self.load_hierarchical_picker_level(hierarchical_picker::PickerLevel::Programs);
        self.mode = Mode::HierarchicalSelection;
        self.current_view = ViewType::HierarchicalTaskPicker;
    }

    fn finalize_planning_session(&mut self) {
        let uuid = generate_session_uuid();
        self.planning_session.uuid = Some(uuid.clone());
        self.planning_session.active = true;

        // If tasks are already loaded (from picker flow), use them; otherwise load from UUIDs
        if !self.planning_session.has_tasks() {
            self.planning_session.rolled_over_tasks.clear();
            let all_tasks = self.load_all_tasks();
            // Get selected tasks from wizard state
            let selected_uuids: Vec<String> = self
                .planning_wizard
                .as_ref()
                .map(|w| w.selected_tasks.clone())
                .unwrap_or_default();

            for task_uuid in selected_uuids {
                if let Some(meta) = all_tasks.iter().find(|t| t.uuid == task_uuid) {
                    self.planning_session.tasks.push(SelectedTask {
                        uuid: meta.uuid.clone(),
                        path: meta.path.clone(),
                        program: meta.program.clone(),
                        project: meta.project.clone(),
                        milestone: meta.milestone.clone(),
                        task_name: meta.task_name.clone(),
                        status: meta.status.clone(),
                        assigned_to: None,
                        start_date: None,
                        due_date: None,
                        priority: None,
                        description: None,
                    });
                }
            }
        } else {
            // Tasks already loaded with metadata from picker - just clear wizard state
            if let Some(ref mut wizard) = self.planning_wizard {
                wizard.selected_tasks.clear();
            }
            self.planning_session.rolled_over_tasks.clear();
        }

        if let Err(e) = create_planning_session(
            &self.config.workspace,
            &uuid,
            &format!(
                "Plan for {}",
                self.planning_session.start_date.as_ref().unwrap()
            ),
            &chrono::Local::now().format("%Y-%m-%d").to_string(),
            self.planning_session.start_date.as_ref().unwrap(),
            self.planning_session.due_date.as_ref().unwrap(),
            self.planning_wizard
                .as_ref()
                .map(|w| w.duration.as_str())
                .unwrap_or("weekly"),
        ) {
            eprintln!("Failed to create planning session file: {e}");
        }

        // Return to TreeView (navigator) after finalizing
        self.current_view = ViewType::TreeView;
        self.mode = Mode::Normal;
    }

    fn finalize_planning_session_from_picker(&mut self) {
        use crate::model::Element;
        use crate::storage::md::parse_element;

        let existing_uuids: std::collections::HashSet<String> = self
            .planning_session
            .tasks
            .iter()
            .map(|t| t.uuid.clone())
            .collect();

        for path_str in self.hierarchical_picker.selected_tasks.iter() {
            let path = std::path::PathBuf::from(&path_str);
            if let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(parsed) = parse_element(&content)
                && let Some(Element::Task(t)) = parsed
            {
                let uuid = t.uuid.clone();
                if existing_uuids.contains(&uuid) {
                    continue; // Already in session, skip
                }

                self.planning_session.tasks.push(SelectedTask {
                    uuid: uuid.clone(),
                    path: path.clone(),
                    program: self
                        .hierarchical_picker
                        .selected_program
                        .clone()
                        .unwrap_or_default(),
                    project: self
                        .hierarchical_picker
                        .selected_project
                        .clone()
                        .unwrap_or_default(),
                    milestone: self
                        .hierarchical_picker
                        .selected_milestone
                        .clone()
                        .unwrap_or_default(),
                    task_name: t.title.clone(),
                    status: t.status.clone(),
                    assigned_to: t.assigned_to.clone(),
                    start_date: t.start_date.clone(),
                    due_date: t.due_date.clone(),
                    priority: t.priority.clone(),
                    description: None,
                });

                // Also add to wizard's selected tasks
                if let Some(ref mut wizard) = self.planning_wizard {
                    wizard.selected_tasks.push(uuid);
                }
            }
        }

        self.hierarchical_picker = hierarchical_picker::HierarchicalPickerState::new();

        // Generate UUID for the session if not already set (needed for preview)
        if self.planning_session.uuid.is_none() {
            self.planning_session.uuid = Some(generate_session_uuid());
        }

        // Transition to PlanningPreview to show tasks and allow user to confirm/add more/cancel
        // (not yet saved - will be saved on user confirmation)
        self.mode = Mode::PlanningPreview;
        self.current_view = ViewType::PlanningPreview;
        self.review_state.preview_focus = 1; // Default to CONFIRM button
    }

    fn close_planning_session(&mut self) {
        if !self.planning_session.active {
            println!("No active planning session to close.");
            return;
        }

        let rolled_tasks = std::mem::take(&mut self.planning_session.rolled_over_tasks);

        if let Some(start_date) = &self.planning_session.start_date
            && let Err(e) = archive_planning_session(&self.config.workspace, start_date)
        {
            eprintln!("Failed to archive planning session: {e}");
        }

        self.planning_session.active = false;
        self.planning_session.uuid = None;
        self.planning_session.start_date = None;
        self.planning_session.due_date = None;
        self.planning_session.tasks.clear();

        if !rolled_tasks.is_empty() {
            self.start_planning_session_with_rolled_tasks(rolled_tasks);
        } else {
            self.current_view = ViewType::TreeView;
        }
    }

    fn start_planning_session_with_rolled_tasks(&mut self, task_uuids: Vec<String>) {
        // Resolve task UUIDs to SelectedTask by loading tasks on-demand
        let all_tasks = self.load_all_tasks();
        let tasks: Vec<_> = task_uuids
            .iter()
            .filter_map(|uuid| {
                all_tasks
                    .iter()
                    .find(|t| &t.uuid == uuid)
                    .cloned()
                    .map(SelectedTask::from)
            })
            .collect();

        // Start a fresh session and add the rolled tasks
        self.start_planning_session();
        self.planning_session.tasks = tasks;
        self.planning_session.rolled_over_tasks = task_uuids;
        self.save_current_planning_session();
        self.mode = Mode::Normal;
        self.current_view = ViewType::WeeklyPlanning;
    }

    fn save_current_planning_session(&mut self) {
        let Some(uuid) = &self.planning_session.uuid else {
            return;
        };
        let Some(start_date) = &self.planning_session.start_date else {
            return;
        };
        let Some(due_date) = &self.planning_session.due_date else {
            return;
        };

        let session = PlanningSession {
            element_type: "planning".to_string(),
            uuid: uuid.clone(),
            title: format!("Plan for {}", start_date),
            creation_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            created_by: None,
            start_date: start_date.clone(),
            end_date: due_date.clone(),
            duration: self.config.planning_duration.clone(),
            status: SessionStatus::Active,
            tasks: self
                .planning_session
                .tasks
                .iter()
                .map(|t| t.uuid.clone())
                .collect(),
        };

        if let Err(e) = save_planning_session(&self.config.workspace, &session) {
            eprintln!("Failed to save planning session: {e}");
        }
    }

    fn toggle_task_selection(&mut self) {
        if self.mode != Mode::TaskSelection {
            return;
        }

        let Some(item) = self
            .navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)
        else {
            return;
        };
        if item.is_header || item.indent < 3 {
            return;
        }
        let Some(path) = &item.path else { return };

        let all_tasks = self.load_all_tasks();
        let selected_task = all_tasks
            .iter()
            .find(|t| &t.path == path)
            .cloned()
            .map(SelectedTask::from)
            .or_else(|| self.read_task_from_file(path, item));

        let Some(task) = selected_task else { return };

        if let Some(pos) = self
            .planning_session
            .tasks
            .iter()
            .position(|t| t.uuid == task.uuid)
        {
            self.planning_session.tasks.remove(pos);
        } else {
            self.planning_session.tasks.push(task);
        }

        // Save session to file
        self.save_current_planning_session();
    }

    fn read_task_from_file(
        &self,
        path: &std::path::Path,
        item: &SidebarItem,
    ) -> Option<SelectedTask> {
        let content = self.config.workspace.read_md_file(path).ok()?;
        let parsed = parse_element(&content).ok().flatten()?;
        let crate::model::Element::Task(task) = parsed else {
            return None;
        };

        let tree_path = item.tree_path.clone().unwrap_or_default();
        let program = tree_path.first()?.clone();
        let project = tree_path.get(1)?.clone();
        let milestone = tree_path.get(2)?.clone();

        Some(SelectedTask {
            uuid: task.uuid,
            path: path.to_path_buf(),
            program,
            project,
            milestone,
            task_name: task.title,
            status: task.status,
            assigned_to: task.assigned_to.clone(),
            start_date: task.start_date.clone(),
            due_date: task.due_date.clone(),
            priority: task.priority.clone(),
            description: Some(task.description.clone()),
        })
    }

    fn cancel_task_selection(&mut self) {
        // Delete the planning session file if it exists
        if let Some(start_date) = &self.planning_session.start_date {
            let path = self
                .config
                .workspace
                .join("planning")
                .join("current")
                .join(format!("{}-planning.md", start_date));
            if path.exists()
                && let Err(e) = std::fs::remove_file(&path)
            {
                eprintln!("Failed to delete planning session file: {e}");
            }
        }

        self.planning_session.active = false;
        self.planning_session.uuid = None;
        self.planning_session.tasks.clear();
        self.planning_session.start_date = None;
        self.planning_session.due_date = None;
        self.mode = Mode::Normal;
    }

    fn load_hierarchical_picker_level(&mut self, level: hierarchical_picker::PickerLevel) {
        use hierarchical_picker::PickerLevel;

        let entries = match level {
            PickerLevel::Programs => self.config.workspace.list_programs().unwrap_or_default(),
            PickerLevel::Projects => {
                let Some(program) = &self.hierarchical_picker.selected_program else {
                    return;
                };
                self.config
                    .workspace
                    .list_projects(program)
                    .unwrap_or_default()
            }
            PickerLevel::Milestones => {
                let Some(program) = &self.hierarchical_picker.selected_program else {
                    return;
                };
                let Some(project) = &self.hierarchical_picker.selected_project else {
                    return;
                };
                self.config
                    .workspace
                    .list_milestones(program, project)
                    .unwrap_or_default()
            }
            PickerLevel::Tasks => {
                let Some(program) = &self.hierarchical_picker.selected_program else {
                    return;
                };
                let Some(project) = &self.hierarchical_picker.selected_project else {
                    return;
                };
                let Some(milestone) = &self.hierarchical_picker.selected_milestone else {
                    return;
                };
                self.config
                    .workspace
                    .list_tasks(program, project, milestone)
                    .unwrap_or_default()
            }
        };
        self.hierarchical_picker.set_items(entries);
    }

    fn start_review_session(&mut self) {
        if !self.planning_session.active || !self.planning_session.has_tasks() {
            return;
        }
        self.review_state.reset();
        self.planning_session.rolled_over_tasks.clear();
        self.mode = Mode::ReviewSession;
    }

    fn navigate_review(&mut self, direction: isize) {
        if !self.planning_session.has_tasks() {
            return;
        }
        let new_idx = if direction < 0 {
            self.review_state.selection_index.saturating_sub(1)
        } else {
            (self.review_state.selection_index + 1).min(self.planning_session.tasks.len() - 1)
        };
        self.review_state.selection_index = new_idx;
    }

    fn cycle_task_status(&mut self) {
        let Some(task) = self
            .planning_session
            .tasks
            .get(self.review_state.selection_index)
        else {
            return;
        };
        let workflow = &self.config.workflow;
        if workflow.is_empty() {
            return;
        }
        let current_idx = workflow.iter().position(|s| s == &task.status).unwrap_or(0);
        let next_idx = (current_idx + 1) % workflow.len();
        let new_status = workflow[next_idx].clone();
        self.update_review_task_status(&new_status);
    }

    fn set_task_start_date(&mut self, date: String) {
        let Some(task) = self
            .planning_session
            .tasks
            .get_mut(self.review_state.selection_index)
        else {
            return;
        };
        task.start_date = Some(date.clone());
        if let Err(e) = crate::storage::md::update_task_fields(
            &task.path,
            [("start_date", Some(date))].into_iter().collect(),
        ) {
            eprintln!("Failed to update task start date: {e}");
        }
    }

    fn set_task_due_date(&mut self, date: String) {
        let Some(task) = self
            .planning_session
            .tasks
            .get_mut(self.review_state.selection_index)
        else {
            return;
        };
        task.due_date = Some(date.clone());
        if let Err(e) = crate::storage::md::update_task_fields(
            &task.path,
            [("due_date", Some(date))].into_iter().collect(),
        ) {
            eprintln!("Failed to update task due date: {e}");
        }
    }

    fn set_task_assigned_to(&mut self, name: String) {
        let Some(task) = self
            .planning_session
            .tasks
            .get_mut(self.review_state.selection_index)
        else {
            return;
        };
        task.assigned_to = Some(name.clone());
        if let Err(e) = crate::storage::md::update_task_fields(
            &task.path,
            [("assigned_to", Some(name))].into_iter().collect(),
        ) {
            eprintln!("Failed to update task assigned_to: {e}");
        }
    }

    fn mark_task_done(&mut self) {
        self.update_review_task_status("done");
    }

    fn update_review_task_status(&mut self, new_status: &str) {
        let Some(task) = self
            .planning_session
            .tasks
            .get_mut(self.review_state.selection_index)
        else {
            return;
        };
        let old_status = task.status.clone();
        if old_status == new_status {
            return;
        }

        // Update task file
        if let Err(e) = crate::storage::md::update_task_status(&task.path, new_status) {
            eprintln!("Failed to update task status: {e}");
            return;
        }

        // Update in-memory state
        task.status = new_status.to_string();

        // Save session file
        self.save_current_planning_session();
    }

    fn toggle_rollover(&mut self) {
        let Some(task) = self
            .planning_session
            .tasks
            .get(self.review_state.selection_index)
        else {
            return;
        };
        let uuid = &task.uuid;

        if let Some(pos) = self
            .planning_session
            .rolled_over_tasks
            .iter()
            .position(|u| u == uuid)
        {
            self.planning_session.rolled_over_tasks.remove(pos);
        } else {
            self.planning_session.rolled_over_tasks.push(uuid.clone());
        }
    }

    fn remove_task_from_session(&mut self) {
        if !self.planning_session.has_tasks() {
            return;
        }
        let Some(task) = self
            .planning_session
            .tasks
            .get(self.review_state.selection_index)
        else {
            return;
        };

        // Remove from rolled_over if present
        if let Some(pos) = self
            .planning_session
            .rolled_over_tasks
            .iter()
            .position(|u| u == &task.uuid)
        {
            self.planning_session.rolled_over_tasks.remove(pos);
        }

        // Remove from planning session tasks
        self.planning_session
            .tasks
            .remove(self.review_state.selection_index);

        // Adjust selection index
        if self.review_state.selection_index >= self.planning_session.tasks.len()
            && self.review_state.selection_index > 0
        {
            self.review_state.selection_index -= 1;
        }

        // Save session file
        self.save_current_planning_session();
    }

    fn add_more_tasks_to_session(&mut self) {
        // Save current session state before adding more tasks
        self.save_current_planning_session();

        // Go back to hierarchical picker to add more tasks
        self.hierarchical_picker = hierarchical_picker::HierarchicalPickerState::new_wizard();
        // Sync selected_tasks from planning_session so previously added tasks show as checked
        for task in &self.planning_session.tasks {
            self.hierarchical_picker
                .selected_tasks
                .insert(task.path.to_string_lossy().to_string());
        }
        self.load_hierarchical_picker_level(hierarchical_picker::PickerLevel::Programs);
        self.mode = Mode::HierarchicalSelection;
        self.current_view = ViewType::HierarchicalTaskPicker;
    }

    pub fn resume_planning_session(&mut self) {
        let Ok(sessions) = list_active_sessions(&self.config.workspace) else {
            return;
        };
        let Some(session_path) = sessions.first() else {
            return;
        };

        let Ok(session) = load_planning_session(session_path) else {
            eprintln!(
                "Failed to load planning session from {}",
                session_path.display()
            );
            return;
        };

        // Restore session state
        self.planning_session.active = true;
        self.planning_session.uuid = Some(session.uuid);
        self.planning_session.start_date = Some(session.start_date);
        self.planning_session.due_date = Some(session.end_date);

        // Rebuild task list from UUIDs by loading tasks on-demand
        let all_tasks = self.load_all_tasks();
        for uuid in &session.tasks {
            if let Some(meta) = all_tasks.iter().find(|t| &t.uuid == uuid).cloned() {
                self.planning_session.tasks.push(SelectedTask::from(meta));
            }
        }
    }

    fn promote_selection_to_path_depth(&mut self, target_depth: usize) {
        if self.navigation_state.sidebar_tree.selected_depth() >= target_depth {
            return;
        }

        let mut selected_path = match self
            .navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)
        {
            Some(item) if !item.is_header && !item.name.is_empty() => {
                self.path_for_sidebar_item(item)
            }
            _ => return,
        };

        if selected_path.len() < target_depth {
            let parent = self.navigation_state.sidebar_tree.selected_path_vec();
            self.select_first_child_for_path(&parent);
            selected_path = match self
                .navigation_state
                .sidebar_items
                .get(self.navigation_state.selected_entry_index)
            {
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

        // Build scope string showing where the new element will be created
        let scope_value = match template_name {
            "program" => "Programs (root level)".to_string(),
            "project" => self
                .navigation_state
                .current_program
                .clone()
                .unwrap_or_default(),
            "milestone" | "task" => {
                let mut parts = Vec::new();
                if let Some(ref p) = self.navigation_state.current_program {
                    parts.push(p.clone());
                }
                if let Some(ref p) = self.navigation_state.current_project {
                    parts.push(p.clone());
                }
                if template_name == "task"
                    && let Some(ref p) = self.navigation_state.current_milestone
                {
                    parts.push(p.clone());
                }
                parts.join(" > ")
            }
            "journal" => "Journal".to_string(),
            _ => "".to_string(),
        };

        let mut fields: Vec<FieldInfo> = all_fields
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
                    is_focused: false,
                    is_editable: !is_keyword,
                    display_order: i + 1,
                }
            })
            .collect();

        // Add scope field at the beginning (non-editable, shows creation path)
        let scope_field = FieldInfo {
            label: "Creating in:".to_string(),
            placeholder: "SCOPE".to_string(),
            value: scope_value,
            is_focused: false,
            is_editable: false,
            display_order: 0,
        };
        fields.insert(0, scope_field);

        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        self.wizard_state.template = Some(TemplateFieldState {
            template_name: template_name.to_string(),
            fields,
            focus: initial_focus,
            values,
            strip_labels,
        });

        if let WizardFocus::Field(idx) = initial_focus {
            if let Some(field) = self
                .wizard_state
                .template
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
                self.navigation_state.current_program.as_ref().map(|prog| {
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
                .navigation_state
                .current_program
                .as_ref()
                .zip(self.navigation_state.current_project.as_ref())
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
                .navigation_state
                .current_program
                .as_ref()
                .zip(self.navigation_state.current_project.as_ref())
                .zip(self.navigation_state.current_milestone.as_ref())
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
        if let Some(ref mut state) = self.wizard_state.template {
            match state.focus {
                WizardFocus::CancelButton => {
                    // Cancel - return to tree view without creating
                    self.wizard_state.template = None;
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
                    self.wizard_state.template = None;

                    // Refresh the tree view to show the newly created element at current level
                    self.load_tree_view_data();

                    // Find and select the newly created element in the sidebar
                    // Stay at parent level (don't auto-navigate into new element)
                    if let Some(ref element_name) = new_element_name {
                        // Find the newly created element in sidebar_items
                        if let Some(pos) = self
                            .navigation_state
                            .sidebar_items
                            .iter()
                            .position(|item| !item.is_header && item.name == *element_name)
                        {
                            self.navigation_state.selected_entry_index = pos;
                            // Also update sidebar_tree.selected_path to sync the breadcrumb
                            if let Some(ref tree_path) =
                                self.navigation_state.sidebar_items[pos].tree_path
                            {
                                self.navigation_state
                                    .sidebar_tree
                                    .set_selected_path(tree_path.clone());
                            }
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
        self.command_palette.filter_with_context(
            self.navigation_state.current_program.as_deref(),
            self.navigation_state.current_project.as_deref(),
            self.navigation_state.current_milestone.as_deref(),
            !self.tree_data.programs.is_empty(),
        );
    }

    fn draw(&self, f: &mut Frame) {
        layout::render(f, self);
    }

    fn navigate_planning_dates_up(&mut self) {
        // Use wizard state
        if let Some(ref mut wizard) = self.planning_wizard {
            // Save current input_buffer value to the appropriate field before moving
            wizard.sync_input_to_field();
            // Navigate using the wizard's focus enum
            wizard.focus = wizard.focus.prev();
            // Load the newly focused field's value into input_buffer
            wizard.sync_field_to_input();
            self.input_buffer = wizard.input_buffer.clone();
        }
    }

    fn navigate_planning_dates_down(&mut self) {
        // Use wizard state
        if let Some(ref mut wizard) = self.planning_wizard {
            // Save current input_buffer value to the appropriate field before moving
            wizard.sync_input_to_field();
            // Navigate using the wizard's focus enum
            wizard.focus = wizard.focus.next();
            // Load the newly focused field's value into input_buffer
            wizard.sync_field_to_input();
            self.input_buffer = wizard.input_buffer.clone();
        }
    }

    fn cycle_duration_left(&mut self) {
        // Use wizard state
        if let Some(ref mut wizard) = self.planning_wizard {
            wizard.cycle_duration(-1);
        }
    }

    fn cycle_duration_right(&mut self) {
        // Use wizard state
        if let Some(ref mut wizard) = self.planning_wizard {
            wizard.cycle_duration(1);
        }
    }

    fn navigate_planning_tasks_up(&mut self) {
        let task_index = self
            .planning_wizard
            .as_ref()
            .map(|w| w.task_index)
            .unwrap_or(0);
        let filtered = self.get_filtered_tasks();
        if !filtered.is_empty()
            && task_index > 0
            && let Some(ref mut wizard) = self.planning_wizard
        {
            wizard.task_index -= 1;
        }
    }

    fn navigate_planning_tasks_down(&mut self) {
        let task_index = self
            .planning_wizard
            .as_ref()
            .map(|w| w.task_index)
            .unwrap_or(0);
        let filtered = self.get_filtered_tasks();
        let max_idx = filtered.len().saturating_sub(1);
        if task_index < max_idx
            && let Some(ref mut wizard) = self.planning_wizard
        {
            wizard.task_index += 1;
        }
    }

    fn toggle_planning_task_selection(&mut self) {
        let task_index = self
            .planning_wizard
            .as_ref()
            .map(|w| w.task_index)
            .unwrap_or(0);
        let filtered_tasks = self.get_filtered_tasks();
        let Some(task) = filtered_tasks.get(task_index) else {
            return;
        };
        let uuid = task.uuid.clone();
        if let Some(ref mut wizard) = self.planning_wizard {
            if let Some(pos) = wizard.selected_tasks.iter().position(|u| *u == uuid) {
                wizard.selected_tasks.remove(pos);
            } else {
                wizard.selected_tasks.push(uuid);
            }
        }
    }

    fn confirm_planning_dates(&mut self) {
        if let Some(ref mut wizard) = self.planning_wizard {
            let focus_idx = wizard.focus.index();
            match focus_idx {
                0..=2 => {
                    self.navigate_planning_dates_down();
                }
                3 => {
                    // Confirm button - validate dates
                    if chrono::NaiveDate::parse_from_str(&wizard.start_date, "%Y-%m-%d").is_err() {
                        wizard.date_error =
                            Some("Invalid start date format. Use YYYY-MM-DD".to_string());
                        return;
                    }
                    if !wizard.end_date.is_empty() {
                        if chrono::NaiveDate::parse_from_str(&wizard.end_date, "%Y-%m-%d").is_err()
                        {
                            wizard.date_error =
                                Some("Invalid end date format. Use YYYY-MM-DD".to_string());
                            return;
                        }
                        let start =
                            chrono::NaiveDate::parse_from_str(&wizard.start_date, "%Y-%m-%d").ok();
                        let end =
                            chrono::NaiveDate::parse_from_str(&wizard.end_date, "%Y-%m-%d").ok();
                        if let (Some(s), Some(e)) = (start, end)
                            && e < s
                        {
                            wizard.date_error =
                                Some("End date must be on or after start date".to_string());
                            return;
                        }
                    }
                    wizard.date_error = None;
                    self.finalize_planning_wizard_dates();
                }
                4 => {
                    self.cancel_planning_wizard();
                }
                _ => {}
            }
        }
    }

    fn confirm_planning_tasks(&mut self) {
        if self
            .planning_wizard
            .as_ref()
            .map(|w| !w.selected_tasks.is_empty())
            .unwrap_or(false)
        {
            self.finalize_planning_session();
        }
    }

    fn open_task_detail_wizard(&mut self) {
        use crate::model::Element;
        use crate::storage::md::parse_element;

        let Some(item) = self
            .hierarchical_picker
            .items
            .get(self.hierarchical_picker.cursor_index)
            .cloned()
        else {
            tracing::debug!("open_task_detail_wizard: no item at cursor index");
            return;
        };

        // Load task from file
        let content = match std::fs::read_to_string(&item.path) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(path = ?item.path, error = %e, "open_task_detail_wizard: failed to read task file");
                return;
            }
        };

        let parsed = match parse_element(&content) {
            Ok(Some(p)) => p,
            Ok(None) => {
                tracing::debug!(path = ?item.path, "open_task_detail_wizard: no YAML frontmatter found");
                return;
            }
            Err(e) => {
                tracing::debug!(path = ?item.path, error = %e, "open_task_detail_wizard: failed to parse task file");
                return;
            }
        };

        let task = match parsed {
            Element::Task(t) => SelectedTask {
                uuid: t.uuid.clone(),
                path: item.path.clone(),
                program: self
                    .hierarchical_picker
                    .selected_program
                    .clone()
                    .unwrap_or_default(),
                project: self
                    .hierarchical_picker
                    .selected_project
                    .clone()
                    .unwrap_or_default(),
                milestone: self
                    .hierarchical_picker
                    .selected_milestone
                    .clone()
                    .unwrap_or_default(),
                task_name: t.title.clone(),
                status: t.status.clone(),
                // Dynamically load existing values from task, or None if not present
                assigned_to: t.assigned_to.clone(),
                start_date: t.start_date.clone(),
                due_date: t.due_date.clone(),
                priority: t.priority.clone(),
                description: Some(t.description.clone()),
            },
            _ => {
                tracing::debug!(path = ?item.path, "open_task_detail_wizard: parsed element is not a Task");
                return;
            }
        };

        self.task_wizard = Some(task_wizard::TaskWizardState::with_task(task));
        self.mode = Mode::TaskDetailWizard;
        self.current_view = ViewType::TaskDetailWizard;
    }

    fn cycle_task_wizard_field_or_next(&mut self) {
        if let Some(ref mut wizard) = self.task_wizard {
            task_wizard::cycle_task_wizard_field_or_next(wizard, &self.config.workflow);
        }
    }

    fn handle_task_wizard_char(&mut self, c: char) {
        if let Some(ref mut wizard) = self.task_wizard {
            task_wizard::handle_task_wizard_char(wizard, c);
        }
    }

    fn handle_task_wizard_backspace(&mut self) {
        if let Some(ref mut wizard) = self.task_wizard {
            task_wizard::handle_task_wizard_backspace(wizard);
        }
    }

    fn confirm_task_detail_wizard(&mut self) {
        if let Some(ref mut wizard) = self.task_wizard
            && let Some(task) = task_wizard::confirm_task_detail_wizard(wizard)
        {
            // Update task file on disk with any changes
            let task_path = task.path.clone();
            let updates = [
                ("status", Some(task.status.clone())),
                ("assigned_to", task.assigned_to.clone()),
                ("start_date", task.start_date.clone()),
                ("due_date", task.due_date.clone()),
                ("priority", task.priority.clone()),
            ];
            let updates_map: std::collections::HashMap<&str, Option<String>> =
                updates.iter().map(|(k, v)| (*k, v.clone())).collect();
            if let Err(e) = crate::storage::md::update_task_fields(&task_path, updates_map) {
                tracing::warn!("Failed to update task file: {}", e);
            }

            // Add task to planning session
            self.planning_session.tasks.push(task);

            // Mark task as selected in picker so checkbox shows [x]
            let path_str = task_path.to_string_lossy().to_string();
            self.hierarchical_picker.selected_tasks.insert(path_str);

            // Save session if UUID exists (finalized state)
            if self.planning_session.uuid.is_some() {
                self.save_current_planning_session();
            }
        }

        // Return to picker
        self.mode = Mode::HierarchicalSelection;
        self.current_view = ViewType::HierarchicalTaskPicker;
    }

    fn cancel_task_detail_wizard(&mut self) {
        // Clear the wizard state
        self.task_wizard = None;
        self.mode = Mode::HierarchicalSelection;
        self.current_view = ViewType::HierarchicalTaskPicker;
    }

    fn confirm_task_detail_field_input(&mut self) {
        if let Some(ref mut wizard) = self.task_wizard {
            task_wizard::confirm_task_detail_field_input(wizard, &self.input_buffer);
        }
        self.input_buffer.clear();
        self.mode = Mode::TaskDetailWizard;
        self.current_view = ViewType::TaskDetailWizard;
    }

    fn cancel_planning_wizard(&mut self) {
        // Clear wizard state
        self.planning_wizard = None;
        self.hierarchical_picker = hierarchical_picker::HierarchicalPickerState::new();
        self.current_view = ViewType::TreeView;
        self.mode = Mode::Normal;
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
            app.tree_data.programs.is_empty(),
            "Programs should be empty in new workspace"
        );

        // Verify sidebar has items (Planning and Journal sections should exist)
        assert!(
            !app.navigation_state.sidebar_items.is_empty(),
            "Sidebar should have items even with empty programs"
        );

        // Open command palette
        app.handle_key(KeyCode::Char('/'));
        assert!(matches!(app.mode, Mode::CommandPalette));

        // Verify "New Program" is in the command list
        assert!(
            app.command_palette
                .matches
                .iter()
                .any(|c| c.label == "New Program"),
            "New Program command should be available even with empty workspace"
        );

        // Verify we can navigate the command list
        assert!(
            !app.command_palette.matches.is_empty(),
            "Command list should not be empty"
        );

        // Verify we can select "New Program" command
        let new_program_idx = app
            .command_palette
            .matches
            .iter()
            .position(|c| c.label == "New Program");
        assert!(
            new_program_idx.is_some(),
            "Should be able to find New Program command index"
        );

        // Navigate to New program command
        if let Some(idx) = new_program_idx {
            app.command_palette.selection_index = idx;
            assert_eq!(app.command_palette.matches[idx].label, "New Program");
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

        app.wizard_state.template = Some(TemplateFieldState {
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
        if let Some(state) = &app.wizard_state.template {
            assert_eq!(state.fields[0].value, "Hello");
        }

        // Test backspace
        app.handle_key(KeyCode::Backspace);
        assert_eq!(app.input_buffer, "Hell");
        if let Some(state) = &app.wizard_state.template {
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
        assert!(
            !app.tree_data.programs.is_empty(),
            "Programs should be loaded"
        );
        assert_eq!(
            app.navigation_state.sidebar_tree.selected_depth(),
            0,
            "Should be at root level"
        );

        // Navigate into the program (select index 1 because index 0 is "Programs" header)
        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();

        // Verify we're now inside the program
        assert_eq!(
            app.navigation_state.sidebar_tree.selected_depth(),
            1,
            "Should be inside program"
        );
        assert!(
            app.navigation_state.current_program.is_some(),
            "Current program should be set"
        );

        // Now simulate creating a new project via the wizard
        // First, start the new project wizard
        app.start_new_project();

        // Verify we're in template field input mode
        assert_eq!(app.current_view, ViewType::InputTemplateField);
        assert!(app.wizard_state.template.is_some());

        // Fill in the project name in the first editable field
        if let Some(ref mut state) = app.wizard_state.template {
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
            app.navigation_state.sidebar_tree.selected_depth(),
            1,
            "Should stay at parent level (program) after creation"
        );
        assert!(
            app.navigation_state.current_project.is_none(),
            "Should NOT auto-navigate into project - current_project should be None"
        );

        // Verify we're back in TreeView
        assert_eq!(app.current_view, ViewType::TreeView);

        // The key test: verify that sidebar_items reflects the current state
        // After creating a project, we should still be in the program, showing projects
        assert!(
            !app.navigation_state.sidebar_items.is_empty(),
            "Sidebar should have items after creation"
        );

        // Selected nodes keep children collapsed, so project children are not shown here.
        let has_new_project = app
            .navigation_state
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
        assert!(app.wizard_state.template.is_some());

        // Fill in the program name in the first editable field
        if let Some(ref mut state) = app.wizard_state.template {
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
        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();

        // Start the new project wizard
        app.start_new_project();

        // Fill in the project name
        if let Some(ref mut state) = app.wizard_state.template {
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
        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();
        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();

        // Start the new milestone wizard
        app.start_new_milestone();

        // Fill in the milestone name
        if let Some(ref mut state) = app.wizard_state.template {
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
        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();
        let project_idx = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "NewProject" && i.indent == 1)
            .expect("NewProject should be selectable");
        app.navigation_state.selected_entry_index = project_idx;
        app.open_tree_item();

        // Start the new task wizard
        app.start_new_task();

        // Fill in the task name
        if let Some(ref mut state) = app.wizard_state.template {
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
        assert!(
            !app.tree_data.programs.is_empty(),
            "Programs should be loaded"
        );

        // Record the initial selection position (before creating new element)
        let _initial_selected_index = app.navigation_state.selected_entry_index;

        // Start the new program wizard
        app.start_new_program();

        // Fill in the program name in the first editable field
        if let Some(ref mut state) = app.wizard_state.template {
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
            .navigation_state
            .sidebar_items
            .iter()
            .any(|item| item.name == "NewProgram");

        assert!(
            new_element_in_sidebar,
            "Newly created element should be in sidebar"
        );

        // The key assertion: selected_entry_index should point to the new element
        let selected_item =
            &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
        assert_eq!(
            selected_item.name, "NewProgram",
            "Selected item should be the newly created program, but got '{}' (index {})",
            selected_item.name, app.navigation_state.selected_entry_index
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
        assert!(app.wizard_state.template.is_some());

        // Check that DESCRIPTION is in the wizard fields
        let has_description_field = app
            .wizard_state
            .template
            .as_ref()
            .map(|state| state.fields.iter().any(|f| f.placeholder == "DESCRIPTION"))
            .unwrap_or(false);

        assert!(
            has_description_field,
            "Wizard should have DESCRIPTION field"
        );

        // Fill in the program name and description
        if let Some(ref mut state) = app.wizard_state.template {
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
        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();

        // Navigate into Project
        let project_idx = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "TestProject")
            .expect("TestProject should be in sidebar");
        app.navigation_state.selected_entry_index = project_idx;
        app.open_tree_item();

        // Single right from project now expands and moves selection into milestone.
        assert_eq!(
            app.navigation_state.sidebar_tree.selected_depth(),
            3,
            "Should be inside milestone after second navigation"
        );

        // Now press left arrow to navigate back
        app.navigate_left();

        // BUG: This should go to project level (path = ["TestProgram", "TestProject"])
        // but it jumps to program level (path = ["TestProgram"])
        assert_eq!(
            app.navigation_state.sidebar_tree.selected_depth(),
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
        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();
        assert_eq!(app.navigation_state.sidebar_tree.selected_depth(), 2);

        // Navigate into Project
        let project_idx = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "TestProject")
            .expect("TestProject should be in sidebar");
        app.navigation_state.selected_entry_index = project_idx;
        app.open_tree_item();
        assert_eq!(app.navigation_state.sidebar_tree.selected_depth(), 3);

        // Now navigate LEFT - this should collapse back to project level
        app.navigate_left();

        // After collapsing, we should be at project level (depth 2)
        // path should be ["TestProgram", "TestProject"], not ["TestProgram"]
        assert_eq!(
            app.navigation_state.sidebar_tree.selected_depth(),
            2,
            "After collapsing milestone, should be at project level (depth 2), not program level (depth 1)"
        );

        // The sidebar should keep milestones collapsed while project is selected.
        let has_milestones = app
            .navigation_state
            .sidebar_items
            .iter()
            .any(|i| i.name == "TestMilestone");
        assert!(
            !has_milestones,
            "Sidebar should keep selected project's children collapsed"
        );

        // Selection should remain valid and on the project node after collapsing back.
        assert!(
            app.navigation_state.selected_entry_index < app.navigation_state.sidebar_items.len(),
            "Selected index should remain in bounds"
        );
        let selected =
            &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
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

        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();

        assert_eq!(
            app.navigation_state.sidebar_tree.selected_path().to_vec(),
            vec!["TestProgram".to_string(), "AlphaProject".to_string()]
        );
        assert!(
            !app.navigation_state
                .sidebar_items
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

        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item_with_leaf_open(false);
        let project_idx = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "TestProject" && i.indent == 1)
            .expect("TestProject should be selectable");
        app.navigation_state.selected_entry_index = project_idx;
        app.open_tree_item_with_leaf_open(false);

        assert_eq!(
            app.navigation_state.sidebar_tree.selected_path().to_vec(),
            vec![
                "TestProgram".to_string(),
                "TestProject".to_string(),
                "M1".to_string()
            ]
        );

        app.navigate_left();

        assert_eq!(
            app.navigation_state.sidebar_tree.selected_path().to_vec(),
            vec!["TestProgram".to_string(), "TestProject".to_string()]
        );
        assert!(
            !app.navigation_state
                .sidebar_items
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

        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item_with_leaf_open(false);
        let project_idx = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "TestProject" && i.indent == 1)
            .expect("TestProject should be selectable");
        app.navigation_state.selected_entry_index = project_idx;
        app.open_tree_item_with_leaf_open(false);
        let milestone_idx = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "M1" && i.indent == 2)
            .expect("M1 should be selectable");
        app.navigation_state.selected_entry_index = milestone_idx;
        app.open_tree_item_with_leaf_open(false);
        let task_idx = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "LeafTask" && i.indent == 3)
            .expect("LeafTask should be selectable after entering milestone");
        app.navigation_state.selected_entry_index = task_idx;
        app.open_tree_item_with_leaf_open(false);

        let selected_before = app.navigation_state.sidebar_tree.selected_path().to_vec();
        app.navigate_right();
        app.navigate_right();

        assert_eq!(app.current_view, ViewType::TreeView);
        assert_eq!(
            app.navigation_state.sidebar_tree.selected_path().to_vec(),
            selected_before
        );
        assert!(
            app.navigation_state.selected_entry_index < app.navigation_state.sidebar_items.len()
        );
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
            .navigation_state
            .sidebar_items
            .iter()
            .position(|item| item.is_journal_item.as_deref() == Some("Today"))
            .expect("Today entry should exist");
        app.navigation_state.selected_entry_index = today_idx;
        app.open_tree_item();

        assert_eq!(app.current_view, ViewType::TreeView);
        assert!(app.wizard_state.template.is_none());
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
        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();

        // Move selection in project list and enter BetaProject.
        if let Some(beta_idx) = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "BetaProject" && i.indent == 1)
        {
            app.navigation_state.selected_entry_index = beta_idx;
        } else {
            panic!("BetaProject should be selectable");
        }
        app.open_tree_item();

        // Single right from project expands and moves to milestone.
        assert_eq!(
            app.navigation_state.sidebar_tree.selected_path().to_vec(),
            vec!["TestProgram", "BetaProject", "M1"]
        );

        // Collapse back one level.
        app.navigate_left();

        assert_eq!(
            app.navigation_state.sidebar_tree.selected_path().to_vec(),
            vec!["TestProgram", "BetaProject"]
        );
        assert!(
            app.navigation_state.selected_entry_index < app.navigation_state.sidebar_items.len()
        );
        let selected =
            &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
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

        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();
        assert_eq!(
            app.navigation_state.sidebar_tree.selected_path().to_vec(),
            vec!["TestProgram", "DirectTask"]
        );

        app.navigate_left();
        assert_eq!(
            app.navigation_state.sidebar_tree.selected_path().to_vec(),
            vec!["TestProgram"]
        );
        assert!(
            app.navigation_state.selected_entry_index < app.navigation_state.sidebar_items.len()
        );
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

        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();
        let common_idx = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "Common" && i.indent == 1)
            .expect("Common should be selectable under program");
        app.navigation_state.selected_entry_index = common_idx;
        app.open_tree_item();

        assert_eq!(
            app.navigation_state.sidebar_tree.selected_path().to_vec(),
            vec!["TestProgram", "Common", "M1"]
        );
        assert!(
            app.navigation_state
                .sidebar_items
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

        app.navigation_state.selected_entry_index = 1;
        app.open_tree_item();

        assert!(
            app.navigation_state
                .sidebar_items
                .iter()
                .any(|i| i.name == "project 1" && i.indent == 1),
            "project 1 should exist as a project under program"
        );
        assert!(
            app.navigation_state
                .sidebar_items
                .iter()
                .any(|i| i.name == "project 2" && i.indent == 1),
            "project 2 should exist as a project under program"
        );
        assert!(
            !app.navigation_state
                .sidebar_items
                .iter()
                .any(|i| i.name == "milestone 1" && i.indent == 1),
            "milestone 1 must not leak into program level"
        );

        let project_idx = app
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "project 1" && i.indent == 1)
            .expect("project 1 should be selectable");
        app.navigation_state.selected_entry_index = project_idx;
        app.open_tree_item();

        assert!(
            app.navigation_state
                .sidebar_items
                .iter()
                .any(|i| i.name == "milestone 1" && i.indent == 2),
            "milestone 1 should appear only under project 1"
        );
    }
}
