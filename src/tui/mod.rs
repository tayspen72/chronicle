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
    ProgramsList,
    ProjectsList,
    MilestonesList,
    TasksList,
    Journal,
    JournalArchiveList,
    Backlog,
    SelectProgram,
    SelectProject,
    SelectMilestone,
    InputProgram,
    InputProject,
    InputMilestone,
    InputTask,
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
}

impl App {
    pub fn new(config: Config) -> Self {
        let command_matches = get_command_list();

        App {
            config,
            current_view: ViewType::Journal,
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
        }
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

    fn return_from_view(&mut self) {
        match self.current_view {
            ViewType::JournalArchiveList => {
                self.current_view = ViewType::Journal;
            }
            ViewType::ProgramsList => {
                self.current_view = ViewType::Journal;
            }
            ViewType::ProjectsList => {
                self.current_program = None;
                self.current_view = ViewType::ProgramsList;
            }
            ViewType::MilestonesList => {
                self.current_project = None;
                self.current_view = ViewType::ProjectsList;
            }
            ViewType::TasksList => {
                self.current_milestone = None;
                self.current_view = ViewType::MilestonesList;
            }
            ViewType::SelectProgram | ViewType::SelectProject | ViewType::SelectMilestone => {
                self.current_view = ViewType::Journal;
            }
            ViewType::InputProgram
            | ViewType::InputProject
            | ViewType::InputMilestone
            | ViewType::InputTask => {
                self.current_view = ViewType::Journal;
            }
            _ => {}
        }
    }

    fn navigate_up(&mut self) {
        match &self.current_view {
            ViewType::JournalArchiveList => {
                if self.selected_entry_index > 0 {
                    self.selected_entry_index -= 1;
                }
            }
            ViewType::ProgramsList | ViewType::SelectProgram => {
                if self.selected_entry_index > 0 {
                    self.selected_entry_index -= 1;
                }
            }
            ViewType::ProjectsList | ViewType::SelectProject => {
                if self.selected_entry_index > 0 {
                    self.selected_entry_index -= 1;
                }
            }
            ViewType::MilestonesList | ViewType::SelectMilestone => {
                if self.selected_entry_index > 0 {
                    self.selected_entry_index -= 1;
                }
            }
            ViewType::TasksList => {
                if self.selected_entry_index > 0 {
                    self.selected_entry_index -= 1;
                }
            }
            ViewType::InputProgram
            | ViewType::InputProject
            | ViewType::InputMilestone
            | ViewType::InputTask => {
                // No navigation in input mode
            }
            _ => {}
        }
    }

    fn navigate_down(&mut self) {
        match &self.current_view {
            ViewType::JournalArchiveList => {
                if self.selected_entry_index < self.journal_entries.len().saturating_sub(1) {
                    self.selected_entry_index += 1;
                }
            }
            ViewType::ProgramsList | ViewType::SelectProgram => {
                if self.selected_entry_index < self.programs.len().saturating_sub(1) {
                    self.selected_entry_index += 1;
                }
            }
            ViewType::ProjectsList | ViewType::SelectProject => {
                if self.selected_entry_index < self.projects.len().saturating_sub(1) {
                    self.selected_entry_index += 1;
                }
            }
            ViewType::MilestonesList | ViewType::SelectMilestone => {
                if self.selected_entry_index < self.milestones.len().saturating_sub(1) {
                    self.selected_entry_index += 1;
                }
            }
            ViewType::TasksList => {
                if self.selected_entry_index < self.tasks.len().saturating_sub(1) {
                    self.selected_entry_index += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_enter(&mut self) {
        match &self.current_view {
            ViewType::JournalArchiveList => {
                self.open_selected_archive_entry();
            }
            ViewType::ProgramsList => {
                self.select_program();
            }
            ViewType::ProjectsList => {
                self.select_project();
            }
            ViewType::MilestonesList => {
                self.select_milestone();
            }
            ViewType::TasksList => {
                self.open_selected_task();
            }
            ViewType::SelectProgram => {
                self.select_program_for_new_project();
            }
            ViewType::SelectProject => {
                self.select_project_for_new_milestone();
            }
            ViewType::SelectMilestone => {
                self.select_milestone_for_new_task();
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
        let data_path = &self.config.data_path;
        match data_path.list_programs() {
            Ok(entries) => {
                self.programs = entries;
                self.selected_entry_index = 0;
                self.current_view = ViewType::ProgramsList;
            }
            Err(e) => {
                eprintln!("Error loading programs: {}", e);
            }
        }
    }

    fn show_projects_list(&mut self) {
        if let Some(ref program) = self.current_program {
            let data_path = &self.config.data_path;
            match data_path.list_projects(program) {
                Ok(entries) => {
                    self.projects = entries;
                    self.selected_entry_index = 0;
                    self.current_view = ViewType::ProjectsList;
                }
                Err(e) => {
                    eprintln!("Error loading projects: {}", e);
                }
            }
        } else {
            eprintln!("No program selected. Use /programs first.");
        }
    }

    fn show_milestones_list(&mut self) {
        if let (Some(ref program), Some(ref project)) =
            (&self.current_program, &self.current_project)
        {
            let data_path = &self.config.data_path;
            match data_path.list_milestones(program, project) {
                Ok(entries) => {
                    self.milestones = entries;
                    self.selected_entry_index = 0;
                    self.current_view = ViewType::MilestonesList;
                }
                Err(e) => {
                    eprintln!("Error loading milestones: {}", e);
                }
            }
        } else {
            eprintln!("No project selected. Use /programs then /projects first.");
        }
    }

    fn show_tasks_list(&mut self) {
        if let (Some(ref program), Some(ref project), Some(ref milestone)) = (
            &self.current_program,
            &self.current_project,
            &self.current_milestone,
        ) {
            let data_path = &self.config.data_path;
            match data_path.list_tasks(program, project, milestone) {
                Ok(entries) => {
                    self.tasks = entries;
                    self.selected_entry_index = 0;
                    self.current_view = ViewType::TasksList;
                }
                Err(e) => {
                    eprintln!("Error loading tasks: {}", e);
                }
            }
        } else {
            eprintln!("No milestone selected. Use /programs, /projects, /milestones first.");
        }
    }

    fn start_new_program(&mut self) {
        self.input_buffer.clear();
        self.current_view = ViewType::InputProgram;
    }

    fn start_new_project(&mut self) {
        self.input_buffer.clear();
        self.show_programs_selection();
    }

    fn start_new_milestone(&mut self) {
        self.input_buffer.clear();
        self.show_projects_selection();
    }

    fn start_new_task(&mut self) {
        self.input_buffer.clear();
        self.show_milestones_selection();
    }

    fn show_programs_selection(&mut self) {
        let data_path = &self.config.data_path;
        match data_path.list_programs() {
            Ok(entries) => {
                self.programs = entries;
                self.selected_entry_index = 0;
                self.current_view = ViewType::SelectProgram;
            }
            Err(e) => {
                eprintln!("Error loading programs: {}", e);
            }
        }
    }

    fn show_projects_selection(&mut self) {
        if let Some(ref program) = self.current_program {
            let data_path = &self.config.data_path;
            match data_path.list_projects(program) {
                Ok(entries) => {
                    self.projects = entries;
                    self.selected_entry_index = 0;
                    self.current_view = ViewType::SelectProject;
                }
                Err(e) => {
                    eprintln!("Error loading projects: {}", e);
                }
            }
        }
    }

    fn show_milestones_selection(&mut self) {
        if let (Some(ref program), Some(ref project)) =
            (&self.current_program, &self.current_project)
        {
            let data_path = &self.config.data_path;
            match data_path.list_milestones(program, project) {
                Ok(entries) => {
                    self.milestones = entries;
                    self.selected_entry_index = 0;
                    self.current_view = ViewType::SelectMilestone;
                }
                Err(e) => {
                    eprintln!("Error loading milestones: {}", e);
                }
            }
        }
    }

    fn confirm_create_program(&mut self) {
        if !self.input_buffer.is_empty() {
            let data_path = &self.config.data_path;
            match data_path.create_program(&self.input_buffer) {
                Ok(_) => {
                    self.current_program = Some(self.input_buffer.clone());
                    self.show_programs_list();
                }
                Err(e) => {
                    eprintln!("Error creating program: {}", e);
                    self.current_view = ViewType::Journal;
                }
            }
        } else {
            self.current_view = ViewType::Journal;
        }
        self.input_buffer.clear();
    }

    fn confirm_create_project(&mut self) {
        if !self.input_buffer.is_empty() {
            if let Some(ref program) = self.current_program {
                let data_path = &self.config.data_path;
                match data_path.create_project(program, &self.input_buffer) {
                    Ok(_) => {
                        self.current_project = Some(self.input_buffer.clone());
                        self.show_projects_list();
                    }
                    Err(e) => {
                        eprintln!("Error creating project: {}", e);
                        self.current_view = ViewType::Journal;
                    }
                }
            }
        } else {
            self.current_view = ViewType::Journal;
        }
        self.input_buffer.clear();
    }

    fn confirm_create_milestone(&mut self) {
        if !self.input_buffer.is_empty() {
            if let (Some(ref program), Some(ref project)) =
                (&self.current_program, &self.current_project)
            {
                let data_path = &self.config.data_path;
                match data_path.create_milestone(program, project, &self.input_buffer) {
                    Ok(_) => {
                        self.current_milestone = Some(self.input_buffer.clone());
                        self.show_milestones_list();
                    }
                    Err(e) => {
                        eprintln!("Error creating milestone: {}", e);
                        self.current_view = ViewType::Journal;
                    }
                }
            }
        } else {
            self.current_view = ViewType::Journal;
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
                let data_path = &self.config.data_path;
                match data_path.create_task(program, project, milestone, &self.input_buffer) {
                    Ok(path) => {
                        self.launch_editor(&path);
                        self.show_tasks_list();
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
            self.command_matches = get_command_list()
                .into_iter()
                .filter(|cmd| cmd.label.to_lowercase().contains(&input))
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
            view: ViewType::ProgramsList,
            exit: false,
            action: Some(CommandAction::ShowProgramsList),
        },
        CommandMatch {
            label: "Projects".to_string(),
            view: ViewType::ProjectsList,
            exit: false,
            action: Some(CommandAction::ShowProjectsList),
        },
        CommandMatch {
            label: "Milestones".to_string(),
            view: ViewType::MilestonesList,
            exit: false,
            action: Some(CommandAction::ShowMilestonesList),
        },
        CommandMatch {
            label: "Tasks".to_string(),
            view: ViewType::TasksList,
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
            view: ViewType::SelectProgram,
            exit: false,
            action: Some(CommandAction::NewProject),
        },
        CommandMatch {
            label: "New Milestone".to_string(),
            view: ViewType::SelectProject,
            exit: false,
            action: Some(CommandAction::NewMilestone),
        },
        CommandMatch {
            label: "New Task".to_string(),
            view: ViewType::SelectMilestone,
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
