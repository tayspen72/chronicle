pub mod command;
pub mod layout;
pub mod navigation;
pub mod views;

use anyhow::Result;
use crossterm::{
    event::{self, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use std::io;

use crate::config::Config;

pub struct App {
    pub config: Config,
    pub current_view: ViewType,
    pub command_palette_open: bool,
    pub command_input: String,
    pub command_matches: Vec<CommandMatch>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    Programs,
    Projects,
    Milestones,
    Tasks,
    Journal,
    Backlog,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandMatch {
    pub label: String,
    pub view: ViewType,
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
        }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| self.draw(f))?;

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

            if self.command_palette_open {
                self.filter_commands();
            }
        }
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
                }
            }
            KeyCode::Up => {
                // Navigate up in current view
            }
            KeyCode::Down => {
                // Navigate down in current view
            }
            KeyCode::Enter => {
                // Select current item
            }
            _ => {}
        }
    }

    fn handle_command_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.command_input.push(c);
            }
            KeyCode::Backspace => {
                self.command_input.pop();
            }
            KeyCode::Esc => {
                self.command_palette_open = false;
                self.command_input.clear();
            }
            KeyCode::Enter => {
                // Execute selected command
                self.command_palette_open = false;
                self.command_input.clear();
            }
            KeyCode::Up => {
                // Navigate command list up
            }
            KeyCode::Down => {
                // Navigate command list down
            }
            _ => {}
        }
    }

    fn filter_commands(&mut self) {
        let input = self.command_input.to_lowercase();
        self.command_matches = get_command_list()
            .into_iter()
            .filter(|cmd| cmd.label.to_lowercase().contains(&input))
            .collect();
    }

    fn draw(&self, f: &mut Frame) {
        layout::render(f, self);
    }
}

fn get_command_list() -> Vec<CommandMatch> {
    vec![
        CommandMatch {
            label: "Go to Programs".to_string(),
            view: ViewType::Programs,
        },
        CommandMatch {
            label: "Go to Projects".to_string(),
            view: ViewType::Projects,
        },
        CommandMatch {
            label: "Go to Milestones".to_string(),
            view: ViewType::Milestones,
        },
        CommandMatch {
            label: "Go to Tasks".to_string(),
            view: ViewType::Tasks,
        },
        CommandMatch {
            label: "Go to Journal".to_string(),
            view: ViewType::Journal,
        },
        CommandMatch {
            label: "Go to Backlog".to_string(),
            view: ViewType::Backlog,
        },
        CommandMatch {
            label: "New Task".to_string(),
            view: ViewType::Tasks,
        },
        CommandMatch {
            label: "New Journal Entry".to_string(),
            view: ViewType::Journal,
        },
    ]
}
