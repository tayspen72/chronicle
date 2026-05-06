use std::fs;

use crate::storage::NotesStorage;
use crate::tui::cache::NoteNode;
use crate::tui::navigation::{SidebarItem, SidebarNodeData, SidebarSection};

use super::super::App;

impl App {
    pub(crate) fn notes_model_path(path: &[String]) -> Vec<String> {
        let mut model_path = Vec::with_capacity(path.len() + 1);
        model_path.push(Self::NOTES_EXPANSION_ROOT.to_string());
        model_path.extend(path.iter().cloned());
        model_path
    }

    pub(crate) fn notes_is_expanded(&self, path: &[String]) -> bool {
        self.navigation_state
            .is_expanded(&Self::notes_model_path(path))
    }

    pub(crate) fn notes_expand_path(&mut self, path: &[String]) {
        self.navigation_state
            .expand_path(&Self::notes_model_path(path));
    }

    pub(crate) fn notes_collapse_path(&mut self, path: &[String]) {
        self.navigation_state
            .collapse_path(&Self::notes_model_path(path));
    }

    pub(crate) fn collapse_all_notes(&mut self) {
        let paths_to_remove: Vec<Vec<String>> = self
            .navigation_state
            .sidebar_tree
            .expanded_paths()
            .iter()
            .filter(|p| p.first().map(|s| s.as_str()) == Some(Self::NOTES_EXPANSION_ROOT))
            .cloned()
            .collect();
        for path in paths_to_remove {
            self.navigation_state.sidebar_tree.collapse_path(&path);
        }
    }

    pub(crate) fn ensure_notes_loaded(&mut self) {
        if self.notes_tree_state.entries().is_empty() {
            self.refresh_notes_cache();
        }
    }

    pub(crate) fn refresh_notes_cache(&mut self) {
        if let Ok(entries) = self.config.workspace.scan_notes() {
            self.notes_tree_state.set_entries(entries);
        }
        self.notes_tree_state
            .set_discovered_folders(self.scan_notes_folders());
    }

    pub(crate) fn scan_notes_folders(&self) -> Vec<(String, String)> {
        let notes_dir = self.config.workspace.notes_dir();
        let mut folders = Vec::new();
        let Ok(categories) = fs::read_dir(&notes_dir) else {
            return folders;
        };

        for category_entry in categories.flatten() {
            let category_path = category_entry.path();
            if !category_path.is_dir() {
                continue;
            }
            let Some(category_name) = category_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
            else {
                continue;
            };

            let Ok(children) = fs::read_dir(&category_path) else {
                continue;
            };
            for child in children.flatten() {
                let child_path = child.path();
                if !child_path.is_dir() {
                    continue;
                }
                let Some(folder_name) = child_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                else {
                    continue;
                };
                folders.push((category_name.clone(), folder_name));
            }
        }

        folders
    }

    pub(crate) fn add_notes_tree_items(&mut self) {
        let categories = self
            .notes_tree_state
            .categories(&self.config.notes.categories);

        for category in &categories {
            let cat_path = vec![category.clone()];
            let node = NoteNode::Category {
                name: category.clone(),
            };
            let mut item = SidebarItem::note(node);
            item.indent = 0;
            self.navigation_state.sidebar_items.push(item);

            if self.notes_is_expanded(&cat_path) {
                self.add_notes_children_for_category(category);
            }
        }
    }

    pub(crate) fn add_notes_children_for_category(&mut self, category: &str) {
        // Direct notes (no subfolder) at indent 1
        let direct = self.notes_tree_state.entries_in_category(category);
        for entry in &direct {
            let node = NoteNode::from_entry(entry);
            let mut item = SidebarItem::note(node);
            item.indent = 1;
            self.navigation_state.sidebar_items.push(item);
        }

        // Subfolders at indent 1; their entries at indent 2
        let folders = self.notes_tree_state.folders_for_category(category);
        for folder in &folders {
            let folder_path = vec![category.to_string(), folder.clone()];
            let node = NoteNode::Folder {
                category: category.to_string(),
                name: folder.clone(),
            };
            let mut item = SidebarItem::note(node);
            item.indent = 1;
            self.navigation_state.sidebar_items.push(item);

            if self.notes_is_expanded(&folder_path) {
                let folder_entries = self.notes_tree_state.entries_in_folder(category, folder);
                for entry in &folder_entries {
                    let node = NoteNode::from_entry(entry);
                    let mut item = SidebarItem::note(node);
                    item.indent = 2;
                    self.navigation_state.sidebar_items.push(item);
                }
            }
        }
    }

    pub(crate) fn select_first_notes_child(&mut self, parent_path: &[String]) {
        let child_depth = parent_path.len() + 1;
        if let Some(idx) = self.navigation_state.sidebar_items.iter().position(|item| {
            item.section == SidebarSection::Notes
                && !item.is_header
                && item
                    .tree_path
                    .as_ref()
                    .is_some_and(|p| p.len() == child_depth && p.starts_with(parent_path))
        }) {
            self.navigation_state.selected_entry_index = idx;
        }
    }

    pub(crate) fn select_notes_item_by_path(&mut self, path: &[String]) {
        if let Some(idx) = self.navigation_state.sidebar_items.iter().position(|item| {
            item.section == SidebarSection::Notes && item.tree_path.as_deref() == Some(path)
        }) {
            self.navigation_state.selected_entry_index = idx;
        }
    }

    pub(crate) fn selected_note_node(&self) -> Option<SidebarNodeData> {
        let item = self
            .navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)?;
        if item.section != SidebarSection::Notes {
            return None;
        }
        Some(item.node_data.clone())
    }

    pub(crate) fn selected_note_tree_path(&self) -> Option<Vec<String>> {
        let item = self
            .navigation_state
            .sidebar_items
            .get(self.navigation_state.selected_entry_index)?;
        if item.section != SidebarSection::Notes {
            return None;
        }
        item.tree_path.clone()
    }
}
