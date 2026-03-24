//! Cache module for TUI.
//!
//! Provides tree building and data structures for the sidebar.

use crate::model::SelectedTask;
use crate::storage::{DirectoryEntry, JournalEntry};
use std::path::PathBuf;

/// Maps month tokens (abbrev/full) to numeric month strings.
const MONTH_TOKENS: &[(&str, &str)] = &[
    ("jan", "01"),
    ("january", "01"),
    ("feb", "02"),
    ("february", "02"),
    ("mar", "03"),
    ("march", "03"),
    ("apr", "04"),
    ("april", "04"),
    ("may", "05"),
    ("jun", "06"),
    ("june", "06"),
    ("jul", "07"),
    ("july", "07"),
    ("aug", "08"),
    ("august", "08"),
    ("sep", "09"),
    ("sept", "09"),
    ("september", "09"),
    ("oct", "10"),
    ("october", "10"),
    ("nov", "11"),
    ("november", "11"),
    ("dec", "12"),
    ("december", "12"),
];

pub fn canonical_month_token(month: &str) -> Option<String> {
    let trimmed = month.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(numeric) = trimmed.parse::<u8>()
        && (1..=12).contains(&numeric)
    {
        return Some(format!("{:02}", numeric));
    }

    let lower = trimmed.to_ascii_lowercase();
    MONTH_TOKENS
        .iter()
        .find(|(token, _)| *token == lower)
        .map(|(_, num)| (*num).to_string())
}

pub fn extract_year_from_path(path: &std::path::Path) -> Option<String> {
    let components: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str().map(String::from),
            _ => None,
        })
        .collect();

    let rel = journal_relative_components(&components)?;
    let year = rel.first()?;
    (year.len() == 4 && year.chars().all(|c| c.is_ascii_digit())).then(|| year.clone())
}

pub fn extract_year_month_from_path(path: &std::path::Path) -> Option<(String, String)> {
    let components: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str().map(String::from),
            _ => None,
        })
        .collect();

    let rel = journal_relative_components(&components)?;
    if rel.len() < 2 {
        return None;
    }
    let year = rel.first()?.clone();
    if year.len() != 4 || !year.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let month = canonical_month_token(rel.get(1)?)?;
    Some((year, month))
}

fn journal_relative_components(components: &[String]) -> Option<Vec<String>> {
    let journal_idx = components.iter().position(|s| s == "journal")?;
    let mut rel = components.get(journal_idx + 1..)?.to_vec();
    if rel.first().is_some_and(|s| s == "history") {
        rel.remove(0);
    }
    Some(rel)
}

/// Represents a node in the journal history tree.
#[derive(Debug, Clone)]
pub enum JournalNode {
    History,
    Year {
        year: String,
    },
    Month {
        year: String,
        month: String,
    },
    Entry {
        year: String,
        month: String,
        entry: JournalEntry,
    },
}

impl JournalNode {
    pub fn path_components(&self) -> Vec<String> {
        match self {
            JournalNode::History => vec![],
            JournalNode::Year { year } => vec![year.clone()],
            JournalNode::Month { year, month } => vec![year.clone(), month.clone()],
            JournalNode::Entry { year, month, entry } => vec![
                year.clone(),
                month.clone(),
                entry.filename.trim_end_matches(".md").to_string(),
            ],
        }
    }

    pub fn is_header(&self) -> bool {
        matches!(
            self,
            JournalNode::History | JournalNode::Year { .. } | JournalNode::Month { .. }
        )
    }

    pub fn label(&self) -> &str {
        match self {
            JournalNode::History => "History",
            JournalNode::Year { year } => year,
            JournalNode::Month { month, .. } => month,
            JournalNode::Entry { entry, .. } => entry.filename.trim_end_matches(".md"),
        }
    }

    pub fn from_entry(entry: &JournalEntry) -> Option<Self> {
        let (year, month) = extract_year_month_from_path(&entry.path)?;
        Some(JournalNode::Entry {
            year,
            month,
            entry: entry.clone(),
        })
    }
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
        if let Some((year, month)) = extract_year_month_from_path(&entry.path) {
            by_year
                .entry(year)
                .or_default()
                .entry(month)
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
            importance: None,
            description: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_node_history() {
        let node = JournalNode::History;
        assert!(node.path_components().is_empty());
        assert!(node.is_header());
        assert_eq!(node.label(), "History");
    }

    #[test]
    fn test_journal_node_year() {
        let node = JournalNode::Year {
            year: "2026".to_string(),
        };
        assert_eq!(node.path_components(), vec!["2026"]);
        assert!(node.is_header());
        assert_eq!(node.label(), "2026");
    }

    #[test]
    fn test_journal_node_month() {
        let node = JournalNode::Month {
            year: "2026".to_string(),
            month: "03".to_string(),
        };
        assert_eq!(node.path_components(), vec!["2026", "03"]);
        assert!(node.is_header());
        assert_eq!(node.label(), "03");
    }

    #[test]
    fn test_journal_node_entry() {
        let entry = JournalEntry {
            filename: "2026-03-15.md".to_string(),
            path: std::path::PathBuf::from("journal/2026/03/2026-03-15.md"),
        };
        let node = JournalNode::Entry {
            year: "2026".to_string(),
            month: "03".to_string(),
            entry,
        };
        assert_eq!(node.path_components(), vec!["2026", "03", "2026-03-15"]);
        assert!(!node.is_header());
        assert_eq!(node.label(), "2026-03-15");
    }

    #[test]
    fn test_journal_node_from_entry() {
        let entry = JournalEntry {
            filename: "2026-03-15.md".to_string(),
            path: std::path::PathBuf::from("journal/2026/03/2026-03-15.md"),
        };
        let node = JournalNode::from_entry(&entry);
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.label(), "2026-03-15");
        assert!(!node.is_header());
    }

    #[test]
    fn test_journal_node_from_entry_invalid_path() {
        let entry = JournalEntry {
            filename: "2026-03-15.md".to_string(),
            path: std::path::PathBuf::from("invalid/path.md"),
        };
        let node = JournalNode::from_entry(&entry);
        assert!(node.is_none());
    }
}
