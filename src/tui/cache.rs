use crate::model::SelectedTask;
use crate::storage::{DirectoryEntry, JournalEntry};
use std::path::PathBuf;

pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Tree item for journal archive: (depth, label, is_header)
pub type TreeItem = (usize, String, bool);

/// Builds a year/month hierarchy tree from journal entries.
pub fn build_journal_tree(entries: &[JournalEntry]) -> Vec<TreeItem> {
    let mut by_year: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, Vec<usize>>,
    > = std::collections::BTreeMap::new();

    for (idx, entry) in entries.iter().enumerate() {
        let date_part = entry.filename.trim_end_matches(".md");
        let parts: Vec<&str> = date_part.split('-').collect();
        if parts.len() >= 2 {
            let year = parts[0].to_string();
            let month_num: usize = parts[1].parse().unwrap_or(1);
            let month_name = MONTH_NAMES
                .get(month_num.saturating_sub(1))
                .unwrap_or(&"Unknown")
                .to_string();
            by_year
                .entry(year)
                .or_default()
                .entry(month_name)
                .or_default()
                .push(idx);
        } else {
            by_year
                .entry("Unknown".to_string())
                .or_default()
                .entry("Unknown".to_string())
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
