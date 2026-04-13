//! Navigation module - sidebar state and tree traversal.

use crate::storage::{DirectoryEntry, NoteEntry};
use crate::tui::cache::{
    JournalNode, NoteNode, extract_year_from_path, extract_year_month_from_path,
};
use crate::tui::sidebar_tree::SidebarTreeModel;

#[derive(Debug, Clone, Default)]
pub enum SidebarNodeData {
    Program(DirectoryEntry),
    Journal(JournalNode),
    Note(NoteNode),
    Planning,
    JournalAction,
    Action,
    #[default]
    Empty,
}

/// State for navigation (tree selection, sidebar, current scope).
#[derive(Debug, Clone, Default)]
pub struct NavigationState {
    pub sidebar_tree: SidebarTreeModel<()>,
    pub selected_entry_index: usize,
    pub sidebar_items: Vec<SidebarItem>,
    pub current_program: Option<String>,
    pub current_project: Option<String>,
    pub current_milestone: Option<String>,
    pub current_task: Option<String>,
}

impl NavigationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn selected_path(&self) -> &[String] {
        self.sidebar_tree.selected_path()
    }

    pub fn selected_depth(&self) -> usize {
        self.sidebar_tree.selected_depth()
    }

    pub fn set_selected_path(&mut self, path: Vec<String>) {
        self.sidebar_tree.set_selected_path(path);
    }

    pub fn expand_path(&mut self, path: &[String]) {
        self.sidebar_tree.expand_path(path);
    }

    pub fn collapse_path(&mut self, path: &[String]) {
        self.sidebar_tree.collapse_path(path);
    }

    pub fn is_expanded(&self, path: &[String]) -> bool {
        self.sidebar_tree.is_expanded(path)
    }

    /// Updates current_* scope fields from a path vector.
    pub fn set_scope_from_path(&mut self, path: &[String]) {
        self.current_program = path.first().cloned();
        self.current_project = path.get(1).cloned();
        self.current_milestone = path.get(2).cloned();
        self.current_task = path.get(3).cloned();
    }

    /// Updates current_* scope fields from the tree model's selected path.
    pub fn update_scope_from_tree(&mut self) {
        let path = self.sidebar_tree.selected_path().to_vec();
        self.set_scope_from_path(&path);
    }

    /// Navigate up in sidebar and return the new index.
    pub fn navigate_up(&mut self) -> usize {
        let new_index = navigate_up(&self.sidebar_items, self.selected_entry_index);
        self.selected_entry_index = new_index;
        new_index
    }

    /// Navigate down in sidebar and return the new index.
    pub fn navigate_down(&mut self) -> usize {
        let new_index = navigate_down(&self.sidebar_items, self.selected_entry_index);
        self.selected_entry_index = new_index;
        new_index
    }
}

/// Section of the sidebar.
#[derive(Debug, Clone, PartialEq)]
pub enum SidebarSection {
    Programs,
    Planning,
    Journal,
    Notes,
}

/// An item in the sidebar navigation tree.
#[derive(Debug, Clone)]
pub struct SidebarItem {
    pub name: String,
    #[allow(dead_code)]
    pub section: SidebarSection,
    pub is_header: bool,
    pub is_planning_item: Option<String>,
    pub is_journal_item: Option<String>,
    pub indent: usize,
    pub path: Option<std::path::PathBuf>,
    pub tree_path: Option<Vec<String>>,
    pub has_children: bool,
    pub is_create_action: bool,
    pub journal_path: Option<Vec<String>>,
    pub is_journal_header: bool,
    pub node_data: SidebarNodeData,
}

impl SidebarItem {
    #[must_use]
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, section: SidebarSection) -> Self {
        Self {
            name: name.into(),
            section,
            is_header: false,
            is_planning_item: None,
            is_journal_item: None,
            indent: 0,
            path: None,
            tree_path: None,
            has_children: false,
            is_create_action: false,
            journal_path: None,
            is_journal_header: false,
            node_data: SidebarNodeData::Empty,
        }
    }

    /// Marks this item as a header.
    #[must_use]
    #[allow(dead_code)]
    pub fn header(mut self) -> Self {
        self.is_header = true;
        self
    }

    /// Sets the indent level for this item.
    #[must_use]
    #[allow(dead_code)]
    pub fn indent(mut self, level: usize) -> Self {
        self.indent = level;
        self
    }

    /// Sets the path for this item.
    #[must_use]
    #[allow(dead_code)]
    pub fn path(mut self, path: std::path::PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Marks this as a planning item.
    #[must_use]
    #[allow(dead_code)]
    pub fn planning_item(mut self, item_type: impl Into<String>) -> Self {
        self.is_planning_item = Some(item_type.into());
        self
    }

    /// Marks this as a journal item.
    #[must_use]
    #[allow(dead_code)]
    pub fn journal_item(mut self, item_type: impl Into<String>) -> Self {
        self.is_journal_item = Some(item_type.into());
        self
    }

    /// Marks this as a create action item.
    #[must_use]
    #[allow(dead_code)]
    pub fn create_action(mut self) -> Self {
        self.is_create_action = true;
        self
    }

    /// Sets the journal path for this item.
    #[must_use]
    pub fn journal_path(mut self, path: Vec<String>) -> Self {
        self.journal_path = Some(path);
        self
    }

    /// Marks this as a journal header (year/month).
    #[must_use]
    pub fn journal_header(mut self) -> Self {
        self.is_journal_header = true;
        self
    }

    #[must_use]
    pub fn node_data(mut self, data: SidebarNodeData) -> Self {
        self.node_data = data;
        self
    }

    pub fn program(entry: DirectoryEntry) -> Self {
        Self {
            name: entry.name.clone(),
            section: SidebarSection::Programs,
            is_header: false,
            is_planning_item: None,
            is_journal_item: None,
            indent: 0,
            path: Some(entry.path.clone()),
            tree_path: Some(vec![entry.name.clone()]),
            has_children: false,
            is_create_action: false,
            journal_path: None,
            is_journal_header: false,
            node_data: SidebarNodeData::Program(entry),
        }
    }

    pub fn journal(node: JournalNode, path: Vec<String>) -> Self {
        let label = node.label().to_string();
        let is_header = node.is_header();
        let indent = path.len();
        let tree_path = path.clone();
        Self {
            name: label,
            section: SidebarSection::Journal,
            is_header: false,
            is_planning_item: None,
            is_journal_item: None,
            indent,
            path: None,
            tree_path: Some(tree_path),
            has_children: is_header,
            is_create_action: false,
            journal_path: Some(path),
            is_journal_header: is_header,
            node_data: SidebarNodeData::Journal(node),
        }
    }

    pub fn note(node: NoteNode) -> Self {
        let label = node.label().to_string();
        let is_container = node.is_container();
        let path_components = node.path_components();
        let indent = path_components.len().saturating_sub(1);
        Self {
            name: label,
            section: SidebarSection::Notes,
            is_header: false,
            is_planning_item: None,
            is_journal_item: None,
            indent,
            path: match &node {
                NoteNode::Entry { path, .. } => Some(path.clone()),
                _ => None,
            },
            tree_path: Some(path_components),
            has_children: is_container,
            is_create_action: false,
            journal_path: None,
            is_journal_header: false,
            node_data: SidebarNodeData::Note(node),
        }
    }
}

/// State for tree navigation.
#[derive(Debug, Clone, Default)]
pub struct TreeState {
    pub path: Vec<String>,
    #[allow(dead_code)]
    pub expanded: Vec<String>,
}

impl TreeState {
    /// Creates a new tree state at the root level.
    #[must_use]
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current depth in the tree.
    #[must_use]
    #[allow(dead_code)]
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// Returns true if we're at the root level.
    #[must_use]
    #[allow(dead_code)]
    pub fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    /// Navigates into a child node.
    #[allow(dead_code)]
    pub fn push(&mut self, name: impl Into<String>) {
        self.path.push(name.into());
    }

    /// Navigates up one level.
    #[allow(dead_code)]
    pub fn pop(&mut self) -> Option<String> {
        self.path.pop()
    }

    /// Clears the navigation path.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.path.clear();
    }
}

/// Builds the sidebar items list from the current navigation state.
///
/// # Arguments
/// * `programs` - List of programs
/// * `projects` - List of projects (if a program is selected)
/// * `milestones` - List of milestones (if a project is selected)
/// * `tasks` - List of tasks (if a milestone is selected)
/// * `current_program` - Currently selected program name
/// * `current_project` - Currently selected project name
/// * `current_milestone` - Currently selected milestone name
///
/// # Returns
/// A vector of sidebar items to display.
#[must_use]
pub fn build_sidebar_items(
    programs: &[DirectoryEntry],
    projects: &[DirectoryEntry],
    milestones: &[DirectoryEntry],
    tasks: &[DirectoryEntry],
    current_program: Option<&str>,
    current_project: Option<&str>,
    current_milestone: Option<&str>,
) -> Vec<SidebarItem> {
    let mut items = Vec::new();

    // Programs section header
    items.push(SidebarItem::new("Programs", SidebarSection::Programs).header());

    // Programs list - show empty state if no programs exist
    if programs.is_empty() {
        items.push(
            SidebarItem::new("+ Create Program...", SidebarSection::Programs)
                .indent(1)
                .create_action(),
        );
    } else {
        for prog in programs {
            items.push(
                SidebarItem::new(&prog.name, SidebarSection::Programs).path(prog.path.clone()),
            );

            // Show projects if this program is expanded
            if current_program == Some(prog.name.as_str()) {
                for proj in projects {
                    items.push(
                        SidebarItem::new(&proj.name, SidebarSection::Programs)
                            .indent(1)
                            .path(proj.path.clone()),
                    );

                    // Show milestones if this project is expanded
                    if current_project == Some(proj.name.as_str()) {
                        for mile in milestones {
                            items.push(
                                SidebarItem::new(&mile.name, SidebarSection::Programs)
                                    .indent(2)
                                    .path(mile.path.clone()),
                            );

                            // Show tasks if this milestone is expanded
                            if current_milestone == Some(mile.name.as_str()) {
                                for task in tasks {
                                    items.push(
                                        SidebarItem::new(&task.name, SidebarSection::Programs)
                                            .indent(3)
                                            .path(task.path.clone()),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Spacer
    items.push(SidebarItem::new("", SidebarSection::Planning));

    // Planning section
    items.push(SidebarItem::new("Planning", SidebarSection::Planning).header());
    items.push(
        SidebarItem::new("Current Plan", SidebarSection::Planning).planning_item("WeeklyPlanning"),
    );
    items.push(SidebarItem::new("Backlog", SidebarSection::Planning).planning_item("Backlog"));

    // Spacer
    items.push(SidebarItem::new("", SidebarSection::Journal));

    // Journal section
    items.push(SidebarItem::new("Journal", SidebarSection::Journal).header());
    items.push(SidebarItem::new("Today", SidebarSection::Journal).journal_item("Today"));
    items.push(SidebarItem::new("History", SidebarSection::Journal).journal_item("History"));

    items
}

/// Calculates the next selectable index when navigating up.
///
/// # Arguments
/// * `items` - The sidebar items list
/// * `current_index` - Currently selected index
///
/// # Returns
/// The next selectable index (wrapping around).
#[must_use]
pub fn navigate_up(items: &[SidebarItem], current_index: usize) -> usize {
    if items.is_empty() {
        return 0;
    }

    let mut new_index = current_index;
    loop {
        if new_index == 0 {
            new_index = items.len() - 1;
        } else {
            new_index -= 1;
        }

        let item = &items[new_index];
        if !item.is_header && !item.name.is_empty() {
            break;
        }

        if new_index == current_index {
            break;
        }
    }

    new_index
}

/// Calculates the next selectable index when navigating down.
///
/// # Arguments
/// * `items` - The sidebar items list
/// * `current_index` - Currently selected index
///
/// # Returns
/// The next selectable index (wrapping around).
#[must_use]
pub fn navigate_down(items: &[SidebarItem], current_index: usize) -> usize {
    if items.is_empty() {
        return 0;
    }

    let mut new_index = current_index;
    loop {
        new_index = (new_index + 1) % items.len();

        let item = &items[new_index];
        if !item.is_header && !item.name.is_empty() {
            break;
        }

        if new_index == current_index {
            break;
        }
    }

    new_index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_state_depth() {
        let mut state = TreeState::new();
        assert_eq!(state.depth(), 0);
        assert!(state.is_root());

        state.push("program");
        assert_eq!(state.depth(), 1);
        assert!(!state.is_root());

        state.push("project");
        assert_eq!(state.depth(), 2);

        state.pop();
        assert_eq!(state.depth(), 1);

        state.clear();
        assert!(state.is_root());
    }

    #[test]
    fn test_sidebar_item_builder() {
        let item = SidebarItem::new("Test", SidebarSection::Programs)
            .header()
            .indent(2);

        assert_eq!(item.name, "Test");
        assert!(item.is_header);
        assert_eq!(item.indent, 2);
        assert!(!item.is_create_action);

        let create_item = SidebarItem::new("+ Create...", SidebarSection::Programs).create_action();
        assert!(create_item.is_create_action);
    }

    #[test]
    fn test_navigate_up_empty() {
        let items: Vec<SidebarItem> = vec![];
        assert_eq!(navigate_up(&items, 0), 0);
    }

    #[test]
    fn test_navigate_down_empty() {
        let items: Vec<SidebarItem> = vec![];
        assert_eq!(navigate_down(&items, 0), 0);
    }

    #[test]
    fn test_navigate_skips_headers() {
        let items = vec![
            SidebarItem::new("Header", SidebarSection::Programs).header(),
            SidebarItem::new("Item1", SidebarSection::Programs),
            SidebarItem::new("Item2", SidebarSection::Programs),
        ];

        // Starting at Item1 (index 1), navigate down should skip header and go to Item2
        let next = navigate_down(&items, 1);
        assert_eq!(next, 2);

        // Starting at Item2 (index 2), navigate up should skip header and go to Item1
        let prev = navigate_up(&items, 2);
        assert_eq!(prev, 1);
    }

    #[test]
    fn test_build_sidebar_items() {
        let programs = vec![DirectoryEntry {
            name: "prog1".to_string(),
            path: std::path::PathBuf::from("/prog1"),
            is_dir: true,
        }];

        let items = build_sidebar_items(&programs, &[], &[], &[], None, None, None);

        // Should have: Programs header, prog1, spacer, Planning header, 2 items, spacer, Journal header, 2 items
        assert!(items.len() > 5);
        assert!(items[0].is_header);
        assert_eq!(items[0].name, "Programs");
    }

    #[test]
    fn test_build_sidebar_items_empty_shows_create() {
        let programs: Vec<DirectoryEntry> = vec![];

        let items = build_sidebar_items(&programs, &[], &[], &[], None, None, None);

        // Should have: Programs header, "+ Create Program...", spacer, Planning header, 2 items, spacer, Journal header, 2 items
        assert!(items.len() > 5);
        assert!(items[0].is_header);
        assert_eq!(items[0].name, "Programs");

        // Second item should be the create action
        assert_eq!(items[1].name, "+ Create Program...");
        assert!(items[1].is_create_action);
        assert_eq!(items[1].indent, 1);
    }

    #[test]
    fn test_first_and_last_selectable_in_section() {
        // Create a sidebar structure similar to build_sidebar_items:
        // - Programs header (index 0)
        // - Program1 (index 1)
        // - Program2 (index 2)
        // - Spacer (index 3)
        // - Planning header (index 4)
        // - Current Plan (index 5)
        // - Backlog (index 6)
        // - Spacer (index 7)
        // - Journal header (index 8)
        // - Today (index 9)
        // - History (index 10)
        let items = vec![
            SidebarItem::new("Programs", SidebarSection::Programs).header(),
            SidebarItem::new("Program1", SidebarSection::Programs),
            SidebarItem::new("Program2", SidebarSection::Programs),
            SidebarItem::new("", SidebarSection::Planning), // spacer
            SidebarItem::new("Planning", SidebarSection::Planning).header(),
            SidebarItem::new("Current Plan", SidebarSection::Planning)
                .planning_item("WeeklyPlanning"),
            SidebarItem::new("Backlog", SidebarSection::Planning).planning_item("Backlog"),
            SidebarItem::new("", SidebarSection::Journal), // spacer
            SidebarItem::new("Journal", SidebarSection::Journal).header(),
            SidebarItem::new("Today", SidebarSection::Journal).journal_item("Today"),
            SidebarItem::new("History", SidebarSection::Journal).journal_item("History"),
        ];

        // Test finding first selectable in Journal section
        let first_journal = items
            .iter()
            .position(|i| {
                !i.is_header && !i.name.is_empty() && i.section == SidebarSection::Journal
            })
            .unwrap();
        assert_eq!(items[first_journal].name, "Today");
        assert_eq!(first_journal, 9);

        // Test finding last selectable in Journal section (for upward navigation)
        let last_journal = items
            .iter()
            .rposition(|i| {
                !i.is_header && !i.name.is_empty() && i.section == SidebarSection::Journal
            })
            .unwrap();
        assert_eq!(items[last_journal].name, "History");
        assert_eq!(last_journal, 10);

        // Test finding first selectable in Planning section
        let first_planning = items
            .iter()
            .position(|i| {
                !i.is_header && !i.name.is_empty() && i.section == SidebarSection::Planning
            })
            .unwrap();
        assert_eq!(items[first_planning].name, "Current Plan");
        assert_eq!(first_planning, 5);

        // Test finding last selectable in Planning section (for upward navigation)
        let last_planning = items
            .iter()
            .rposition(|i| {
                !i.is_header && !i.name.is_empty() && i.section == SidebarSection::Planning
            })
            .unwrap();
        assert_eq!(items[last_planning].name, "Backlog");
        assert_eq!(last_planning, 6);
    }
}

/// Tracks the notes tree data (loaded from disk).
#[derive(Debug, Clone, Default)]
pub struct NotesTreeState {
    entries: Vec<NoteEntry>,
    discovered_folders: std::collections::BTreeSet<(String, String)>,
}

impl NotesTreeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_entries(&mut self, entries: Vec<NoteEntry>) {
        self.entries = entries;
    }

    pub fn set_discovered_folders(&mut self, folders: Vec<(String, String)>) {
        self.discovered_folders = folders.into_iter().collect();
    }

    pub fn entries(&self) -> &[NoteEntry] {
        &self.entries
    }

    /// Returns sorted, deduplicated category names present in loaded entries.
    pub fn categories(&self, default_categories: &[String]) -> Vec<String> {
        // Always show all configured categories even if empty
        let mut cats: Vec<String> = default_categories.to_vec();
        // Also include any on-disk categories not in config (user may have added manually)
        for entry in &self.entries {
            if !cats.contains(&entry.category) {
                cats.push(entry.category.clone());
            }
        }
        cats
    }

    /// Returns sorted subfolder names within a given category.
    pub fn folders_for_category(&self, category: &str) -> Vec<String> {
        let mut folders: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for entry in &self.entries {
            if entry.category == category
                && let Some(folder) = &entry.folder
            {
                folders.insert(folder.clone());
            }
        }
        for (cat, folder) in &self.discovered_folders {
            if cat == category {
                folders.insert(folder.clone());
            }
        }
        folders.into_iter().collect()
    }

    /// Returns entries directly in a category (no subfolder).
    pub fn entries_in_category(&self, category: &str) -> Vec<&NoteEntry> {
        self.entries
            .iter()
            .filter(|e| e.category == category && e.folder.is_none())
            .collect()
    }

    /// Returns entries within a specific subfolder.
    pub fn entries_in_folder(&self, category: &str, folder: &str) -> Vec<&NoteEntry> {
        self.entries
            .iter()
            .filter(|e| e.category == category && e.folder.as_deref() == Some(folder))
            .collect()
    }
}

/// Tracks expansion state for the journal history tree
#[derive(Debug, Clone, Default)]
pub struct JournalTreeState {
    entries: Vec<crate::storage::JournalEntry>,
    discovered_years: std::collections::BTreeSet<String>,
    discovered_months: std::collections::BTreeSet<(String, String)>,
}

pub fn journal_entry_label(jpath: &[String]) -> Option<&str> {
    jpath
        .get(if jpath.len() == 3 { 2 } else { 1 })
        .map(|s| s.as_str())
}

impl JournalTreeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn set_entries(&mut self, entries: Vec<crate::storage::JournalEntry>) {
        self.entries = entries;
    }

    pub fn set_discovered_dirs(&mut self, years: Vec<String>, months: Vec<(String, String)>) {
        self.discovered_years = years.into_iter().collect();
        self.discovered_months = months.into_iter().collect();
    }

    pub fn entries(&self) -> &[crate::storage::JournalEntry] {
        &self.entries
    }

    pub fn years(&self) -> Vec<String> {
        let mut years: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for entry in &self.entries {
            if let Some(year) = extract_year_from_path(&entry.path) {
                years.insert(year);
            }
        }
        years.extend(self.discovered_years.iter().cloned());
        years.into_iter().rev().collect()
    }

    pub fn months_for_year(&self, year: &str) -> Vec<String> {
        let mut months: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for entry in &self.entries {
            if let Some((entry_year, month)) = extract_year_month_from_path(&entry.path)
                && entry_year == year
            {
                months.insert(month);
            }
        }
        for (entry_year, month) in &self.discovered_months {
            if entry_year == year {
                months.insert(month.clone());
            }
        }
        months.into_iter().collect()
    }

    pub fn entries_for_month(&self, year: &str, month: &str) -> Vec<&crate::storage::JournalEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                extract_year_month_from_path(&entry.path)
                    .is_some_and(|(y, m)| y == year && m == month)
            })
            .collect()
    }
}

#[cfg(test)]
mod tree_state_tests {
    use super::*;

    #[test]
    fn notes_tree_state_includes_discovered_empty_folders() {
        let mut state = NotesTreeState::new();
        state.set_entries(vec![]);
        state.set_discovered_folders(vec![(
            "Projects".to_string(),
            "Tether-Firmware".to_string(),
        )]);

        let folders = state.folders_for_category("Projects");
        assert_eq!(folders, vec!["Tether-Firmware".to_string()]);
        assert!(
            state
                .entries_in_folder("Projects", "Tether-Firmware")
                .is_empty()
        );
    }

    #[test]
    fn journal_tree_state_includes_discovered_years_and_months() {
        let mut state = JournalTreeState::new();
        state.set_entries(vec![]);
        state.set_discovered_dirs(
            vec!["2026".to_string()],
            vec![("2026".to_string(), "04".to_string())],
        );

        assert_eq!(state.years(), vec!["2026".to_string()]);
        assert_eq!(state.months_for_year("2026"), vec!["04".to_string()]);
        assert!(state.entries_for_month("2026", "04").is_empty());
    }
}
