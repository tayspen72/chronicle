use std::fs;

use chrono::Local;

use crate::Result;
use crate::storage::{JournalStorage, WorkspaceStorage};
use crate::tui::ViewType;
use crate::tui::cache::{JournalNode, canonical_month_token};
use crate::tui::navigation::{SidebarItem, SidebarSection};

use super::super::{App, JournalNavNode};

impl App {
    /// Expands the History item: loads journal entries if needed, expands the tree,
    /// reloads program data, rebuilds sidebar with programs + journal tree, and selects the first child.
    pub(crate) fn expand_history(&mut self) {
        self.ensure_journal_entries_loaded();

        self.journal_expand_path(&[]);
        self.load_tree_view_data();
        self.select_first_journal_child(&[]);
    }

    pub(crate) fn ensure_journal_entries_loaded(&mut self) {
        if self.journal_entries.is_empty() {
            self.refresh_journal_entries_cache();
        }
    }

    pub(crate) fn refresh_journal_entries_cache(&mut self) {
        match self.config.workspace.list_journal_entries() {
            Ok(entries) => {
                self.journal_tree_state.set_entries(entries.clone());
                self.journal_entries = entries;
            }
            Err(e) => {
                tracing::warn!("Failed to list journal entries: {}", e);
                self.journal_tree_state.set_entries(Vec::new());
                self.journal_entries.clear();
            }
        }
        let (years, months) = self.scan_journal_dirs();
        self.journal_tree_state.set_discovered_dirs(years, months);
    }

    pub(crate) fn add_journal_months_and_entries(&mut self, year: &str) {
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

            if self.journal_is_expanded(&month_path) {
                self.add_journal_entries_for_month(year, month);
            }
        }
    }

    pub(crate) fn add_journal_entries_for_month(&mut self, year: &str, month: &str) {
        self.add_journal_entries_with_indent(year, month, 3);
    }

    pub(crate) fn add_journal_entries_with_indent(
        &mut self,
        year: &str,
        month: &str,
        indent_level: usize,
    ) {
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

    pub(crate) fn expand_journal_item(&mut self, path: &[String]) {
        self.journal_expand_path(path);
        self.load_tree_view_data();
        self.select_first_journal_child(path);
    }

    pub(crate) fn collapse_journal_item(&mut self, path: &[String]) {
        self.journal_collapse_path(path);
    }

    pub(crate) fn journal_model_path(path: &[String]) -> Vec<String> {
        let mut model_path = Vec::with_capacity(path.len() + 1);
        model_path.push(Self::JOURNAL_EXPANSION_ROOT.to_string());
        model_path.extend(path.iter().cloned());
        model_path
    }

    pub(crate) fn journal_is_expanded(&self, path: &[String]) -> bool {
        self.navigation_state
            .is_expanded(&Self::journal_model_path(path))
    }

    pub(crate) fn journal_expand_path(&mut self, path: &[String]) {
        self.navigation_state
            .expand_path(&Self::journal_model_path(path));
    }

    pub(crate) fn journal_collapse_path(&mut self, path: &[String]) {
        self.navigation_state
            .collapse_path(&Self::journal_model_path(path));
    }

    pub(crate) fn collapse_all_journal(&mut self) {
        self.journal_collapse_path(&[]);
    }

    pub(crate) fn scan_journal_dirs(&self) -> (Vec<String>, Vec<(String, String)>) {
        use std::collections::BTreeSet;

        fn scan_root(
            root: &std::path::Path,
            years: &mut BTreeSet<String>,
            months: &mut BTreeSet<(String, String)>,
        ) {
            let Ok(year_entries) = fs::read_dir(root) else {
                return;
            };
            for year_entry in year_entries.flatten() {
                let year_path = year_entry.path();
                if !year_path.is_dir() {
                    continue;
                }
                let Some(year_name) = year_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                else {
                    continue;
                };
                if year_name.len() != 4 || !year_name.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                years.insert(year_name.clone());

                let Ok(month_entries) = fs::read_dir(&year_path) else {
                    continue;
                };
                for month_entry in month_entries.flatten() {
                    let month_path = month_entry.path();
                    if !month_path.is_dir() {
                        continue;
                    }
                    let Some(month_name) = month_path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if let Some(month_token) = canonical_month_token(month_name) {
                        months.insert((year_name.clone(), month_token));
                    }
                }
            }
        }

        let mut years = BTreeSet::new();
        let mut months = BTreeSet::new();
        let journal_dir = self.config.workspace.journal_dir();
        scan_root(&journal_dir, &mut years, &mut months);
        scan_root(&journal_dir.join("history"), &mut years, &mut months);

        (years.into_iter().collect(), months.into_iter().collect())
    }

    pub(crate) fn selected_journal_node(&self) -> Option<JournalNavNode> {
        let item = self
            .navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)?;
        if item.section != SidebarSection::Journal {
            return None;
        }

        if let Some(action) = item.is_journal_item.as_deref() {
            return Some(match action {
                "Today" => JournalNavNode::Today,
                "History" => JournalNavNode::History,
                _ => JournalNavNode::OtherAction,
            });
        }

        item.journal_path
            .as_ref()
            .map(|path| {
                if item.is_journal_header {
                    JournalNavNode::Header(path.clone())
                } else {
                    JournalNavNode::Entry(path.clone())
                }
            })
            .or(Some(JournalNavNode::OtherAction))
    }

    pub(crate) fn select_first_journal_child(&mut self, parent_path: &[String]) {
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

    pub(crate) fn select_journal_item_by_path(&mut self, path: &[String]) {
        if let Some(idx) = self
            .navigation_state
            .sidebar_items
            .iter()
            .position(|item| item.journal_path.as_ref() == Some(&path.to_vec()))
        {
            self.navigation_state.selected_entry_index = idx;
        }
    }

    pub(crate) fn select_history(&mut self) {
        self.navigation_state.selected_entry_index = self
            .navigation_state
            .sidebar_items
            .iter()
            .position(|i| i.name == "History" && i.is_journal_item.is_some())
            .unwrap_or(0);
    }

    pub(crate) fn add_journal_tree_items(&mut self) {
        let years = self.journal_tree_state.years();

        for year in &years {
            let year_path = vec![year.clone()];
            let node = JournalNode::Year { year: year.clone() };
            let mut item = SidebarItem::journal(node, year_path.clone());
            item.indent = 1;
            self.navigation_state.sidebar_items.push(item);

            if self.journal_is_expanded(&year_path) {
                self.add_journal_months_and_entries(year);
            }
        }
    }

    pub(crate) fn open_today_journal(&mut self) {
        match self.create_today_journal_from_template() {
            Ok(path) => self.launch_editor(&path),
            Err(e) => eprintln!("Error opening today's journal: {}", e),
        }
    }

    pub(crate) fn create_today_journal_from_template(&self) -> Result<std::path::PathBuf> {
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

    pub(crate) fn show_archive_list(&mut self) {
        self.refresh_journal_entries_cache();
        self.navigation_state.selected_entry_index = 0;
        self.current_view = ViewType::JournalArchiveList;
    }

    pub(crate) fn open_archive_entry(&mut self, tree_index: usize) {
        if let Some(Some(entry_idx)) = self.archive_tree_mapping.get(tree_index)
            && let Some(entry) = self.journal_entries.get(*entry_idx)
        {
            let path = entry.path.clone();
            self.launch_editor(&path);
        }
    }

    pub(crate) fn open_selected_archive_entry(&mut self) {
        self.open_archive_entry(self.navigation_state.selected_entry_index);
    }
}
