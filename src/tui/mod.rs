pub mod app;
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

#[cfg(test)]
mod tests;

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
use ratatui::{Frame, Terminal, backend::CrosstermBackend, style::Style};
use std::io::{self, Write};

use crate::config::Config;
use crate::model::{PlanningSession, SelectedTask, SessionStatus};
use crate::storage::md::parse_element;
use crate::storage::planning::{
    archive_planning_session, create_planning_session, generate_session_uuid, list_active_sessions,
    load_planning_session, save_planning_session,
};
use crate::storage::{
    DirectoryEntry, JournalEntry, JournalStorage, NotesStorage, WorkspaceStorage,
    validate_element_name,
};
use crate::theme::Theme;
use cache::{TaskMetadata, TreeData};
use command::CommandPalette;
use hierarchical_picker::HierarchicalPickerState;
use navigation::{
    JournalTreeState, NavigationState, NotesTreeState, SidebarItem, SidebarNodeData, SidebarSection,
};
use planning_session::PlanningSessionState;
use planning_wizard::PlanningDateFocus;
use review::ReviewState;
use wizard::{FieldInfo, FieldKind, TemplateFieldState, WizardFocus, WizardState};

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
    /// User is navigating the Current Plan report by tasks
    CurrentPlanNavigation,
    /// User is selecting a theme
    ThemeSelection,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    TreeView,
    Journal,
    JournalArchiveList,
    #[allow(dead_code)]
    JournalToday,
    Backlog,
    MyTasks,
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
    InputNote,
    MoveNote,
}

#[derive(Debug, Clone)]
pub(crate) enum JournalNavNode {
    Today,
    History,
    Header(Vec<String>),
    Entry(Vec<String>),
    OtherAction,
}

#[derive(Debug)]
pub struct ElementReportRow {
    pub name: String,
    pub status: String,
    pub grandchild_count: usize,
}

#[derive(Debug)]
pub struct ElementReport {
    pub child_plural: &'static str,
    pub grandchild_singular: &'static str,
    pub grandchild_plural: &'static str,
    pub rows: Vec<ElementReportRow>,
}

#[derive(Debug)]
pub struct SelectedElementView {
    pub title: String,
    pub status: String,
    pub content: String,
    pub report: ElementReport,
}

struct ReportDefinition {
    child_plural: &'static str,
    grandchild_singular: &'static str,
    grandchild_plural: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningReportKind {
    WeeklyPlanning,
    Backlog,
    MyTasks,
}

impl PlanningReportKind {
    fn from_sidebar_tag(tag: &str) -> Option<Self> {
        match tag {
            "WeeklyPlanning" => Some(Self::WeeklyPlanning),
            "Backlog" => Some(Self::Backlog),
            "MyTasks" => Some(Self::MyTasks),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DailyTodoItem {
    pub line_index: usize,
    pub checked: bool,
    pub text: String,
}

/// Step in the two-step new-note wizard (category → folder → template).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoteWizardStep {
    Category,
    Folder,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoteCreationMode {
    Note,
    FolderOnly,
}

/// State for the new-note wizard before the main template wizard opens.
#[derive(Debug, Clone)]
pub struct NoteCreationState {
    pub step: NoteWizardStep,
    pub mode: NoteCreationMode,
    pub categories: Vec<String>,
    pub selected_category_index: usize,
    pub folder_input: String,
    pub available_folders: Vec<String>,
    pub selected_folder_index: usize,
}

/// State for the move-note picker.
#[derive(Debug, Clone)]
pub struct MoveNoteState {
    pub source_path: std::path::PathBuf,
    pub destinations: Vec<String>,
    pub selected_index: usize,
    pub filter: String,
}

impl MoveNoteState {
    pub fn filtered_destinations(&self) -> Vec<(usize, &str)> {
        let f = self.filter.to_lowercase();
        self.destinations
            .iter()
            .enumerate()
            .filter(|(_, d)| f.is_empty() || d.to_lowercase().contains(&f))
            .map(|(i, d)| (i, d.as_str()))
            .collect()
    }
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
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
    pub template_edit_target_path: Option<std::path::PathBuf>,
    pub template_edit_selected_path: Option<Vec<String>>,
    pub planning_session: PlanningSessionState,
    pub review_state: ReviewState,
    // Hierarchical task picker state
    pub hierarchical_picker: HierarchicalPickerState,
    // Extracted planning wizard state (replaces fields above)
    pub planning_wizard: Option<planning_wizard::PlanningWizardState>,
    // Task wizard state
    pub task_wizard: Option<task_wizard::TaskWizardState>,
    // True when task wizard edits a subtask selected in TreeView (not add-to-plan flow).
    pub task_wizard_tree_edit_mode: bool,
    pub backlog_staged_uuids: Option<Vec<String>>,
    // Planning preview confirmed flag
    // Archive list tree structure (maps tree index -> journal entry index)
    pub archive_tree_mapping: Vec<Option<usize>>,
    // Journal tree state for sidebar expansion
    pub journal_tree_state: JournalTreeState,
    // Notes tree state for sidebar expansion
    pub notes_tree_state: NotesTreeState,
    // Note flow state (category → folder step, then note template or folder creation)
    pub note_creation_state: Option<NoteCreationState>,
    // Move note picker state
    pub move_note_state: Option<MoveNoteState>,
    // Theme selection state
    pub available_themes: Vec<String>,
    pub theme_selection_index: usize,
    pub previous_theme: Option<crate::theme::Theme>,
}

impl App {
    const FOCUS_ASSIGNED_TO: usize = 100;
    const FOCUS_START_DATE: usize = 101;
    const FOCUS_DUE_DATE: usize = 102;
    pub(crate) const JOURNAL_EXPANSION_ROOT: &'static str = "__journal__";
    pub(crate) const NOTES_EXPANSION_ROOT: &'static str = "__notes__";

    pub fn new(config: Config) -> Self {
        let theme = config.load_theme().unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to load theme '{}': {}, using default",
                config.theme,
                e
            );
            crate::theme::default_theme()
        });

        let mut app = App {
            config,
            theme,
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
            template_edit_target_path: None,
            template_edit_selected_path: None,
            planning_session: PlanningSessionState::new(),
            review_state: ReviewState::new(),
            hierarchical_picker: HierarchicalPickerState::new(),
            planning_wizard: None,
            task_wizard: None,
            task_wizard_tree_edit_mode: false,
            backlog_staged_uuids: None,
            archive_tree_mapping: Vec::new(),
            journal_tree_state: JournalTreeState::new(),
            notes_tree_state: NotesTreeState::new(),
            note_creation_state: None,
            move_note_state: None,
            available_themes: crate::theme::loader::list_available_themes(),
            theme_selection_index: 0,
            previous_theme: None,
        };

        app.load_tree_view_data();
        app.ensure_journal_entries_loaded();
        app.ensure_notes_loaded();
        app.resume_planning_session();
        app
    }

    pub fn background_style(&self) -> ratatui::style::Style {
        self.theme.ui.background.unwrap_or_default()
    }

    pub fn text_primary(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .text
            .primary
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::White))
    }

    pub fn text_secondary(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .text
            .secondary
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn sidebar_style(&self) -> ratatui::style::Style {
        self.theme.ui.sidebar.item.unwrap_or_default()
    }

    pub fn sidebar_selected_style(&self) -> ratatui::style::Style {
        self.theme.ui.sidebar.selected.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn sidebar_header_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .sidebar
            .header
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn sidebar_create_action_style(&self, selected: bool) -> ratatui::style::Style {
        if let Some(style) = &self.theme.ui.sidebar.create_action {
            if selected {
                return (*style)
                    .bg(ratatui::style::Color::Cyan)
                    .fg(ratatui::style::Color::Black);
            }
            return *style;
        }
        if selected {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Cyan)
        } else {
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)
        }
    }

    pub fn border_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .border
            .normal
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn status_color(&self) -> ratatui::style::Color {
        match self.mode {
            Mode::Normal => self
                .theme
                .ui
                .status
                .normal
                .unwrap_or(ratatui::style::Color::Green),
            Mode::CommandPalette => self
                .theme
                .ui
                .status
                .command
                .unwrap_or(ratatui::style::Color::Yellow),
            Mode::Input | Mode::InputTaskDetailField => self
                .theme
                .ui
                .status
                .input
                .unwrap_or(ratatui::style::Color::Cyan),
            Mode::TaskSelection => self
                .theme
                .ui
                .status
                .select
                .unwrap_or(ratatui::style::Color::Magenta),
            Mode::ReviewSession => self
                .theme
                .ui
                .status
                .review
                .unwrap_or(ratatui::style::Color::LightMagenta),
            Mode::HierarchicalSelection => self
                .theme
                .ui
                .status
                .hierarchical_selection
                .unwrap_or(ratatui::style::Color::LightCyan),
            Mode::PlanningPreview => self
                .theme
                .ui
                .status
                .planning_preview
                .unwrap_or(ratatui::style::Color::LightBlue),
            Mode::TaskDetailWizard => self
                .theme
                .ui
                .status
                .task_detail_wizard
                .unwrap_or(ratatui::style::Color::LightYellow),
            Mode::CurrentPlanNavigation => self
                .theme
                .ui
                .status
                .current_plan_navigation
                .unwrap_or(ratatui::style::Color::LightCyan),
            Mode::ThemeSelection => self
                .theme
                .ui
                .status
                .theme_selection
                .unwrap_or(ratatui::style::Color::LightCyan),
        }
    }

    pub fn command_input_style(&self) -> ratatui::style::Style {
        self.theme.ui.command.input.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::White)
                .bg(ratatui::style::Color::Black)
        })
    }

    pub fn command_result_style(&self) -> ratatui::style::Style {
        self.theme.ui.command.result.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::White)
                .bg(ratatui::style::Color::Black)
        })
    }

    pub fn command_result_selected_style(&self) -> ratatui::style::Style {
        self.theme.ui.command.result_selected.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn command_section_style(&self) -> ratatui::style::Style {
        self.text_secondary()
            .add_modifier(ratatui::style::Modifier::BOLD)
    }

    pub fn command_border_style(&self) -> ratatui::style::Style {
        self.theme.ui.border.focused.unwrap_or_else(|| {
            ratatui::style::Style::default().fg(ratatui::style::Color::LightBlue)
        })
    }

    pub fn theme_border_style(&self) -> ratatui::style::Style {
        self.theme.ui.border.focused.unwrap_or_else(|| {
            ratatui::style::Style::default().fg(ratatui::style::Color::LightCyan)
        })
    }

    pub fn content_title_style(&self) -> ratatui::style::Style {
        self.theme.ui.content.title.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn content_header_style(&self) -> ratatui::style::Style {
        self.theme.ui.content.header.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn content_table_header_style(&self) -> ratatui::style::Style {
        self.theme.ui.content.table_header.unwrap_or_else(|| {
            ratatui::style::Style::default().fg(ratatui::style::Color::LightBlue)
        })
    }

    pub fn content_table_border_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .content
            .table_border
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn wizard_field_label_style(&self) -> ratatui::style::Style {
        self.theme.ui.wizard.field_label.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn wizard_field_value_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .wizard
            .field_value
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::White))
    }

    pub fn wizard_field_empty_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .wizard
            .field_value_empty
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn wizard_field_auto_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .wizard
            .field_value_auto
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn wizard_button_confirm_style(&self) -> ratatui::style::Style {
        self.theme.ui.wizard.button_confirm.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn wizard_button_cancel_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .wizard
            .button_cancel
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn wizard_button_selected_style(&self) -> ratatui::style::Style {
        self.theme.ui.wizard.button_selected.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::LightBlue)
        })
    }

    pub fn selection_bg(&self) -> ratatui::style::Color {
        self.theme
            .ui
            .selection
            .bg
            .unwrap_or(ratatui::style::Color::LightBlue)
    }

    pub fn selection_fg(&self) -> ratatui::style::Style {
        if let Some(fg) = self.theme.ui.selection.fg {
            Style::default().fg(fg)
        } else {
            Style::default().fg(ratatui::style::Color::Black)
        }
    }

    pub fn hierarchy_program_style(&self) -> ratatui::style::Style {
        Style::default().add_modifier(ratatui::style::Modifier::BOLD)
    }

    pub fn hierarchy_project_style(&self) -> ratatui::style::Style {
        Style::default().add_modifier(ratatui::style::Modifier::ITALIC)
    }

    pub fn hierarchy_milestone_style(&self) -> ratatui::style::Style {
        Style::default().fg(ratatui::style::Color::LightCyan)
    }

    pub fn selection_active_style(&self) -> ratatui::style::Style {
        self.theme.ui.selection.active.unwrap_or_else(|| {
            Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Yellow)
        })
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
                    self.cancel_planning_wizard();
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

        // Handle CurrentPlanNavigation mode specially
        if self.mode == Mode::CurrentPlanNavigation {
            match self.active_planning_report() {
                Some(PlanningReportKind::WeeklyPlanning) => match code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.navigate_review(-1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.navigate_review(1);
                    }
                    KeyCode::Char('s') => {
                        self.cycle_task_status();
                    }
                    KeyCode::Char('x') => {
                        self.remove_task_from_session();
                    }
                    KeyCode::Enter => {
                        self.open_current_plan_task_in_editor();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.mode = Mode::Normal;
                        self.current_view = ViewType::TreeView;
                    }
                    _ => {}
                },
                Some(PlanningReportKind::Backlog) => match code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.navigate_report_rows(-1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.navigate_report_rows(1);
                    }
                    KeyCode::Char('a') => {
                        self.assign_selected_backlog_task_to_owner();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.mode = Mode::Normal;
                        self.current_view = ViewType::TreeView;
                    }
                    _ => {}
                },
                Some(PlanningReportKind::MyTasks) => match code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.navigate_report_rows(-1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.navigate_report_rows(1);
                    }
                    KeyCode::Char('s') => {
                        self.cycle_selected_my_task_status();
                    }
                    KeyCode::Char('x') => {
                        self.toggle_selected_my_todo();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.mode = Mode::Normal;
                        self.current_view = ViewType::TreeView;
                    }
                    _ => {}
                },
                None => {
                    self.mode = Mode::Normal;
                }
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
                            self.show_current_plan_report();
                        }
                        2 => self.cancel_planning_wizard(),
                        _ => {}
                    }
                }
                KeyCode::Esc => {
                    if self.review_state.preview_focus == 2 {
                        self.cancel_planning_wizard();
                    } else {
                        self.review_state.preview_focus = 2;
                    }
                }
                _ => {}
            }
            return;
        }

        // Handle TaskDetailWizard mode specially
        if self.mode == Mode::TaskDetailWizard {
            const TASK_WIZARD_FIELD_COUNT: usize = 7; // task name, status, assigned_to, start_date, due_date, importance, description
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
                        if let Some(ref mut wizard) = self.task_wizard {
                            if wizard.field_index == 8 {
                                self.cancel_task_detail_wizard();
                            } else {
                                // Escape jumps to CANCEL button
                                wizard.field_index = 8; // CancelButton index
                            }
                        }
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        if let Some(ref mut wizard) = self.task_wizard {
                            task_wizard::cycle_task_wizard_choice(
                                wizard,
                                -1,
                                &self.config.workflow,
                                &self.config.importance,
                            );
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if let Some(ref mut wizard) = self.task_wizard {
                            task_wizard::cycle_task_wizard_choice(
                                wizard,
                                1,
                                &self.config.workflow,
                                &self.config.importance,
                            );
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

        // Handle ThemeSelection mode
        if self.mode == Mode::ThemeSelection {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.theme_selection_index > 0 {
                        self.theme_selection_index -= 1;
                        self.preview_theme();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.theme_selection_index < self.available_themes.len().saturating_sub(1) {
                        self.theme_selection_index += 1;
                        self.preview_theme();
                    }
                }
                KeyCode::Enter => {
                    self.confirm_theme_selection();
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.cancel_theme_selection();
                }
                _ => {}
            }
            return;
        }

        // Handle InputNote view (two-step: category then folder)
        if self.current_view == ViewType::InputNote {
            match code {
                KeyCode::Up => {
                    if let Some(ref mut state) = self.note_creation_state
                        && state.step == NoteWizardStep::Category
                    {
                        if state.selected_category_index > 0 {
                            state.selected_category_index -= 1;
                        }
                    } else if let Some(ref mut state) = self.note_creation_state
                        && state.step == NoteWizardStep::Folder
                        && state.mode == NoteCreationMode::Note
                        && state.selected_folder_index > 0
                    {
                        state.selected_folder_index -= 1;
                    }
                }
                KeyCode::Char('k')
                    if self
                        .note_creation_state
                        .as_ref()
                        .is_some_and(|s| s.step == NoteWizardStep::Category) =>
                {
                    if let Some(ref mut state) = self.note_creation_state
                        && state.selected_category_index > 0
                    {
                        state.selected_category_index -= 1;
                    }
                }
                KeyCode::Char('k')
                    if self.note_creation_state.as_ref().is_some_and(|s| {
                        s.step == NoteWizardStep::Folder && s.mode == NoteCreationMode::Note
                    }) =>
                {
                    if let Some(ref mut state) = self.note_creation_state
                        && state.selected_folder_index > 0
                    {
                        state.selected_folder_index -= 1;
                    }
                }
                KeyCode::Down => {
                    if let Some(ref mut state) = self.note_creation_state
                        && state.step == NoteWizardStep::Category
                    {
                        let max = state.categories.len().saturating_sub(1);
                        if state.selected_category_index < max {
                            state.selected_category_index += 1;
                        }
                    } else if let Some(ref mut state) = self.note_creation_state
                        && state.step == NoteWizardStep::Folder
                        && state.mode == NoteCreationMode::Note
                    {
                        let max = state.available_folders.len();
                        if state.selected_folder_index < max {
                            state.selected_folder_index += 1;
                        }
                    }
                }
                KeyCode::Char('j')
                    if self
                        .note_creation_state
                        .as_ref()
                        .is_some_and(|s| s.step == NoteWizardStep::Category) =>
                {
                    if let Some(ref mut state) = self.note_creation_state {
                        let max = state.categories.len().saturating_sub(1);
                        if state.selected_category_index < max {
                            state.selected_category_index += 1;
                        }
                    }
                }
                KeyCode::Char('j')
                    if self.note_creation_state.as_ref().is_some_and(|s| {
                        s.step == NoteWizardStep::Folder && s.mode == NoteCreationMode::Note
                    }) =>
                {
                    if let Some(ref mut state) = self.note_creation_state {
                        let max = state.available_folders.len();
                        if state.selected_folder_index < max {
                            state.selected_folder_index += 1;
                        }
                    }
                }
                KeyCode::Enter => {
                    self.handle_note_wizard_enter();
                }
                KeyCode::Backspace => {
                    if self.note_creation_state.as_ref().is_some_and(|s| {
                        s.step == NoteWizardStep::Folder && s.mode == NoteCreationMode::FolderOnly
                    }) {
                        self.handle_input_backspace();
                    }
                }
                KeyCode::Char(c) => {
                    if self.note_creation_state.as_ref().is_some_and(|s| {
                        s.step == NoteWizardStep::Folder && s.mode == NoteCreationMode::FolderOnly
                    }) {
                        self.handle_input_char(c);
                    }
                }
                KeyCode::Esc => {
                    self.note_creation_state = None;
                    self.current_view = ViewType::TreeView;
                }
                _ => {}
            }
            return;
        }

        // Handle MoveNote view (filterable destination picker)
        if self.current_view == ViewType::MoveNote {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(ref mut state) = self.move_note_state
                        && state.selected_index > 0
                    {
                        state.selected_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ref mut state) = self.move_note_state {
                        let max = state.filtered_destinations().len().saturating_sub(1);
                        if state.selected_index < max {
                            state.selected_index += 1;
                        }
                    }
                }
                KeyCode::Enter => {
                    self.confirm_move_note();
                }
                KeyCode::Backspace => {
                    self.handle_input_backspace();
                }
                KeyCode::Char(c) => {
                    self.handle_input_char(c);
                }
                KeyCode::Esc => {
                    self.move_note_state = None;
                    self.current_view = ViewType::TreeView;
                }
                _ => {}
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
                            field.was_edited = true;
                        }
                        if state.focus == WizardFocus::CancelButton {
                            self.wizard_state.template = None;
                            self.current_view = ViewType::TreeView;
                        } else {
                            state.focus = WizardFocus::CancelButton;
                            self.input_buffer.clear();
                        }
                    }
                } else if self.current_view == ViewType::InputPlanningSessionDates {
                    // ESC jumps to CancelButton instead of canceling
                    if let Some(ref mut wizard) = self.planning_wizard {
                        if wizard.focus == PlanningDateFocus::CancelButton {
                            self.cancel_planning_wizard();
                        } else {
                            wizard.focus = PlanningDateFocus::CancelButton;
                        }
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
                if self.current_view == ViewType::InputTemplateField {
                    self.cycle_template_choice(1);
                } else if self.current_view == ViewType::InputPlanningSessionDates {
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
                if self.current_view == ViewType::InputTemplateField {
                    self.cycle_template_choice(-1);
                } else if self.current_view == ViewType::InputPlanningSessionDates {
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
            KeyCode::Char('e') => {
                if !(self.current_view == ViewType::TreeView
                    && self.open_metadata_editor_for_selected_item())
                {
                    self.handle_input_char('e');
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

    fn cycle_template_choice(&mut self, delta: isize) {
        if let Some(ref mut state) = self.wizard_state.template {
            match state.focus {
                WizardFocus::Field(idx) => {
                    let Some(field) = state.fields.get_mut(idx) else {
                        return;
                    };
                    if field.choices.is_empty() || !field.is_editable {
                        return;
                    }

                    let len = field.choices.len() as isize;
                    let current_idx = field
                        .choices
                        .iter()
                        .position(|choice| choice.eq_ignore_ascii_case(&field.value))
                        .unwrap_or(0) as isize;
                    let next_idx = (current_idx + delta).rem_euclid(len) as usize;
                    field.value = field.choices[next_idx].clone();
                    field.was_edited = true;
                    self.input_buffer = field.value.clone();
                }
                WizardFocus::ConfirmButton => {
                    state.focus = WizardFocus::CancelButton;
                }
                WizardFocus::CancelButton => {
                    state.focus = WizardFocus::ConfirmButton;
                }
            }
        }
    }

    fn navigate_right(&mut self) {
        if self.current_view == ViewType::TreeView {
            // Notes tree navigation
            if let Some(note_path) = self.selected_note_tree_path() {
                if let Some(SidebarNodeData::Note(node)) = self.selected_note_node() {
                    use crate::tui::cache::NoteNode;
                    match node {
                        NoteNode::Category { .. } | NoteNode::Folder { .. } => {
                            if !self.notes_is_expanded(&note_path) {
                                self.notes_expand_path(&note_path);
                                self.load_tree_view_data();
                                self.select_first_notes_child(&note_path);
                            }
                        }
                        NoteNode::Entry { .. } => {
                            // Leaf note — right is a no-op
                        }
                    }
                }
                return;
            }

            if let Some(journal_node) = self.selected_journal_node() {
                match journal_node {
                    JournalNavNode::History => {
                        if !self.journal_is_expanded(&[]) {
                            self.expand_history();
                        }
                    }
                    JournalNavNode::Header(path) => {
                        if !self.journal_is_expanded(&path) {
                            self.expand_journal_item(&path);
                        }
                    }
                    JournalNavNode::Today
                    | JournalNavNode::Entry(_)
                    | JournalNavNode::OtherAction => {
                        // Right on journal leaves/actions with no children is a no-op.
                    }
                }
                return;
            }

            let idx = self.navigation_state.selected_entry_index;
            if idx < self.navigation_state.sidebar_items.len() {
                tracing::debug!(
                    selected_index = self.navigation_state.selected_entry_index,
                    path = ?self.navigation_state.sidebar_tree.selected_path(),
                    "navigate right"
                );
                self.open_tree_item_with_leaf_open(false);
            }
        }
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

        // Notes tree left navigation
        if let Some(note_path) = self.selected_note_tree_path() {
            use crate::tui::cache::NoteNode;
            if let Some(SidebarNodeData::Note(node)) = self.selected_note_node() {
                match node {
                    NoteNode::Category { .. } => {
                        // Root-level categories have no parent to collapse into.
                    }
                    NoteNode::Folder { .. } => {
                        let parent = note_path[..note_path.len().saturating_sub(1)].to_vec();
                        self.notes_collapse_path(&note_path);
                        self.notes_collapse_path(&parent);
                        self.load_tree_view_data();
                        self.select_notes_item_by_path(&parent);
                    }
                    NoteNode::Entry { .. } => {
                        let parent = note_path[..note_path.len().saturating_sub(1)].to_vec();
                        self.notes_collapse_path(&note_path);
                        self.notes_collapse_path(&parent);
                        self.load_tree_view_data();
                        self.select_notes_item_by_path(&parent);
                    }
                }
            }
            return;
        }

        if let Some(journal_node) = self.selected_journal_node() {
            match journal_node {
                JournalNavNode::History | JournalNavNode::Today | JournalNavNode::OtherAction => {
                    // Root-level journal actions cannot be collapsed.
                }
                JournalNavNode::Header(path) => {
                    let parent_path = path[..path.len().saturating_sub(1)].to_vec();
                    if parent_path.is_empty() {
                        self.collapse_journal_item(&path);
                        self.collapse_journal_item(&[]);
                    } else {
                        self.collapse_journal_item(&path);
                        self.collapse_journal_item(&parent_path);
                    }
                    self.load_tree_view_data();

                    if parent_path.is_empty() {
                        self.select_history();
                    } else {
                        self.select_journal_item_by_path(&parent_path);
                    }
                }
                JournalNavNode::Entry(path) => {
                    let parent_path = path[..path.len().saturating_sub(1)].to_vec();
                    if parent_path.is_empty() {
                        self.select_history();
                    } else {
                        self.collapse_journal_item(&path);
                        self.collapse_journal_item(&parent_path);
                        self.load_tree_view_data();
                        self.select_journal_item_by_path(&parent_path);
                    }
                }
            }
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

        // Root-level task-management items have no parent to collapse into.
        if selected_path.len() == 1 && !item.has_children {
            return;
        }

        let mut parent = selected_path.clone();
        parent.pop();

        if parent.is_empty() {
            if item.has_children {
                self.collapse_path(&selected_path);
                self.load_tree_view_data();
            }
            return;
        }

        self.collapse_path(&selected_path);
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

    fn current_plan_sidebar_index(&self) -> Option<usize> {
        self.navigation_state.sidebar_items.iter().position(|item| {
            item.name == "Current Plan"
                && item.is_planning_item.as_deref() == Some("WeeklyPlanning")
        })
    }

    pub fn active_planning_report(&self) -> Option<PlanningReportKind> {
        self.navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)
            .and_then(|item| item.is_planning_item.as_deref())
            .and_then(PlanningReportKind::from_sidebar_tag)
    }

    fn show_current_plan_report(&mut self) {
        if let Some(idx) = self.current_plan_sidebar_index() {
            self.navigation_state.selected_entry_index = idx;
        }
        self.current_view = ViewType::TreeView;
        self.review_state.selection_index = 0;
        self.mode = if self.planning_session.has_tasks() {
            Mode::CurrentPlanNavigation
        } else {
            Mode::Normal
        };
    }

    fn navigate_up(&mut self) {
        let prev_section = self
            .navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)
            .map(|i| i.section.clone());

        self.navigation_state.navigate_up();
        self.sync_scope_from_sidebar_selection();
        self.clear_backlog_staging_if_needed();

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
                    self.collapse_all_journal();
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
                // If leaving Notes section, collapse all notes tree
                else if prev_section == SidebarSection::Notes && new_section != prev_section {
                    self.collapse_all_notes();
                    self.load_tree_view_data();
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
        self.clear_backlog_staging_if_needed();

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
                    self.collapse_all_journal();
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
                // If leaving Notes section, collapse all notes tree
                else if prev_section == SidebarSection::Notes && new_section != prev_section {
                    self.collapse_all_notes();
                    self.load_tree_view_data();
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

        // Handle notes items via node_data
        if item.section == SidebarSection::Notes {
            if let SidebarNodeData::Note(ref node) = item.node_data.clone() {
                use crate::tui::cache::NoteNode;
                match node {
                    NoteNode::Category { .. } | NoteNode::Folder { .. } => {
                        let note_path = item.tree_path.clone().unwrap_or_default();
                        if !self.notes_is_expanded(&note_path) {
                            self.notes_expand_path(&note_path);
                            self.load_tree_view_data();
                            self.select_first_notes_child(&note_path);
                        }
                    }
                    NoteNode::Entry { path, .. } => {
                        if open_leaf_content {
                            let path = path.clone();
                            self.launch_editor(&path);
                        }
                    }
                }
            }
            return;
        }

        if let Some(plan_type) = &item.is_planning_item {
            match plan_type.as_str() {
                "WeeklyPlanning" => {
                    if !self.planning_session.active {
                        self.start_planning_session();
                        return;
                    }
                    self.current_view = ViewType::TreeView;
                    if self.planning_session.has_tasks() {
                        if self.review_state.selection_index >= self.planning_session.tasks.len() {
                            self.review_state.selection_index = 0;
                        }
                        self.mode = Mode::CurrentPlanNavigation;
                    } else {
                        self.mode = Mode::Normal;
                    }
                }
                "MyTasks" => {
                    self.current_view = ViewType::TreeView;
                    self.mode = Mode::CurrentPlanNavigation;
                    self.review_state.selection_index = 0;
                }
                "Backlog" => {
                    self.current_view = ViewType::TreeView;
                    self.mode = Mode::CurrentPlanNavigation;
                    if self.backlog_staged_uuids.is_none() {
                        self.backlog_staged_uuids = Some(self.backlog_task_uuids());
                    }
                    self.review_state.selection_index = 0;
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
                        self.launch_editor(&today_path);
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
            if !open_leaf_content {
                return;
            }
            // This is a journal entry - open it in viewer
            let label = navigation::journal_entry_label(jpath).unwrap_or(&item.name);
            if let Some(entry) = self
                .journal_entries
                .iter()
                .find(|e| *e.filename.trim_end_matches(".md") == *label)
            {
                let path = entry.path.clone();
                self.launch_editor(&path);
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
            if has_children {
                if self.navigation_state.sidebar_tree.is_expanded(&node_path) {
                    return;
                }
                self.navigation_state.sidebar_tree.expand_path(&node_path);
                self.set_selected_tree_path(node_path.clone());
                self.load_tree_view_data();
                self.select_first_child_for_path(&node_path);
            } else if self.navigation_state.sidebar_tree.selected_path() != node_path {
                self.set_selected_tree_path(node_path.clone());
                self.load_tree_view_data();
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

    pub(crate) fn load_tree_view_data(&mut self) {
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

    pub fn selected_element_view(&self) -> Option<SelectedElementView> {
        let idx = self.navigation_state.selected_entry_index;
        let item = self.navigation_state.sidebar_items.get(idx)?;
        if item.section != SidebarSection::Programs || item.is_header {
            return None;
        }

        let path = if !self
            .navigation_state
            .sidebar_tree
            .selected_path()
            .is_empty()
        {
            self.navigation_state.sidebar_tree.selected_path().to_vec()
        } else {
            item.tree_path.clone()?
        };
        if path.is_empty() {
            return None;
        }

        let depth = path.len();
        let def = Self::report_definition_for_depth(depth)?;
        let selected_entry = self.resolve_entry_at_path(&path)?;
        let content = self
            .config
            .workspace
            .read_md_file(&selected_entry.path)
            .unwrap_or_else(|_| "".to_string());
        let selected_status = self.status_for_entry(&selected_entry);

        let children = self.load_tree_level(&path);
        let rows = children
            .into_iter()
            .map(|child| {
                let child_name = child.name.clone();
                let status = self.status_for_entry(&child);
                let mut child_path = path.clone();
                child_path.push(child_name.clone());
                let grandchild_count = self.load_tree_level(&child_path).len();
                ElementReportRow {
                    name: child_name,
                    status,
                    grandchild_count,
                }
            })
            .collect();

        let report = ElementReport {
            child_plural: def.child_plural,
            grandchild_singular: def.grandchild_singular,
            grandchild_plural: def.grandchild_plural,
            rows,
        };

        Some(SelectedElementView {
            title: selected_entry.name.clone(),
            status: selected_status,
            content,
            report,
        })
    }

    fn report_definition_for_depth(depth: usize) -> Option<ReportDefinition> {
        match depth {
            1 => Some(ReportDefinition {
                child_plural: "Projects",
                grandchild_singular: "Milestone",
                grandchild_plural: "Milestones",
            }),
            2 => Some(ReportDefinition {
                child_plural: "Milestones",
                grandchild_singular: "Task",
                grandchild_plural: "Tasks",
            }),
            3 => Some(ReportDefinition {
                child_plural: "Tasks",
                grandchild_singular: "Subtask",
                grandchild_plural: "Subtasks",
            }),
            4 => Some(ReportDefinition {
                child_plural: "Subtasks",
                grandchild_singular: "Grandchild",
                grandchild_plural: "Grandchildren",
            }),
            _ => None,
        }
    }

    fn status_for_entry(&self, entry: &DirectoryEntry) -> String {
        self.config
            .workspace
            .read_md_file(&entry.path)
            .ok()
            .and_then(|content| parse_element(&content).ok().flatten())
            .map(|element| {
                let status = element.status().trim();
                let status_text = if status.is_empty() {
                    "unspecified"
                } else {
                    status
                };
                status_text.to_string()
            })
            .unwrap_or_else(|| "unknown".to_string())
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
            .push(SidebarItem::new("Task Management", SidebarSection::Programs).header());

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
            SidebarItem::new("My Tasks", SidebarSection::Planning)
                .planning_item("MyTasks")
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
        if self.journal_is_expanded(&[]) {
            self.add_journal_tree_items();
        }

        self.navigation_state
            .sidebar_items
            .push(SidebarItem::new("", SidebarSection::Notes));
        self.navigation_state
            .sidebar_items
            .push(SidebarItem::new("Notes", SidebarSection::Notes).header());
        self.add_notes_tree_items();
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
            ViewType::Backlog => {
                self.mode = Mode::CurrentPlanNavigation;
                self.review_state.selection_index = 0;
                if self.backlog_staged_uuids.is_none() {
                    self.backlog_staged_uuids = Some(self.backlog_task_uuids());
                }
            }
            ViewType::MyTasks => {
                self.mode = Mode::CurrentPlanNavigation;
                self.review_state.selection_index = 0;
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
            ViewType::InputNote => {
                self.handle_note_wizard_enter();
            }
            ViewType::MoveNote => {
                self.confirm_move_note();
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
                    field.was_edited = true;
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
                        if focus_idx == 0 {
                            wizard.start_date_edited = true;
                        } else {
                            wizard.end_date_edited = true;
                        }
                    }
                }
            }
            ViewType::PlanningTaskPicker => {
                if let Some(ref mut wizard) = self.planning_wizard {
                    wizard.task_filter.push(c);
                    wizard.task_index = 0;
                }
            }
            ViewType::InputNote => {
                if let Some(ref mut state) = self.note_creation_state
                    && state.step == NoteWizardStep::Folder
                    && state.mode == NoteCreationMode::FolderOnly
                {
                    self.input_buffer.push(c);
                    state.folder_input.push(c);
                }
            }
            ViewType::MoveNote => {
                if let Some(ref mut state) = self.move_note_state {
                    state.filter.push(c);
                    state.selected_index = 0;
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
                    field.was_edited = true;
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
                        if focus_idx == 0 {
                            wizard.start_date_edited = true;
                        } else {
                            wizard.end_date_edited = true;
                        }
                    }
                }
            }
            ViewType::PlanningTaskPicker => {
                if let Some(ref mut wizard) = self.planning_wizard {
                    wizard.task_filter.pop();
                }
            }
            ViewType::InputNote => {
                if let Some(ref mut state) = self.note_creation_state
                    && state.step == NoteWizardStep::Folder
                    && state.mode == NoteCreationMode::FolderOnly
                {
                    self.input_buffer.pop();
                    state.folder_input.pop();
                }
            }
            ViewType::MoveNote => {
                if let Some(ref mut state) = self.move_note_state {
                    state.filter.pop();
                    state.selected_index = 0;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn launch_editor(&mut self, path: &std::path::Path) {
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

    fn start_new_subtask(&mut self) {
        self.promote_selection_to_path_depth(1);
        self.promote_selection_to_path_depth(2);
        self.promote_selection_to_path_depth(3);
        self.promote_selection_to_path_depth(4);
        self.input_buffer.clear();
        self.open_template_wizard("subtask", None);
    }

    fn start_new_note(&mut self) {
        self.ensure_notes_loaded();
        let categories = self.config.notes.categories.clone();
        self.note_creation_state = Some(NoteCreationState {
            step: NoteWizardStep::Category,
            mode: NoteCreationMode::Note,
            categories,
            selected_category_index: 0,
            folder_input: String::new(),
            available_folders: Vec::new(),
            selected_folder_index: 0,
        });
        self.input_buffer.clear();
        self.current_view = ViewType::InputNote;
    }

    fn start_new_note_folder(&mut self) {
        self.ensure_notes_loaded();
        let categories = self.config.notes.categories.clone();
        self.note_creation_state = Some(NoteCreationState {
            step: NoteWizardStep::Category,
            mode: NoteCreationMode::FolderOnly,
            categories,
            selected_category_index: 0,
            folder_input: String::new(),
            available_folders: Vec::new(),
            selected_folder_index: 0,
        });
        self.input_buffer.clear();
        self.current_view = ViewType::InputNote;
    }

    fn confirm_note_category(&mut self) {
        if let Some(ref mut state) = self.note_creation_state {
            // Advance to folder step
            state.step = NoteWizardStep::Folder;
            state.selected_folder_index = 0;
            state.folder_input.clear();
            if state.mode == NoteCreationMode::Note {
                let category = state
                    .categories
                    .get(state.selected_category_index)
                    .cloned()
                    .unwrap_or_default();
                state.available_folders = self.notes_tree_state.folders_for_category(&category);
            } else {
                state.available_folders.clear();
            }
            self.input_buffer.clear();
        }
    }

    fn confirm_note_folder(&mut self) {
        let mut created_folder_path: Option<Vec<String>> = None;

        if let Some(ref mut state) = self.note_creation_state {
            let folder_name = if state.mode == NoteCreationMode::Note {
                if state.selected_folder_index == 0 {
                    String::new()
                } else {
                    state
                        .available_folders
                        .get(state.selected_folder_index.saturating_sub(1))
                        .cloned()
                        .unwrap_or_default()
                }
            } else {
                self.input_buffer.trim().to_string()
            };
            state.folder_input = folder_name.clone();

            if state.mode == NoteCreationMode::FolderOnly {
                if folder_name.is_empty() {
                    tracing::warn!("Folder name cannot be empty");
                    return;
                }
                if let Err(e) = validate_element_name(&folder_name) {
                    tracing::warn!("Invalid note folder name '{}': {}", folder_name, e);
                    return;
                }
                let category = state
                    .categories
                    .get(state.selected_category_index)
                    .cloned()
                    .unwrap_or_default();
                let target = self
                    .config
                    .workspace
                    .notes_dir()
                    .join(&category)
                    .join(&folder_name);
                if let Err(e) = fs::create_dir_all(&target) {
                    tracing::error!("Failed to create note folder '{}': {}", target.display(), e);
                    return;
                }
                created_folder_path = Some(vec![category, folder_name]);
            }
        }

        if let Some(folder_path) = created_folder_path {
            self.input_buffer.clear();
            self.note_creation_state = None;
            self.refresh_notes_cache();
            self.load_tree_view_data();
            let category_path = vec![folder_path[0].clone()];
            self.notes_expand_path(&category_path);
            self.load_tree_view_data();
            self.select_notes_item_by_path(&folder_path);
            self.current_view = ViewType::TreeView;
            return;
        }

        self.input_buffer.clear();
        self.open_template_wizard("note", None);
    }

    fn handle_note_wizard_enter(&mut self) {
        match self.note_creation_state.as_ref().map(|s| s.step) {
            Some(NoteWizardStep::Category) => self.confirm_note_category(),
            Some(NoteWizardStep::Folder) => self.confirm_note_folder(),
            None => {}
        }
    }

    fn start_move_note(&mut self) {
        // Find the currently selected note in the sidebar
        let idx = self.navigation_state.selected_entry_index;
        let source_path = match self.navigation_state.sidebar_items.get(idx) {
            Some(item) if item.section == SidebarSection::Notes => match &item.node_data {
                SidebarNodeData::Note(crate::tui::cache::NoteNode::Entry { path, .. }) => {
                    path.clone()
                }
                _ => return,
            },
            _ => return,
        };

        self.ensure_notes_loaded();
        let categories = self.config.notes.categories.clone();
        let mut destinations: Vec<String> = categories.clone();
        // Add existing subfolders as destinations
        for cat in &categories {
            for folder in self.notes_tree_state.folders_for_category(cat) {
                destinations.push(format!("{}/{}", cat, folder));
            }
        }
        destinations.sort();

        self.move_note_state = Some(MoveNoteState {
            source_path,
            destinations,
            selected_index: 0,
            filter: String::new(),
        });
        self.input_buffer.clear();
        self.current_view = ViewType::MoveNote;
    }

    fn confirm_move_note(&mut self) {
        let Some(state) = self.move_note_state.take() else {
            return;
        };
        let filtered = state.filtered_destinations();
        let Some((_, dest_str)) = filtered.get(state.selected_index) else {
            self.current_view = ViewType::TreeView;
            return;
        };
        let dest_str = dest_str.to_string();
        let parts: Vec<&str> = dest_str.splitn(2, '/').collect();
        let filename = state
            .source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("note.md")
            .to_string();
        let to = if parts.len() == 2 {
            self.config
                .workspace
                .notes_dir()
                .join(parts[0])
                .join(parts[1])
                .join(&filename)
        } else {
            self.config
                .workspace
                .notes_dir()
                .join(parts[0])
                .join(&filename)
        };
        if let Err(e) = self.config.workspace.move_note(&state.source_path, &to) {
            tracing::error!("Failed to move note: {}", e);
        }
        self.refresh_notes_cache();
        self.load_tree_view_data();
        self.current_view = ViewType::TreeView;
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
                        importance: None,
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
                    importance: t.importance.clone(),
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
            importance: task.importance.clone(),
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

    fn split_frontmatter_and_body(content: &str) -> Option<(&str, &str)> {
        let rest = content.strip_prefix("---\n")?;
        let end = rest.find("\n---\n")?;
        let frontmatter = &rest[..end];
        let body = &rest[end + "\n---\n".len()..];
        Some((frontmatter, body))
    }

    fn yaml_value_to_string(value: &serde_yaml::Value) -> String {
        match value {
            serde_yaml::Value::Null => String::new(),
            serde_yaml::Value::Bool(b) => b.to_string(),
            serde_yaml::Value::Number(n) => n.to_string(),
            serde_yaml::Value::String(s) => s.clone(),
            serde_yaml::Value::Sequence(_) | serde_yaml::Value::Mapping(_) => {
                serde_yaml::to_string(value)
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            }
            _ => String::new(),
        }
    }

    fn open_metadata_editor_for_selected_item(&mut self) -> bool {
        use crate::tui::navigation::SidebarSection;

        let Some(item) = self
            .navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)
        else {
            return false;
        };
        if item.section != SidebarSection::Programs || item.is_header {
            return false;
        }
        let Some(tree_path) = item.tree_path.clone() else {
            return false;
        };
        let Some(path) = item.path.clone() else {
            return false;
        };
        let template_name = match tree_path.len() {
            1 => "program",
            2 => "project",
            3 => "milestone",
            4 => "task",
            5 => "subtask",
            _ => return false,
        };

        let Ok(content) = self.config.workspace.read_md_file(&path) else {
            return false;
        };
        let Some((frontmatter, body)) = Self::split_frontmatter_and_body(&content) else {
            return false;
        };
        let mapping: serde_yaml::Mapping = serde_yaml::from_str(frontmatter).unwrap_or_default();

        self.open_template_wizard(template_name, Some(item.name.clone()));
        self.template_edit_target_path = Some(path);
        self.template_edit_selected_path = Some(tree_path);

        if let Some(ref mut state) = self.wizard_state.template {
            let description_text = if let Some((_, after)) = body.split_once("# Description") {
                after.trim().to_string()
            } else {
                String::new()
            };

            for field in &mut state.fields {
                let Some(placeholder) = field.placeholder.clone() else {
                    continue;
                };

                let mapped = match placeholder.as_str() {
                    "NAME" => mapping
                        .get(&serde_yaml::Value::String("title".to_string()))
                        .map(Self::yaml_value_to_string),
                    "DEFAULT_STATUS" => mapping
                        .get(&serde_yaml::Value::String("status".to_string()))
                        .map(Self::yaml_value_to_string),
                    "IMPORTANCE" => mapping
                        .get(&serde_yaml::Value::String("importance".to_string()))
                        .map(Self::yaml_value_to_string),
                    "TODAY" => mapping
                        .get(&serde_yaml::Value::String("creation_date".to_string()))
                        .map(Self::yaml_value_to_string),
                    "OWNER" => mapping
                        .get(&serde_yaml::Value::String("created_by".to_string()))
                        .map(Self::yaml_value_to_string),
                    "ASSIGNED_TO" => mapping
                        .get(&serde_yaml::Value::String("assigned_to".to_string()))
                        .map(Self::yaml_value_to_string),
                    "DUE_DATE" => mapping
                        .get(&serde_yaml::Value::String("due_date".to_string()))
                        .map(Self::yaml_value_to_string),
                    "UUID" => mapping
                        .get(&serde_yaml::Value::String("uuid".to_string()))
                        .map(Self::yaml_value_to_string),
                    "DESCRIPTION" => Some(description_text.clone()),
                    _ => mapping
                        .get(&serde_yaml::Value::String(field.key.clone()))
                        .map(Self::yaml_value_to_string),
                };

                if let Some(value) = mapped {
                    field.value = value.clone();
                    state.values.insert(placeholder, value);
                }
            }
        }

        true
    }

    fn open_template_wizard(&mut self, template_name: &str, seeded_name: Option<String>) {
        self.template_edit_target_path = None;
        self.template_edit_selected_path = None;
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
            "subtask" => include_str!("../../templates/subtask.md"),
            "journal" => include_str!("../../templates/journal.md"),
            "note" => include_str!("../../templates/note.md"),
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
                "subtask" => {
                    values.insert("SUBTASK_NAME".to_string(), name);
                }
                _ => {}
            }
        }

        if let Some(default_importance) = self.config.importance.first() {
            values.insert("IMPORTANCE".to_string(), default_importance.clone());
        }
        values.insert("UUID".to_string(), uuid::Uuid::new_v4().to_string());

        let strip_labels: std::collections::HashSet<String> = all_fields
            .iter()
            .filter(|f| f.strip_label)
            .filter_map(|f| f.placeholder.clone())
            .collect();

        let seeded_keywords = ["NAME", "TODAY", "OWNER", "UUID"];
        let base_keywords = ["TODAY", "OWNER", "UUID"];
        let keywords: &[&str] = if seeded_name.is_some() {
            &seeded_keywords
        } else {
            &base_keywords
        };

        let path_hint = match template_name {
            "program" => "Programs -> new program".to_string(),
            "project" => self
                .navigation_state
                .current_program
                .clone()
                .map(|p| format!("{p} -> new project"))
                .unwrap_or_else(|| "Programs -> new project".to_string()),
            "milestone" | "task" | "subtask" => {
                let mut parts = Vec::new();
                if let Some(ref p) = self.navigation_state.current_program {
                    parts.push(p.clone());
                }
                if let Some(ref p) = self.navigation_state.current_project {
                    parts.push(p.clone());
                }
                if matches!(template_name, "task" | "subtask")
                    && let Some(ref p) = self.navigation_state.current_milestone
                {
                    parts.push(p.clone());
                }
                if template_name == "subtask"
                    && let Some(ref p) = self.navigation_state.current_task
                {
                    parts.push(p.clone());
                }
                parts.push(format!("new {template_name}"));
                parts.join(" -> ")
            }
            "journal" => "Journal -> new entry".to_string(),
            "note" => {
                if let Some(ref state) = self.note_creation_state {
                    let cat = state
                        .categories
                        .get(state.selected_category_index)
                        .cloned()
                        .unwrap_or_default();
                    if state.folder_input.trim().is_empty() {
                        format!("Notes -> {cat} -> new note")
                    } else {
                        format!(
                            "Notes -> {cat} -> {} -> new note",
                            state.folder_input.trim()
                        )
                    }
                } else {
                    "Notes -> new note".to_string()
                }
            }
            _ => "new element".to_string(),
        };

        let fields: Vec<FieldInfo> = all_fields
            .into_iter()
            .enumerate()
            .map(|(i, def)| {
                let placeholder = def.placeholder.clone();
                let key = def.key.clone();
                let is_keyword = placeholder
                    .as_ref()
                    .is_some_and(|p| keywords.contains(&p.as_str()));
                let is_choice =
                    key.eq_ignore_ascii_case("status") || key.eq_ignore_ascii_case("importance");
                let choices = if key.eq_ignore_ascii_case("status") {
                    self.config.workflow.clone()
                } else if key.eq_ignore_ascii_case("importance") {
                    self.config.importance.clone()
                } else {
                    Vec::new()
                };

                let value = if let Some(ph) = placeholder.as_ref() {
                    values.get(ph).cloned().unwrap_or_default()
                } else {
                    def.fixed_value.clone().unwrap_or_default()
                };

                let is_editable = !is_keyword && (placeholder.is_some() || is_choice);
                let kind = if is_choice {
                    FieldKind::Choice
                } else if !is_editable && placeholder.is_some() {
                    FieldKind::AutoFilled
                } else if !is_editable {
                    FieldKind::Fixed
                } else if value.is_empty() {
                    FieldKind::Empty
                } else {
                    FieldKind::Suggested
                };
                FieldInfo {
                    key,
                    label: def.label,
                    placeholder,
                    value,
                    is_editable,
                    was_edited: false,
                    kind,
                    choices,
                    display_order: i,
                }
            })
            .collect();

        let initial_focus = fields
            .iter()
            .position(|f| f.is_editable)
            .map(WizardFocus::Field)
            .unwrap_or(WizardFocus::ConfirmButton);

        self.wizard_state.template = Some(TemplateFieldState {
            template_name: template_name.to_string(),
            path_hint,
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
            "subtask" => self
                .navigation_state
                .current_program
                .as_ref()
                .zip(self.navigation_state.current_project.as_ref())
                .zip(self.navigation_state.current_milestone.as_ref())
                .zip(self.navigation_state.current_task.as_ref())
                .and_then(|(((prog, proj), milestone), task)| {
                    name.map(|subtask_name| {
                        self.config
                            .workspace
                            .programs_dir()
                            .join(prog)
                            .join("projects")
                            .join(proj)
                            .join("milestones")
                            .join(milestone)
                            .join("tasks")
                            .join(task)
                            .join("subtasks")
                            .join(subtask_name)
                            .join(format!("{}.md", subtask_name))
                    })
                }),
            "journal" => Some(self.config.workspace.today_journal_path()),
            "note" => name.and_then(|note_name| {
                self.note_creation_state.as_ref().map(|state| {
                    let cat = state
                        .categories
                        .get(state.selected_category_index)
                        .cloned()
                        .unwrap_or_default();
                    let folder = state.folder_input.trim().to_string();
                    let notes_dir = self.config.workspace.notes_dir();
                    if folder.is_empty() {
                        notes_dir.join(&cat).join(format!("{}.md", note_name))
                    } else {
                        notes_dir
                            .join(&cat)
                            .join(&folder)
                            .join(format!("{}.md", note_name))
                    }
                })
            }),
            _ => None,
        }
    }

    fn confirm_template_field(&mut self) {
        if let Some(ref mut state) = self.wizard_state.template {
            match state.focus {
                WizardFocus::CancelButton => {
                    // Cancel - return to tree view without creating
                    self.wizard_state.template = None;
                    self.template_edit_target_path = None;
                    self.template_edit_selected_path = None;
                    self.note_creation_state = None;
                    self.current_view = ViewType::TreeView;
                }
                WizardFocus::ConfirmButton => {
                    // Confirm - collect all field values and create the element
                    // Save any current field value first
                    for field in &state.fields {
                        if let Some(placeholder) = field.placeholder.as_ref() {
                            if field.is_editable && field.value.trim().is_empty() {
                                continue;
                            }
                            state
                                .values
                                .insert(placeholder.clone(), field.value.clone());
                        }
                    }

                    let template_name = state.template_name.clone();
                    let values = state.values.clone();
                    let strip_labels = state.strip_labels.clone();
                    let name = values.get("NAME").cloned();
                    let candidate_name = name.as_deref();
                    let edit_target_path = self.template_edit_target_path.clone();
                    let edit_selected_path = self.template_edit_selected_path.clone();
                    let edit_mode = edit_target_path.is_some();

                    if !edit_mode
                        && let Some(candidate_name) = candidate_name
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

                    let target_path = if let Some(path) = edit_target_path {
                        Some(path)
                    } else {
                        self.resolve_template_target_path(&template_name, name.as_deref())
                    };

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
                    self.template_edit_target_path = None;
                    self.template_edit_selected_path = None;

                    // If this was a note, refresh notes cache and clear creation state
                    if template_name == "note" {
                        self.note_creation_state = None;
                        self.refresh_notes_cache();
                    }

                    // Refresh the tree view to show the newly created element at current level
                    self.load_tree_view_data();

                    // Find and select the newly created element in the sidebar
                    // Stay at parent level (don't auto-navigate into new element)
                    if edit_mode {
                        if let Some(path) = edit_selected_path {
                            self.set_selected_tree_path(path);
                            self.load_tree_view_data();
                        }
                    } else if let Some(ref element_name) = new_element_name {
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
                        field.was_edited = true;
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
            wizard.duration_edited = true;
        }
    }

    fn cycle_duration_right(&mut self) {
        // Use wizard state
        if let Some(ref mut wizard) = self.planning_wizard {
            wizard.cycle_duration(1);
            wizard.duration_edited = true;
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
                importance: t.importance.clone(),
                description: Some(t.description.clone()),
            },
            _ => {
                tracing::debug!(path = ?item.path, "open_task_detail_wizard: parsed element is not a Task");
                return;
            }
        };

        self.task_wizard = Some(task_wizard::TaskWizardState::with_task(task));
        self.task_wizard_tree_edit_mode = false;
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
        let tree_edit_mode = self.task_wizard_tree_edit_mode;
        let mut confirmed_task: Option<SelectedTask> = None;
        if let Some(ref mut wizard) = self.task_wizard {
            confirmed_task = task_wizard::confirm_task_detail_wizard(wizard);
        }
        let Some(task) = confirmed_task else {
            return;
        };

        // Update task/subtask file on disk with any changes.
        let task_path = task.path.clone();
        let updates = [
            ("status", Some(task.status.clone())),
            ("assigned_to", task.assigned_to.clone()),
            ("start_date", task.start_date.clone()),
            ("due_date", task.due_date.clone()),
            ("importance", task.importance.clone()),
        ];
        let updates_map: std::collections::HashMap<&str, Option<String>> =
            updates.iter().map(|(k, v)| (*k, v.clone())).collect();
        if let Err(e) = crate::storage::md::update_task_fields(&task_path, updates_map) {
            tracing::warn!("Failed to update task file: {}", e);
        }

        if tree_edit_mode {
            // Return to tree view after saving selected subtask metadata.
            let selected_path = self.navigation_state.sidebar_tree.selected_path_vec();
            self.task_wizard = None;
            self.task_wizard_tree_edit_mode = false;
            self.mode = Mode::Normal;
            self.current_view = ViewType::TreeView;
            if !selected_path.is_empty() {
                self.set_selected_tree_path(selected_path);
            }
            self.load_tree_view_data();
            return;
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

        // Return to picker
        self.mode = Mode::HierarchicalSelection;
        self.current_view = ViewType::HierarchicalTaskPicker;
        self.task_wizard_tree_edit_mode = false;
    }

    fn cancel_task_detail_wizard(&mut self) {
        // Clear the wizard state
        self.task_wizard = None;
        let tree_edit_mode = self.task_wizard_tree_edit_mode;
        self.task_wizard_tree_edit_mode = false;
        if tree_edit_mode {
            self.mode = Mode::Normal;
            self.current_view = ViewType::TreeView;
        } else {
            self.mode = Mode::HierarchicalSelection;
            self.current_view = ViewType::HierarchicalTaskPicker;
        }
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
