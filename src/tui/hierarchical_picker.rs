use crate::storage::DirectoryEntry;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerLevel {
    Programs,
    Projects,
    Milestones,
    Tasks,
}

impl PickerLevel {
    pub fn parent(self) -> Option<Self> {
        match self {
            PickerLevel::Programs => None,
            PickerLevel::Projects => Some(PickerLevel::Programs),
            PickerLevel::Milestones => Some(PickerLevel::Projects),
            PickerLevel::Tasks => Some(PickerLevel::Milestones),
        }
    }

    pub fn child(self) -> Option<Self> {
        match self {
            PickerLevel::Programs => Some(PickerLevel::Projects),
            PickerLevel::Projects => Some(PickerLevel::Milestones),
            PickerLevel::Milestones => Some(PickerLevel::Tasks),
            PickerLevel::Tasks => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PickerItem {
    pub name: String,
    pub path: std::path::PathBuf,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct HierarchicalPickerState {
    pub level: PickerLevel,
    pub selected_program: Option<String>,
    pub selected_project: Option<String>,
    pub selected_milestone: Option<String>,
    pub selected_tasks: HashSet<String>,
    pub filter_text: String,
    pub items: Vec<PickerItem>,
    pub cursor_index: usize,
}

impl Default for HierarchicalPickerState {
    fn default() -> Self {
        Self {
            level: PickerLevel::Programs,
            selected_program: None,
            selected_project: None,
            selected_milestone: None,
            selected_tasks: HashSet::new(),
            filter_text: String::new(),
            items: Vec::new(),
            cursor_index: 0,
        }
    }
}

impl HierarchicalPickerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn breadcrumb(&self) -> String {
        let parts: Vec<_> = [
            self.selected_program.as_deref(),
            self.selected_project.as_deref(),
            self.selected_milestone.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();

        if parts.is_empty() {
            "Programs".to_string()
        } else {
            parts.join(" > ")
        }
    }

    pub fn level_title(&self) -> &'static str {
        match self.level {
            PickerLevel::Programs => "Programs",
            PickerLevel::Projects => "Projects",
            PickerLevel::Milestones => "Milestones",
            PickerLevel::Tasks => "Tasks",
        }
    }

    pub fn filtered_items(&self) -> Vec<&PickerItem> {
        if self.filter_text.is_empty() {
            return self.items.iter().collect();
        }
        let filter_lower = self.filter_text.to_lowercase();
        self.items
            .iter()
            .filter(|item| item.name.to_lowercase().contains(&filter_lower))
            .collect()
    }

    pub fn navigate_up(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index -= 1;
        }
    }

    pub fn navigate_down(&mut self) {
        let filtered = self.filtered_items();
        let max_idx = filtered.len().saturating_sub(1);
        if self.cursor_index < max_idx {
            self.cursor_index += 1;
        }
    }

    pub fn go_back(&mut self) -> bool {
        if let Some(parent_level) = self.level.parent() {
            self.level = parent_level;
            // Clear selections for levels we're navigating away from
            match parent_level {
                PickerLevel::Programs => {
                    self.selected_project = None;
                    self.selected_milestone = None;
                }
                PickerLevel::Projects => {
                    self.selected_milestone = None;
                }
                _ => {}
            }
            self.cursor_index = 0;
            self.filter_text.clear();
            true
        } else {
            false
        }
    }

    pub fn select_current(&mut self) -> Option<(PickerLevel, String)> {
        let filtered = self.filtered_items();
        let item = filtered.get(self.cursor_index)?;
        let name = item.name.clone();

        match self.level {
            PickerLevel::Programs => self.selected_program = Some(name.clone()),
            PickerLevel::Projects => self.selected_project = Some(name.clone()),
            PickerLevel::Milestones => self.selected_milestone = Some(name.clone()),
            PickerLevel::Tasks => {
                let path_str = item.path.to_string_lossy().to_string();
                if self.selected_tasks.contains(&path_str) {
                    self.selected_tasks.remove(&path_str);
                } else {
                    self.selected_tasks.insert(path_str);
                }
            }
        }

        // Navigate to child level for non-task selections
        if !matches!(self.level, PickerLevel::Tasks) {
            if let Some(child_level) = self.level.child() {
                self.level = child_level;
            }
        }

        self.cursor_index = 0;
        self.filter_text.clear();
        Some((self.level, name))
    }

    pub fn toggle_task_selection(&mut self) {
        let filtered = self.filtered_items();
        if let Some(item) = filtered.get(self.cursor_index) {
            let path_str = item.path.to_string_lossy().to_string();
            if self.selected_tasks.contains(&path_str) {
                self.selected_tasks.remove(&path_str);
            } else {
                self.selected_tasks.insert(path_str);
            }
        }
    }

    pub fn set_items(&mut self, entries: Vec<DirectoryEntry>) {
        self.items = entries
            .into_iter()
            .map(|e| PickerItem {
                name: e.name,
                path: e.path,
                is_dir: e.is_dir,
            })
            .collect();
        self.cursor_index = 0;
    }

    pub fn selected_count(&self) -> usize {
        self.selected_tasks.len()
    }
}
