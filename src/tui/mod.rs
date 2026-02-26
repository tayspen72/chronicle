pub mod command;
pub mod layout;
pub mod navigation;
pub mod views;

use std::process::Command;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use std::io::{self, Write};

use crate::config::Config;
use crate::storage::{DirectoryEntry, JournalEntry, JournalStorage, WorkspaceStorage};

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    TreeView,
    Journal,
    JournalArchiveList,
    Backlog,
    ViewingContent,
    InputProgram,
    InputProject,
    InputMilestone,
    InputTask,
}

#[derive(Debug, Clone, Default)]
pub struct TreeState {
    pub path: Vec<String>,
    pub expanded: Vec<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum CommandAction {
    OpenTodayJournal,
    ShowArchiveList,
    ShowProgramsList,
    ShowProjectsList,
    ShowMilestonesList,
    ShowTasksList,
    NewProgram,
    NewProject,
    NewMilestone,
    NewTask,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandMatch {
    pub label: String,
    pub view: ViewType,
    pub exit: bool,
    pub action: Option<CommandAction>,
}

pub struct App {
    pub config: Config,
    pub current_view: ViewType,
    pub tree_state: TreeState,
    pub command_palette_open: bool,
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
    pub programs: Vec<DirectoryEntry>,
    pub projects: Vec<DirectoryEntry>,
    pub milestones: Vec<DirectoryEntry>,
    pub tasks: Vec<DirectoryEntry>,
    pub input_buffer: String,
    pub selected_content: Option<DirectoryEntry>,
    pub current_content_text: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let command_matches = get_command_list();

        let mut app = App {
            config,
            current_view: ViewType::TreeView,
            tree_state: TreeState::default(),
            command_palette_open: false,
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
            programs: Vec::new(),
            projects: Vec::new(),
            milestones: Vec::new(),
            tasks: Vec::new(),
            input_buffer: String::new(),
            selected_content: None,
            current_content_text: None,
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
                        if self.command_palette_open {
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
                self.command_palette_open = true;
                self.command_input.clear();
                self.filter_commands();
            }
            KeyCode::Esc => {
                if self.command_palette_open {
                    self.command_palette_open = false;
                    self.command_input.clear();
                    self.command_selection_index = 0;
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
                self.navigate_up();
            }
            KeyCode::Down => {
                self.navigate_down();
            }
            KeyCode::Enter => {
                self.handle_enter();
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

    fn navigate_right(&mut self) {
        match self.current_view {
            ViewType::TreeView => {
                self.open_tree_item();
            }
            _ => {}
        }
    }

    fn navigate_left(&mut self) {
        match self.current_view {
            ViewType::TreeView => {
                if !self.tree_state.path.is_empty() {
                    self.tree_state.path.pop();
                    let new_depth = self.tree_state.path.len();
                    match new_depth {
                        0 => {
                            self.current_program = None;
                            self.current_project = None;
                            self.current_milestone = None;
                        }
                        1 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = None;
                            self.current_milestone = None;
                        }
                        2 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = Some(self.tree_state.path[1].clone());
                            self.current_milestone = None;
                        }
                        3 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = Some(self.tree_state.path[1].clone());
                            self.current_milestone = Some(self.tree_state.path[2].clone());
                        }
                        _ => {}
                    }
                    self.selected_entry_index = 0;
                    self.load_tree_view_data();
                }
            }
            _ => {}
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
                        }
                        1 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = None;
                            self.current_milestone = None;
                        }
                        2 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = Some(self.tree_state.path[1].clone());
                            self.current_milestone = None;
                        }
                        3 => {
                            self.current_program = Some(self.tree_state.path[0].clone());
                            self.current_project = Some(self.tree_state.path[1].clone());
                            self.current_milestone = Some(self.tree_state.path[2].clone());
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
        match &self.current_view {
            ViewType::TreeView
            | ViewType::JournalArchiveList
            | ViewType::InputProgram
            | ViewType::InputProject
            | ViewType::InputMilestone
            | ViewType::InputTask => {
                if self.selected_entry_index > 0 {
                    self.selected_entry_index -= 1;
                }
            }
            _ => {}
        }
    }

    fn navigate_down(&mut self) {
        let item_count = self.get_visible_item_count();
        match &self.current_view {
            ViewType::TreeView
            | ViewType::JournalArchiveList
            | ViewType::InputProgram
            | ViewType::InputProject
            | ViewType::InputMilestone
            | ViewType::InputTask => {
                if self.selected_entry_index < item_count.saturating_sub(1) {
                    self.selected_entry_index += 1;
                }
            }
            _ => {}
        }
    }

    fn get_visible_item_count(&self) -> usize {
        match &self.current_view {
            ViewType::TreeView => {
                let depth = self.tree_state.path.len();
                let mut count = 0;

                count += self.programs.len();

                if depth >= 1 && !self.projects.is_empty() {
                    count += self.projects.len();
                }

                if depth >= 2 && !self.milestones.is_empty() {
                    count += self.milestones.len();
                }

                if depth >= 3 && !self.tasks.is_empty() {
                    count += self.tasks.len();
                }

                count
            }
            ViewType::JournalArchiveList => self.journal_entries.len(),
            _ => 0,
        }
    }

    fn open_tree_item(&mut self) {
        let depth = self.tree_state.path.len();
        let idx = self.selected_entry_index;

        let entry_to_open: Option<DirectoryEntry>;

        if depth == 0 {
            if idx < self.programs.len() {
                entry_to_open = self.programs.get(idx).cloned();
            } else {
                entry_to_open = None;
            }
        } else if depth == 1 {
            let prog_count = self.programs.len();
            if idx < prog_count {
                entry_to_open = None;
            } else {
                let proj_idx = idx - prog_count;
                entry_to_open = self.projects.get(proj_idx).cloned();
            }
        } else if depth == 2 {
            let prog_count = self.programs.len();
            let proj_count = self.projects.len();
            if idx < prog_count {
                entry_to_open = None;
            } else if idx < prog_count + proj_count {
                entry_to_open = None;
            } else {
                let mile_idx = idx - prog_count - proj_count;
                entry_to_open = self.milestones.get(mile_idx).cloned();
            }
        } else if depth >= 3 {
            let prog_count = self.programs.len();
            let proj_count = self.projects.len();
            let mile_count = self.milestones.len();
            if idx < prog_count {
                entry_to_open = None;
            } else if idx < prog_count + proj_count {
                entry_to_open = None;
            } else if idx < prog_count + proj_count + mile_count {
                entry_to_open = None;
            } else {
                let task_idx = idx - prog_count - proj_count - mile_count;
                entry_to_open = self.tasks.get(task_idx).cloned();
            }
        } else {
            entry_to_open = None;
        }

        if let Some(entry) = entry_to_open {
            if entry.is_dir {
                self.tree_state.path.push(entry.name.clone());
                self.selected_entry_index = 0;
                self.load_tree_view_data();
            } else {
                self.open_content(&entry);
            }
        }
    }

    fn get_current_tree_items(&self) -> &Vec<DirectoryEntry> {
        match self.tree_state.path.len() {
            0 => &self.programs,
            1 => &self.projects,
            2 => &self.milestones,
            3 => &self.tasks,
            _ => &self.programs,
        }
    }

    fn open_content(&mut self, entry: &DirectoryEntry) {
        if let Ok(content) = self.config.data_path.read_md_file(&entry.path) {
            self.current_content_text = Some(content);
            self.current_view = ViewType::ViewingContent;
            self.selected_content = Some(entry.clone());
        }
    }

    fn load_tree_view_data(&mut self) {
        match self.tree_state.path.len() {
            0 => {
                match self.config.data_path.list_programs() {
                    Ok(entries) => self.programs = entries,
                    Err(_) => self.programs.clear(),
                }
                self.current_program = None;
                self.current_project = None;
                self.current_milestone = None;
            }
            1 => {
                let program = &self.tree_state.path[0];
                self.current_program = Some(program.clone());
                match self.config.data_path.list_projects(program) {
                    Ok(entries) => self.projects = entries,
                    Err(_) => self.projects.clear(),
                }
                self.current_project = None;
                self.current_milestone = None;
            }
            2 => {
                let program = &self.tree_state.path[0];
                let project = &self.tree_state.path[1];
                self.current_program = Some(program.clone());
                self.current_project = Some(project.clone());
                match self.config.data_path.list_milestones(program, project) {
                    Ok(entries) => self.milestones = entries,
                    Err(_) => self.milestones.clear(),
                }
                self.current_milestone = None;
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
                    .data_path
                    .list_tasks(program, project, milestone)
                {
                    Ok(entries) => self.tasks = entries,
                    Err(_) => self.tasks.clear(),
                }
            }
            _ => {}
        }
        self.selected_entry_index = 0;
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
            _ => {}
        }
    }

    fn select_program_for_new_project(&mut self) {
        if let Some(entry) = self.programs.get(self.selected_entry_index) {
            self.current_program = Some(entry.name.clone());
            self.input_buffer.clear();
            self.current_view = ViewType::InputProject;
        }
    }

    fn select_project_for_new_milestone(&mut self) {
        if let Some(entry) = self.projects.get(self.selected_entry_index) {
            self.current_project = Some(entry.name.clone());
            self.input_buffer.clear();
            self.current_view = ViewType::InputMilestone;
        }
    }

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
                self.command_palette_open = false;
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
                self.command_palette_open = false;
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
        let data_path = &self.config.data_path;
        match data_path.open_or_create_today_journal() {
            Ok((path, _)) => {
                self.launch_editor(&path);
            }
            Err(e) => {
                eprintln!("Error opening today's journal: {}", e);
            }
        }
    }

    fn show_archive_list(&mut self) {
        let data_path = &self.config.data_path;
        match data_path.list_journal_entries() {
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
        if !self.input_buffer.is_empty() {
            match self.config.data_path.create_program(&self.input_buffer) {
                Ok(_) => {
                    self.current_program = Some(self.input_buffer.clone());
                    self.tree_state.path.push(self.input_buffer.clone());
                    self.selected_entry_index = 0;
                    self.load_tree_view_data();
                    self.current_view = ViewType::TreeView;
                }
                Err(e) => {
                    eprintln!("Error creating program: {}", e);
                    self.current_view = ViewType::Journal;
                }
            }
        } else {
            self.current_view = ViewType::TreeView;
        }
        self.input_buffer.clear();
    }

    fn confirm_create_project(&mut self) {
        if !self.input_buffer.is_empty() {
            if let Some(ref program) = self.current_program {
                match self
                    .config
                    .data_path
                    .create_project(program, &self.input_buffer)
                {
                    Ok(_) => {
                        self.current_project = Some(self.input_buffer.clone());
                        self.tree_state.path.push(self.input_buffer.clone());
                        self.load_tree_view_data();
                        // Set index to first project (after all programs)
                        self.selected_entry_index = self.programs.len();
                        self.current_view = ViewType::TreeView;
                    }
                    Err(e) => {
                        eprintln!("Error creating project: {}", e);
                        self.current_view = ViewType::Journal;
                    }
                }
            }
        } else {
            self.current_view = ViewType::TreeView;
        }
        self.input_buffer.clear();
    }

    fn confirm_create_milestone(&mut self) {
        if !self.input_buffer.is_empty() {
            if let (Some(ref program), Some(ref project)) =
                (&self.current_program, &self.current_project)
            {
                match self
                    .config
                    .data_path
                    .create_milestone(program, project, &self.input_buffer)
                {
                    Ok(_) => {
                        self.current_milestone = Some(self.input_buffer.clone());
                        self.tree_state.path.push(self.input_buffer.clone());
                        self.load_tree_view_data();
                        // Set index to first milestone (after programs + projects)
                        self.selected_entry_index = self.programs.len() + self.projects.len();
                        self.current_view = ViewType::TreeView;
                    }
                    Err(e) => {
                        eprintln!("Error creating milestone: {}", e);
                        self.current_view = ViewType::Journal;
                    }
                }
            }
        } else {
            self.current_view = ViewType::TreeView;
        }
        self.input_buffer.clear();
    }

    fn confirm_create_task(&mut self) {
        if !self.input_buffer.is_empty() {
            if let (Some(ref program), Some(ref project), Some(ref milestone)) = (
                &self.current_program,
                &self.current_project,
                &self.current_milestone,
            ) {
                match self.config.data_path.create_task(
                    program,
                    project,
                    milestone,
                    &self.input_buffer,
                ) {
                    Ok(path) => {
                        self.load_tree_view_data();
                        // Set index to first task (after programs + projects + milestones)
                        self.selected_entry_index =
                            self.programs.len() + self.projects.len() + self.milestones.len();
                        self.launch_editor(&path);
                    }
                    Err(e) => {
                        eprintln!("Error creating task: {}", e);
                        self.current_view = ViewType::Journal;
                    }
                }
            }
        } else {
            self.current_view = ViewType::Journal;
        }
        self.input_buffer.clear();
    }

    fn select_program(&mut self) {
        if let Some(entry) = self.programs.get(self.selected_entry_index) {
            self.current_program = Some(entry.name.clone());
            self.current_project = None;
            self.current_milestone = None;
            self.show_projects_list();
        }
    }

    fn select_project(&mut self) {
        if let Some(entry) = self.projects.get(self.selected_entry_index) {
            self.current_project = Some(entry.name.clone());
            self.current_milestone = None;
            self.show_milestones_list();
        }
    }

    fn select_milestone(&mut self) {
        if let Some(entry) = self.milestones.get(self.selected_entry_index) {
            self.current_milestone = Some(entry.name.clone());
            self.show_tasks_list();
        }
    }

    fn open_selected_task(&mut self) {
        if let Some(entry) = self.tasks.get(self.selected_entry_index) {
            let path = entry.path.clone();
            self.launch_editor(&path);
        }
    }

    fn filter_commands(&mut self) {
        let input = self.command_input.to_lowercase();
        let depth = self.tree_state.path.len();

        if input.starts_with("journal") || input.starts_with("/journal") {
            let remainder = input
                .trim_start_matches('/')
                .trim_start_matches("journal")
                .trim();

            if remainder.is_empty() {
                self.command_matches = vec![
                    CommandMatch {
                        label: "Open Today's Journal".to_string(),
                        view: ViewType::Journal,
                        exit: false,
                        action: Some(CommandAction::OpenTodayJournal),
                    },
                    CommandMatch {
                        label: "Read Archived Journal Entries".to_string(),
                        view: ViewType::Journal,
                        exit: false,
                        action: Some(CommandAction::ShowArchiveList),
                    },
                ];
            } else {
                self.command_matches = vec![
                    CommandMatch {
                        label: "Open Today's Journal".to_string(),
                        view: ViewType::Journal,
                        exit: false,
                        action: Some(CommandAction::OpenTodayJournal),
                    },
                    CommandMatch {
                        label: "Read Archived Journal Entries".to_string(),
                        view: ViewType::Journal,
                        exit: false,
                        action: Some(CommandAction::ShowArchiveList),
                    },
                ]
                .into_iter()
                .filter(|cmd| cmd.label.to_lowercase().contains(remainder))
                .collect();
            }
        } else {
            let all_commands = get_command_list();
            self.command_matches = all_commands
                .into_iter()
                .filter(|cmd| {
                    let matches_input = cmd.label.to_lowercase().contains(&input);

                    if depth == 0 {
                        matches_input
                            && matches!(
                                cmd.label.as_str(),
                                "New Program"
                                    | "New Project"
                                    | "Programs"
                                    | "Journal"
                                    | "Backlog"
                                    | "Open Today's Journal"
                                    | "Read Archived Journal Entries"
                                    | "Exit"
                            )
                    } else if depth == 1 {
                        matches_input
                            && matches!(
                                cmd.label.as_str(),
                                "New Project"
                                    | "Programs"
                                    | "Projects"
                                    | "Journal"
                                    | "Backlog"
                                    | "Open Today's Journal"
                                    | "Read Archived Journal Entries"
                                    | "Exit"
                            )
                    } else if depth == 2 {
                        matches_input
                            && matches!(
                                cmd.label.as_str(),
                                "New Milestone"
                                    | "Programs"
                                    | "Projects"
                                    | "Milestones"
                                    | "Journal"
                                    | "Backlog"
                                    | "Open Today's Journal"
                                    | "Read Archived Journal Entries"
                                    | "Exit"
                            )
                    } else {
                        matches_input
                            && matches!(
                                cmd.label.as_str(),
                                "New Task"
                                    | "Programs"
                                    | "Projects"
                                    | "Milestones"
                                    | "Tasks"
                                    | "Journal"
                                    | "Backlog"
                                    | "Open Today's Journal"
                                    | "Read Archived Journal Entries"
                                    | "Exit"
                            )
                    }
                })
                .collect();
        }

        self.command_selection_index = 0;
    }

    fn draw(&self, f: &mut Frame) {
        layout::render(f, self);
    }
}

fn get_command_list() -> Vec<CommandMatch> {
    vec![
        CommandMatch {
            label: "Programs".to_string(),
            view: ViewType::TreeView,
            exit: false,
            action: Some(CommandAction::ShowProgramsList),
        },
        CommandMatch {
            label: "Projects".to_string(),
            view: ViewType::TreeView,
            exit: false,
            action: Some(CommandAction::ShowProjectsList),
        },
        CommandMatch {
            label: "Milestones".to_string(),
            view: ViewType::TreeView,
            exit: false,
            action: Some(CommandAction::ShowMilestonesList),
        },
        CommandMatch {
            label: "Tasks".to_string(),
            view: ViewType::TreeView,
            exit: false,
            action: Some(CommandAction::ShowTasksList),
        },
        CommandMatch {
            label: "Journal".to_string(),
            view: ViewType::Journal,
            exit: false,
            action: None,
        },
        CommandMatch {
            label: "Backlog".to_string(),
            view: ViewType::Backlog,
            exit: false,
            action: None,
        },
        CommandMatch {
            label: "New Program".to_string(),
            view: ViewType::InputProgram,
            exit: false,
            action: Some(CommandAction::NewProgram),
        },
        CommandMatch {
            label: "New Project".to_string(),
            view: ViewType::InputProject,
            exit: false,
            action: Some(CommandAction::NewProject),
        },
        CommandMatch {
            label: "New Milestone".to_string(),
            view: ViewType::InputMilestone,
            exit: false,
            action: Some(CommandAction::NewMilestone),
        },
        CommandMatch {
            label: "New Task".to_string(),
            view: ViewType::InputTask,
            exit: false,
            action: Some(CommandAction::NewTask),
        },
        CommandMatch {
            label: "Open Today's Journal".to_string(),
            view: ViewType::Journal,
            exit: false,
            action: Some(CommandAction::OpenTodayJournal),
        },
        CommandMatch {
            label: "Read Archived Journal Entries".to_string(),
            view: ViewType::Journal,
            exit: false,
            action: Some(CommandAction::ShowArchiveList),
        },
        CommandMatch {
            label: "Exit".to_string(),
            view: ViewType::Journal,
            exit: true,
            action: None,
        },
    ]
}
