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
    pub items: Vec<PickerItem>,
    pub cursor_index: usize,
    pub is_wizard_mode: bool,
}

impl Default for HierarchicalPickerState {
    fn default() -> Self {
        Self {
            level: PickerLevel::Programs,
            selected_program: None,
            selected_project: None,
            selected_milestone: None,
            selected_tasks: HashSet::new(),
            items: Vec::new(),
            cursor_index: 0,
            is_wizard_mode: false,
        }
    }
}

impl HierarchicalPickerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_wizard() -> Self {
        Self {
            level: PickerLevel::Programs,
            selected_program: None,
            selected_project: None,
            selected_milestone: None,
            selected_tasks: HashSet::new(),
            items: Vec::new(),
            cursor_index: 0,
            is_wizard_mode: true,
        }
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

    pub fn navigate_up(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index -= 1;
        }
    }

    pub fn navigate_down(&mut self) {
        let max_idx = self.items.len().saturating_sub(1);
        if self.cursor_index < max_idx {
            self.cursor_index += 1;
        }
    }

    pub fn go_back(&mut self) -> bool {
        if let Some(parent_level) = self.level.parent() {
            self.level = parent_level;
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
            true
        } else {
            false
        }
    }

    pub fn select_current(&mut self) -> Option<(PickerLevel, String)> {
        let item = self.items.get(self.cursor_index)?;
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

        if !matches!(self.level, PickerLevel::Tasks)
            && let Some(child_level) = self.level.child()
        {
            self.level = child_level;
        }

        self.cursor_index = 0;
        Some((self.level, name))
    }

    pub fn toggle_task_selection(&mut self) {
        if let Some(item) = self.items.get(self.cursor_index) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_updates_cursor() {
        let mut picker = HierarchicalPickerState::new();
        picker.items = vec![
            PickerItem {
                name: "A".to_string(),
                path: std::path::PathBuf::from("/a"),
                is_dir: true,
            },
            PickerItem {
                name: "B".to_string(),
                path: std::path::PathBuf::from("/b"),
                is_dir: true,
            },
            PickerItem {
                name: "C".to_string(),
                path: std::path::PathBuf::from("/c"),
                is_dir: true,
            },
        ];

        assert_eq!(picker.cursor_index, 0);

        picker.navigate_down();
        assert_eq!(picker.cursor_index, 1);

        picker.navigate_down();
        assert_eq!(picker.cursor_index, 2);

        picker.navigate_down();
        assert_eq!(picker.cursor_index, 2);

        picker.navigate_up();
        assert_eq!(picker.cursor_index, 1);

        picker.navigate_up();
        assert_eq!(picker.cursor_index, 0);

        picker.navigate_up();
        assert_eq!(picker.cursor_index, 0);
    }

    #[test]
    fn test_select_current_navigates_levels() {
        let mut picker = HierarchicalPickerState::new();
        picker.items = vec![PickerItem {
            name: "TestProgram".to_string(),
            path: std::path::PathBuf::from("/test"),
            is_dir: true,
        }];
        picker.level = PickerLevel::Programs;

        let result = picker.select_current();
        assert!(result.is_some());
        let (new_level, name) = result.unwrap();
        assert_eq!(new_level, PickerLevel::Projects);
        assert_eq!(name, "TestProgram");
        assert_eq!(picker.selected_program, Some("TestProgram".to_string()));
        assert_eq!(picker.level, PickerLevel::Projects);
    }

    #[test]
    fn test_go_back_navigates_hierarchy() {
        let mut picker = HierarchicalPickerState::new();
        picker.level = PickerLevel::Tasks;
        picker.selected_program = Some("Prog".to_string());
        picker.selected_project = Some("Proj".to_string());
        picker.selected_milestone = Some("Mile".to_string());

        let went_back = picker.go_back();
        assert!(went_back);
        assert_eq!(picker.level, PickerLevel::Milestones);
        assert!(picker.selected_milestone.is_some());

        let went_back = picker.go_back();
        assert!(went_back);
        assert_eq!(picker.level, PickerLevel::Projects);
        assert!(picker.selected_milestone.is_none());

        let went_back = picker.go_back();
        assert!(went_back);
        assert_eq!(picker.level, PickerLevel::Programs);
        assert!(picker.selected_project.is_none());

        let went_back = picker.go_back();
        assert!(!went_back);
    }
}
