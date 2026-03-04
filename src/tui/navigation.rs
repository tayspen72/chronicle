//! Navigation module.
//!
//! Handles sidebar navigation state and tree traversal.
//!
//! NOTE: This module contains extracted types and logic for navigation.
//! The App struct in mod.rs still has inline implementations that duplicate this logic.
//! TODO: Wire up these types to replace inline navigation handling in App.

use crate::storage::DirectoryEntry;

/// Section of the sidebar.
#[derive(Debug, Clone, PartialEq)]
pub enum SidebarSection {
    Programs,
    Planning,
    Journal,
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
}

impl SidebarItem {
    /// Creates a new sidebar item.
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

    // Programs list
    for prog in programs {
        items.push(SidebarItem::new(&prog.name, SidebarSection::Programs).path(prog.path.clone()));

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

    // Spacer
    items.push(SidebarItem::new("", SidebarSection::Planning));

    // Planning section
    items.push(SidebarItem::new("Planning", SidebarSection::Planning).header());
    items.push(
        SidebarItem::new("Weekly Planning", SidebarSection::Planning)
            .planning_item("WeeklyPlanning"),
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
}
