//! Cache module for TUI.
//!
//! Provides tree building and data structures for the sidebar.

use crate::model::SelectedTask;
use crate::storage::{DirectoryEntry, JournalEntry};
use std::path::PathBuf;

/// 3-letter month abbreviations (lowercase) mapped to full names
pub const MONTH_ABBREVS: &[(&str, &str)] = &[
    ("jan", "January"),
    ("feb", "February"),
    ("mar", "March"),
    ("apr", "April"),
    ("may", "May"),
    ("jun", "June"),
    ("jul", "July"),
    ("aug", "August"),
    ("sep", "September"),
    ("oct", "October"),
    ("nov", "November"),
    ("dec", "December"),
];

pub fn month_abbrev_to_name(abbrev: &str) -> Option<&'static str> {
    MONTH_ABBREVS
        .iter()
        .find(|(a, _)| *a == abbrev.to_lowercase())
        .map(|(_, name)| *name)
}

pub fn month_name_to_abbrev(name: &str) -> Option<&'static str> {
    MONTH_ABBREVS
        .iter()
        .find(|(_, full_name)| *full_name == name)
        .map(|(abbrev, _)| *abbrev)
}

pub fn extract_year_from_path(path: &std::path::Path) -> Option<String> {
    let components: Vec<_> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str().map(String::from),
            _ => None,
        })
        .collect();

    if let Some(pos) = components.iter().position(|s| s == "history")
        && let Some(year) = components.get(pos + 1)
        && year.len() == 4
        && year.chars().all(|c| c.is_ascii_digit())
    {
        return Some(year.clone());
    }
    None
}

pub fn extract_year_month_from_path(path: &std::path::Path) -> Option<(String, String)> {
    let components: Vec<_> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str().map(String::from),
            _ => None,
        })
        .collect();

    if let Some(pos) = components.iter().position(|s| s == "history")
        && components.len() >= pos + 3
    {
        let year = components.get(pos + 1)?.clone();
        let month = components.get(pos + 2)?.clone();
        if year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()) {
            return Some((year, month));
        }
    }
    None
}

/// Tree item for journal archive: (depth, label, is_header)
pub type TreeItem = (usize, String, bool);

/// Builds a year/month hierarchy tree from journal entries.
pub fn build_journal_tree(entries: &[JournalEntry]) -> Vec<TreeItem> {
    let mut by_year: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, Vec<usize>>,
    > = std::collections::BTreeMap::new();

    for (idx, entry) in entries.iter().enumerate() {
        if let Some((year, month_abbrev)) = extract_year_month_from_path(&entry.path) {
            let month_name = month_abbrev_to_name(&month_abbrev)
                .unwrap_or("Unknown")
                .to_string();
            by_year
                .entry(year)
                .or_default()
                .entry(month_name)
                .or_default()
                .push(idx);
        }
    }

    let mut tree_items = Vec::new();
    for (year, by_month) in &by_year {
        tree_items.push((0, year.clone(), true));
        for (month, indices) in by_month {
            tree_items.push((1, month.clone(), true));
            for idx in indices {
                if let Some(entry) = entries.get(*idx) {
                    let label = entry.filename.trim_end_matches(".md").to_string();
                    tree_items.push((2, label, false));
                }
            }
        }
    }
    tree_items
}

#[derive(Debug, Clone, Default)]
pub struct TreeData {
    pub programs: Vec<DirectoryEntry>,
    pub projects: Vec<DirectoryEntry>,
    pub milestones: Vec<DirectoryEntry>,
    pub tasks: Vec<DirectoryEntry>,
    pub subtasks: Vec<DirectoryEntry>,
}

impl TreeData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.programs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.programs.len()
    }

    pub fn at_depth(&self, depth: usize) -> &Vec<DirectoryEntry> {
        match depth {
            0 => &self.programs,
            1 => &self.projects,
            2 => &self.milestones,
            3 => &self.tasks,
            4 => &self.subtasks,
            _ => &self.programs,
        }
    }

    pub fn at_depth_mut(&mut self, depth: usize) -> &mut Vec<DirectoryEntry> {
        match depth {
            0 => &mut self.programs,
            1 => &mut self.projects,
            2 => &mut self.milestones,
            3 => &mut self.tasks,
            4 => &mut self.subtasks,
            _ => &mut self.programs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskMetadata {
    pub uuid: String,
    pub path: PathBuf,
    pub program: String,
    pub project: String,
    pub milestone: String,
    pub task_name: String,
    pub status: String,
}

impl From<TaskMetadata> for SelectedTask {
    fn from(meta: TaskMetadata) -> Self {
        SelectedTask {
            uuid: meta.uuid,
            path: meta.path,
            program: meta.program,
            project: meta.project,
            milestone: meta.milestone,
            task_name: meta.task_name,
            status: meta.status,
            assigned_to: None,
            start_date: None,
            due_date: None,
            priority: None,
            description: None,
        }
    }
}
