//! Test utilities for TUI integration tests.

use crate::config::Config;
use crate::tui::App;
use std::fs;
use tempfile::TempDir;

pub fn app_with_empty_workspace() -> (TempDir, App) {
    let temp = TempDir::new().unwrap();
    let config = Config {
        workspace: temp.path().to_path_buf(),
        ..Default::default()
    };
    (temp, App::new(config))
}

pub fn app_with_hierarchy() -> (TempDir, App) {
    let temp = TempDir::new().unwrap();
    let workspace = temp.path();

    fs::create_dir_all(
        workspace.join("programs/TestProgram/projects/TestProject/milestones/TestMilestone/tasks"),
    )
    .unwrap();
    fs::write(
        workspace.join(
            "programs/TestProgram/projects/TestProject/milestones/TestMilestone/tasks/TestTask.md",
        ),
        "# Test Task\nstatus: todo\n",
    )
    .unwrap();

    let config = Config {
        workspace: workspace.to_path_buf(),
        ..Default::default()
    };
    (temp, App::new(config))
}

pub fn app_with_journal_entries() -> (TempDir, App) {
    let temp = TempDir::new().unwrap();
    let workspace = temp.path();

    fs::create_dir_all(workspace.join("journal/2026/02")).unwrap();
    fs::create_dir_all(workspace.join("journal/2026/03")).unwrap();
    fs::write(
        workspace.join("journal/2026/03/2026-03-15.md"),
        "# Journal 2026-03-15\n",
    )
    .unwrap();
    fs::write(
        workspace.join("journal/2026/03/2026-03-14.md"),
        "# Journal 2026-03-14\n",
    )
    .unwrap();
    fs::write(
        workspace.join("journal/2026/02/2026-02-28.md"),
        "# Journal 2026-02-28\n",
    )
    .unwrap();

    let config = Config {
        workspace: workspace.to_path_buf(),
        ..Default::default()
    };
    (temp, App::new(config))
}

pub fn press_keys(app: &mut App, keys: impl IntoIterator<Item = crossterm::event::KeyCode>) {
    for key in keys {
        app.handle_key(key);
    }
}

pub mod keys {
    pub use crossterm::event::KeyCode::*;

    pub fn type_str(s: &str) -> Vec<crossterm::event::KeyCode> {
        s.chars().map(Char).collect()
    }

    pub fn down(n: usize) -> Vec<crossterm::event::KeyCode> {
        std::iter::repeat_n(Down, n).collect()
    }

    pub fn up(n: usize) -> Vec<crossterm::event::KeyCode> {
        std::iter::repeat_n(Up, n).collect()
    }
}

pub mod assert {
    use crate::tui::{App, Mode, ViewType};

    pub fn mode_is(app: &App, expected: Mode) {
        assert_eq!(
            app.mode, expected,
            "Expected mode {:?}, got {:?}",
            expected, app.mode
        );
    }

    pub fn view_is(app: &App, expected: ViewType) {
        assert_eq!(
            app.current_view, expected,
            "Expected view {:?}, got {:?}",
            expected, app.current_view
        );
    }

    pub fn sidebar_selected(app: &App, expected_name: &str) {
        let idx = app.navigation_state.selected_entry_index;
        let item = &app.navigation_state.sidebar_items[idx];
        assert_eq!(
            item.name, expected_name,
            "Expected sidebar '{}', got '{}'",
            expected_name, item.name
        );
    }

    pub fn file_not_exists_at(workspace: &std::path::Path, relative: &str) {
        let path = workspace.join(relative);
        assert!(!path.exists(), "File should not exist: {}", path.display());
    }

    pub fn file_exists_at(workspace: &std::path::Path, relative: &str) {
        let path = workspace.join(relative);
        assert!(path.exists(), "File should exist: {}", path.display());
    }
}
