//! Integration tests for sidebar/tree navigation.

use chronicle::model::SelectedTask;
use chronicle::storage::JournalStorage;
use chronicle::storage::md::parse_element;
use chronicle::tui::navigation::SidebarSection;
use chronicle::tui::test_utils::{app_with_empty_workspace, app_with_journal_entries};
use chronicle::tui::{Mode, ViewType};
use crossterm::event::KeyCode;
use std::fs;
use std::path::PathBuf;

fn planned_task(
    name: &str,
    status: &str,
    program: &str,
    project: &str,
    milestone: &str,
) -> SelectedTask {
    SelectedTask {
        uuid: format!("uuid-{}", name),
        path: PathBuf::from(format!("/tmp/{name}.md")),
        program: program.to_string(),
        project: project.to_string(),
        milestone: milestone.to_string(),
        task_name: name.to_string(),
        status: status.to_string(),
        assigned_to: None,
        start_date: None,
        due_date: None,
        importance: None,
        description: None,
    }
}

#[test]
fn test_initial_mode_is_normal() {
    let (_, app) = app_with_empty_workspace();
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_initial_view_is_tree_view() {
    let (_, app) = app_with_empty_workspace();
    assert_eq!(app.current_view, ViewType::TreeView);
}

#[test]
fn test_navigate_down_moves_selection() {
    let (_, mut app) = app_with_empty_workspace();

    // Find an item in a section with multiple valid items (Journal: Today, History)
    // "Today" is at index 8, "History" is at index 9
    let today_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "Today")
        .expect("Today should exist in sidebar");
    app.navigation_state.selected_entry_index = today_idx;

    let initial = app.navigation_state.selected_entry_index;
    app.handle_key(KeyCode::Down);
    let after = app.navigation_state.selected_entry_index;

    // Should move to History (next valid item in Journal section)
    assert_ne!(after, initial, "Down should move to different item");
}

#[test]
fn test_navigate_up_moves_selection() {
    let (_, mut app) = app_with_empty_workspace();

    // Find an item in a section with multiple valid items (Journal: Today, History)
    // Start from History (index 9), then press Up to go to Today (index 8)
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "History")
        .expect("History should exist in sidebar");
    app.navigation_state.selected_entry_index = history_idx;

    let before = app.navigation_state.selected_entry_index;
    app.handle_key(KeyCode::Up);
    let after = app.navigation_state.selected_entry_index;

    // Should move to Today (previous valid item in Journal section)
    assert!(after <= before, "Up should move to previous item");
}

#[test]
fn test_sidebar_has_programs_header() {
    let (_, app) = app_with_empty_workspace();

    let first_item = &app.navigation_state.sidebar_items[0];
    assert_eq!(first_item.name, "Task Management");
    assert!(first_item.is_header);
}

#[test]
fn test_sidebar_has_planning_section() {
    let (_, app) = app_with_empty_workspace();

    let has_planning = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.name == "Planning" && item.is_header);

    assert!(has_planning, "Sidebar should have Planning section");
}

#[test]
fn test_sidebar_has_journal_section() {
    let (_, app) = app_with_empty_workspace();

    let has_journal = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.name == "Journal" && item.is_header);

    assert!(has_journal, "Sidebar should have Journal section");
}

#[test]
fn test_navigation_to_history_view() {
    let (_temp, mut app) = app_with_journal_entries();

    // Find History in sidebar
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History");

    assert!(history_idx.is_some(), "History should be in sidebar");

    app.navigation_state.selected_entry_index = history_idx.unwrap();

    // Press Right to expand history
    app.handle_key(KeyCode::Right);

    // Expanding history should render year nodes
    assert!(
        app.navigation_state
            .sidebar_items
            .iter()
            .any(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 1)),
        "Years should be visible after expanding History"
    );
}

#[test]
fn test_years_not_visible_when_history_not_expanded() {
    // Create app with journal entries
    let (_temp, app) = app_with_journal_entries();

    // Years should NOT be in the sidebar
    let has_year = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 1));

    assert!(
        !has_year,
        "Years should NOT be visible when History is collapsed"
    );
}

#[test]
fn test_journal_entries_loaded_on_history_expand() {
    let (_temp, mut app) = app_with_journal_entries();

    // Journal entries are preloaded during app startup.
    assert!(
        !app.journal_entries.is_empty(),
        "Journal entries should be preloaded at startup"
    );

    // Find History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History with Right
    app.handle_key(KeyCode::Right);

    // Journal entries should now be loaded
    assert!(
        !app.journal_entries.is_empty(),
        "Journal entries should be loaded after History expand"
    );

    // Should have 3 entries: 2026-03-15, 2026-03-14, 2026-02-28
    assert_eq!(
        app.journal_entries.len(),
        3,
        "Should have 3 journal entries"
    );
}

#[test]
fn test_journal_tree_state_years_and_months() {
    let (_temp, mut app) = app_with_journal_entries();

    // Load journal entries directly
    let entries = app.config.workspace.list_journal_entries().unwrap();
    app.journal_entries = entries;
    app.journal_tree_state
        .set_entries(app.journal_entries.clone());

    // Expand History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .expect("History should exist in sidebar");
    app.navigation_state.selected_entry_index = history_idx;
    app.handle_key(KeyCode::Right);

    // Check years
    let years = app.journal_tree_state.years();
    assert!(!years.is_empty(), "Should have at least one year");
    assert_eq!(years.len(), 1, "Should have exactly one year (2026)");
    assert_eq!(years[0], "2026", "Year should be 2026");

    // Check months for the year
    let months = app.journal_tree_state.months_for_year("2026");
    assert!(!months.is_empty(), "Should have at least one month");
    // Should have 02 and 03
    assert!(months.contains(&"02".to_string()), "Should have 02");
    assert!(months.contains(&"03".to_string()), "Should have 03");
}

#[test]
fn test_sidebar_includes_programs_when_journal_expanded() {
    let (_temp, mut app) = app_with_journal_entries();

    // Find History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History
    app.handle_key(KeyCode::Right);

    // Sidebar should still have Task Management section
    let has_programs = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.name == "Task Management" && item.is_header);
    assert!(
        has_programs,
        "Task Management section should still be in sidebar"
    );

    // Sidebar should still have Planning section
    let has_planning = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.name == "Planning" && item.is_header);
    assert!(has_planning, "Planning section should still be in sidebar");
}

#[test]
fn test_expand_history_shows_month_items() {
    let (_temp, mut app) = app_with_journal_entries();

    // Find and select History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History
    app.handle_key(KeyCode::Right);

    // Year "2026" should be visible at indent 1
    let year_items: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.name == "2026")
        .collect();

    assert!(
        !year_items.is_empty(),
        "Year '2026' should be visible after History expand"
    );
    assert_eq!(year_items[0].indent, 1, "Year should have indent 1");

    // Expand year "2026" to see months
    let year_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "2026")
        .unwrap();
    app.navigation_state.selected_entry_index = year_idx;
    app.handle_key(KeyCode::Right);

    // Now month items should be visible at indent 2 (under year)
    let month_items: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.name == "02" || item.name == "03")
        .collect();

    assert!(
        !month_items.is_empty(),
        "Month items (02, 03) should be visible after expanding year"
    );

    // Should have indent 2 (under year, under History)
    for month_item in &month_items {
        assert_eq!(
            month_item.indent, 2,
            "Month '{}' should have indent 2",
            month_item.name
        );
    }
}

#[test]
fn test_expand_history_with_preloaded_entries_shows_months() {
    // This tests the case where journal_entries is already populated
    // (e.g., user viewed "Today" first, then clicked History)
    let (_temp, mut app) = app_with_journal_entries();

    // Pre-populate journal_entries as if user viewed Today first
    let entries = app.config.workspace.list_journal_entries().unwrap();
    app.journal_entries = entries;

    // Find and select History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History - shows year "2026"
    app.handle_key(KeyCode::Right);

    // Selection should be on year "2026", not a program
    let selected_idx = app.navigation_state.selected_entry_index;
    let selected_item = &app.navigation_state.sidebar_items[selected_idx];
    assert_eq!(
        selected_item.name, "2026",
        "Selection should be on year '2026', got '{}'",
        selected_item.name
    );

    // Expand year to see months
    app.handle_key(KeyCode::Right);

    // Month items should be visible
    let month_items: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.name == "02" || item.name == "03")
        .collect();

    assert!(
        !month_items.is_empty(),
        "Month items should be visible even when journal_entries was pre-populated"
    );
}

#[test]
fn test_expand_history_selection_stays_on_journal() {
    let (_temp, mut app) = app_with_journal_entries();

    // Find and select History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History - should select year "2026"
    app.handle_key(KeyCode::Right);

    // Selection should be on year "2026", NOT a program
    let selected_idx = app.navigation_state.selected_entry_index;
    let selected_item = &app.navigation_state.sidebar_items[selected_idx];

    assert_eq!(
        selected_item.name, "2026",
        "Selection should be on year '2026', got '{}'",
        selected_item.name
    );
    assert!(
        selected_item.is_journal_header,
        "Selection should be a journal header"
    );

    // Should NOT be a program (programs have 'path' set, not 'journal_path')
    assert!(
        selected_item.journal_path.is_some(),
        "Selected item should be a journal item"
    );
    assert!(
        selected_item.path.is_none(),
        "Selected item should NOT be a program (no path)"
    );
}

#[test]
fn test_hjkl_mappings_do_not_crash() {
    let (_, mut app) = app_with_empty_workspace();

    // k key - up (from initial position)
    app.handle_key(KeyCode::Char('k'));

    // j key - down
    app.handle_key(KeyCode::Char('j'));

    // h/l keys should not crash
    app.handle_key(KeyCode::Char('h'));
    app.handle_key(KeyCode::Char('l'));

    // Should still be in valid state
    assert!(matches!(app.mode, Mode::Normal | Mode::CommandPalette));
}

#[test]
fn test_navigate_left_from_journal_entry_collapses_to_parent() {
    let (_temp, mut app) = app_with_journal_entries();

    // Pre-populate journal entries
    let entries = app.config.workspace.list_journal_entries().unwrap();
    app.journal_entries = entries.clone();
    app.journal_tree_state
        .set_entries(app.journal_entries.clone());

    // Find and select History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History (shows year "2026")
    app.handle_key(KeyCode::Right);

    // Expand year (shows months)
    app.handle_key(KeyCode::Right);

    // Select 03 month
    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "03")
        .unwrap();
    app.navigation_state.selected_entry_index = march_idx;

    // Expand 03 (shows entries)
    app.handle_key(KeyCode::Right);

    // Should now have journal entry items visible
    let has_entries = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 3));
    assert!(
        has_entries,
        "Journal entries should be visible after expanding month"
    );

    // Select first journal entry
    let entry_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 3))
        .unwrap();
    app.navigation_state.selected_entry_index = entry_idx;

    // Get the entry's path before navigation
    let entry_path = app.navigation_state.sidebar_items[entry_idx]
        .journal_path
        .clone();
    assert!(
        entry_path.is_some(),
        "Selected item should have journal_path"
    );
    let entry_path = entry_path.unwrap();

    // Press Left to collapse entry and go to parent month
    app.handle_key(KeyCode::Left);

    // Selection should be on the parent month header
    let selected_idx = app.navigation_state.selected_entry_index;
    let selected_item = &app.navigation_state.sidebar_items[selected_idx];

    assert_eq!(
        selected_item.name, "03",
        "Selection should be on 03 (parent), got '{}'",
        selected_item.name
    );
    assert!(
        selected_item.is_journal_header,
        "Selected item should be a journal header (03)"
    );
    assert!(
        selected_item
            .journal_path
            .as_ref()
            .is_some_and(|p| p.len() == 2),
        "Selected item should have 2-element path (year, month)"
    );

    // Verify the entry is collapsed (no longer visible)
    let has_entries_after = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.journal_path.as_ref() == Some(&entry_path));
    assert!(
        !has_entries_after,
        "The entry should no longer be visible after collapse"
    );
}

#[test]
fn test_navigate_left_from_month_collapses_to_history() {
    let (_temp, mut app) = app_with_journal_entries();

    // Pre-populate journal entries
    let entries = app.config.workspace.list_journal_entries().unwrap();
    app.journal_entries = entries.clone();
    app.journal_tree_state
        .set_entries(app.journal_entries.clone());

    // Find and select History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History (shows year "2026")
    app.handle_key(KeyCode::Right);

    // Expand year (shows months)
    app.handle_key(KeyCode::Right);

    // Select 03 month
    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "03")
        .unwrap();
    app.navigation_state.selected_entry_index = march_idx;

    // Verify we're on a month header
    let march_item = &app.navigation_state.sidebar_items[march_idx];
    assert!(march_item.is_journal_header, "Should be on month header");

    // Press Left to collapse month and go to year "2026"
    app.handle_key(KeyCode::Left);

    // Selection should be on year "2026"
    let selected_idx = app.navigation_state.selected_entry_index;
    let selected_item = &app.navigation_state.sidebar_items[selected_idx];

    assert_eq!(
        selected_item.name, "2026",
        "Selection should be on year '2026' after collapsing month, got '{}'",
        selected_item.name
    );
    assert!(
        selected_item.is_journal_header,
        "Selected item should be year header"
    );

    // Months should be hidden after collapsing the month tier (via its parent year).
    let month_items: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.name == "03" || item.name == "02")
        .collect();
    assert!(
        month_items.is_empty(),
        "Months should NOT be visible after collapsing month tier. Found: {:?}",
        month_items.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_collapse_year_hides_months_in_sidebar() {
    // Regression test: collapsing year should hide months in sidebar
    let (_temp, mut app) = app_with_journal_entries();

    // Pre-populate journal entries
    let entries = app.config.workspace.list_journal_entries().unwrap();
    app.journal_entries = entries.clone();
    app.journal_tree_state
        .set_entries(app.journal_entries.clone());

    // Find and select History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History (shows year "2026")
    app.handle_key(KeyCode::Right);

    // Expand year (shows months)
    app.handle_key(KeyCode::Right);

    // Verify months are visible
    let month_items_before: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.name == "03" || item.name == "02")
        .collect();
    assert!(
        !month_items_before.is_empty(),
        "Months should be visible after expanding year"
    );

    // Select year "2026"
    let year_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "2026")
        .unwrap();
    app.navigation_state.selected_entry_index = year_idx;

    // Press Left to collapse year - should go back to History
    app.handle_key(KeyCode::Left);

    // Selection should be on History
    let selected_item =
        &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert_eq!(
        selected_item.name, "History",
        "Selection should be on History after collapsing year"
    );

    // Months should NOT be visible in sidebar after collapsing year
    let month_items_after: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.name == "03" || item.name == "02")
        .collect();
    assert!(
        month_items_after.is_empty(),
        "Months should NOT be visible after collapsing year. Found: {:?}",
        month_items_after
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );

    // Years should also be hidden after collapsing at year tier back to History.
    let year_items_after: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 1))
        .collect();
    assert!(
        year_items_after.is_empty(),
        "Years should NOT be visible after collapsing year tier. Found: {:?}",
        year_items_after.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_collapse_month_hides_entries_in_sidebar() {
    // Regression test: collapsing month should hide entries in sidebar
    let (_temp, mut app) = app_with_journal_entries();

    // Pre-populate journal entries
    let entries = app.config.workspace.list_journal_entries().unwrap();
    app.journal_entries = entries.clone();
    app.journal_tree_state
        .set_entries(app.journal_entries.clone());

    // Find and select History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History (shows year "2026")
    app.handle_key(KeyCode::Right);

    // Expand year (shows months)
    app.handle_key(KeyCode::Right);

    // Verify NO entries are visible after expanding year (before expanding any month)
    let entries_before_expand_month: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 3))
        .collect();
    assert!(
        entries_before_expand_month.is_empty(),
        "No entries should be visible before expanding a month. Found: {:?}",
        entries_before_expand_month
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );

    // Select 03 explicitly (auto-selection after expanding year selects 02)
    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "03")
        .unwrap();
    app.navigation_state.selected_entry_index = march_idx;

    // Expand 03 (shows entries)
    app.handle_key(KeyCode::Right);

    // Selection should move into 03's first entry after expansion
    let selected_item =
        &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert!(
        selected_item
            .journal_path
            .as_ref()
            .is_some_and(|p| p.len() == 3),
        "Selection should move to the first 03 entry after expansion"
    );

    // Verify entries are visible after expanding 03
    let entries_after_expand: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 3))
        .collect();
    assert!(
        !entries_after_expand.is_empty(),
        "Entries should be visible after expanding 03"
    );

    // Press Left to collapse entry and parent month
    // This should hide entries and select the parent month
    app.handle_key(KeyCode::Left);

    // Selection should be on 03 (parent month), not the year
    let selected_item =
        &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert_eq!(
        selected_item.name, "03",
        "Selection should be on parent month after pressing Left on entry"
    );

    // Entries should NOT be visible in sidebar after collapsing month
    let entries_after: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 3))
        .collect();
    assert!(
        entries_after.is_empty(),
        "Entries should NOT be visible after collapsing month. Found: {:?}",
        entries_after.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_journal_tree_has_correct_indentation() {
    // Regression test: journal tree should have correct indentation with pipes
    let (_temp, mut app) = app_with_journal_entries();

    // Pre-populate journal entries
    let entries = app.config.workspace.list_journal_entries().unwrap();
    app.journal_entries = entries.clone();
    app.journal_tree_state
        .set_entries(app.journal_entries.clone());

    // Find and select History
    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .unwrap();
    app.navigation_state.selected_entry_index = history_idx;

    // Expand History (shows year "2026")
    app.handle_key(KeyCode::Right);

    // Expand year (shows months)
    app.handle_key(KeyCode::Right);

    // Check state after expanding year (before expanding any month)
    let feb_expanded = app
        .navigation_state
        .is_expanded(&["2026".to_string(), "02".to_string()]);
    let mar_expanded = app
        .navigation_state
        .is_expanded(&["2026".to_string(), "03".to_string()]);
    assert!(
        !feb_expanded,
        "02 should NOT be expanded after just expanding year"
    );
    assert!(
        !mar_expanded,
        "03 should NOT be expanded after just expanding year"
    );

    // Verify NO entries are visible after expanding year (before expanding any month)
    let entries_before_expand_month: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 3))
        .collect();
    assert!(
        entries_before_expand_month.is_empty(),
        "No entries should be visible before expanding a month. Found: {:?}",
        entries_before_expand_month
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );

    // Select 03 explicitly (auto-selection may have selected 02)
    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "03")
        .unwrap();
    app.navigation_state.selected_entry_index = march_idx;

    // Expand 03 (shows entries)
    app.handle_key(KeyCode::Right);

    // Verify indentation levels
    let year_items: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.name == "2026")
        .collect();
    assert!(!year_items.is_empty(), "Year '2026' should exist");
    assert_eq!(year_items[0].indent, 1, "Year should have indent 1");

    let feb_items: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.name == "02")
        .collect();
    assert!(!feb_items.is_empty(), "02 should exist");
    assert_eq!(feb_items[0].indent, 2, "02 should have indent 2");

    let entry_items: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 3))
        .collect();
    assert!(!entry_items.is_empty(), "Entries should exist");
    for entry in &entry_items {
        assert_eq!(
            entry.indent, 3,
            "Entry '{}' should have indent 3",
            entry.name
        );
    }
}

#[test]
fn test_navigate_down_from_last_program_to_planning() {
    let (_, mut app) = app_with_empty_workspace();

    // Find the last program item (before the Planning section)
    let items_before_planning: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .take_while(|i| i.section != SidebarSection::Planning)
        .enumerate()
        .filter(|(_, item)| !item.is_header && !item.name.is_empty())
        .collect();

    // Should have at least one program
    assert!(!items_before_planning.is_empty(), "Should have programs");

    // Get last program
    let (last_prog_idx, last_prog) = items_before_planning.last().unwrap();
    println!(
        "Last program: '{}' at index {}",
        last_prog.name, last_prog_idx
    );

    // Select it
    app.navigation_state.selected_entry_index = *last_prog_idx;

    // Press Down
    app.handle_key(KeyCode::Down);

    let after_idx = app.navigation_state.selected_entry_index;
    let after_item = &app.navigation_state.sidebar_items[after_idx];
    println!("After Down: '{}' at index {}", after_item.name, after_idx);

    // Should be in Planning section
    assert_eq!(
        after_item.section,
        SidebarSection::Planning,
        "Should navigate to Planning section, got {:?}",
        after_item.section
    );
}

#[test]
fn test_navigate_up_from_first_planning_to_programs() {
    let (_, mut app) = app_with_empty_workspace();

    // Find first Planning item
    let first_plan_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.section == SidebarSection::Planning && !i.is_header && !i.name.is_empty())
        .expect("Should have planning items");

    let first_plan_item = &app.navigation_state.sidebar_items[first_plan_idx];
    println!(
        "First planning item: '{}' at index {}",
        first_plan_item.name, first_plan_idx
    );

    // Select it
    app.navigation_state.selected_entry_index = first_plan_idx;

    // Press Up
    app.handle_key(KeyCode::Up);

    let after_idx = app.navigation_state.selected_entry_index;
    let after_item = &app.navigation_state.sidebar_items[after_idx];
    println!("After Up: '{}' at index {}", after_item.name, after_idx);

    // Should be in Programs section
    assert_eq!(
        after_item.section,
        SidebarSection::Programs,
        "Should navigate to Programs section, got {:?}",
        after_item.section
    );
}

#[test]
fn test_navigate_up_wraps_to_journal() {
    let (_, mut app) = app_with_empty_workspace();

    // Find first item (should be Task Management header)
    let first_idx = 0;
    let first_item = &app.navigation_state.sidebar_items[first_idx];
    println!(
        "First item: '{}' is_header={}",
        first_item.name, first_item.is_header
    );

    // Select first item
    app.navigation_state.selected_entry_index = first_idx;

    // Press Up (should wrap to end)
    app.handle_key(KeyCode::Up);

    let after_idx = app.navigation_state.selected_entry_index;
    let after_item = &app.navigation_state.sidebar_items[after_idx];
    println!(
        "After Up from first: '{}' at index {}",
        after_item.name, after_idx
    );

    // Should be in Notes section (the last section, which wrap lands on)
    assert_eq!(
        after_item.section,
        SidebarSection::Notes,
        "Should wrap to Notes section (last section), got {:?}",
        after_item.section
    );
}

#[test]
fn test_right_on_journal_entry_is_noop() {
    let (_temp, mut app) = app_with_journal_entries();

    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .expect("History should exist");
    app.navigation_state.selected_entry_index = history_idx;
    app.handle_key(KeyCode::Right);
    app.handle_key(KeyCode::Right);

    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "03")
        .expect("03 should exist");
    app.navigation_state.selected_entry_index = march_idx;
    app.handle_key(KeyCode::Right);

    let selected_before = app.navigation_state.selected_entry_index;
    let entry_path_before = app.navigation_state.sidebar_items[selected_before]
        .journal_path
        .clone()
        .expect("entry should have journal path");

    app.handle_key(KeyCode::Right);

    let selected_after = app.navigation_state.selected_entry_index;
    assert_eq!(
        selected_after, selected_before,
        "Right on journal entry should not move selection"
    );
    assert_eq!(
        app.navigation_state.sidebar_items[selected_after].journal_path,
        Some(entry_path_before),
        "Right on journal entry should keep selected entry"
    );
    assert_eq!(
        app.current_view,
        ViewType::TreeView,
        "Right on journal entry should not open content"
    );
}

#[test]
fn test_right_on_expanded_journal_headers_is_noop() {
    let (_temp, mut app) = app_with_journal_entries();

    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .expect("History should exist");
    app.navigation_state.selected_entry_index = history_idx;
    app.handle_key(KeyCode::Right);

    let year_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "2026")
        .expect("Year should exist");
    app.navigation_state.selected_entry_index = year_idx;
    app.handle_key(KeyCode::Right);

    let year_idx_again = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "2026")
        .expect("Year should still exist");
    app.navigation_state.selected_entry_index = year_idx_again;
    app.handle_key(KeyCode::Right);
    assert_eq!(
        app.navigation_state.selected_entry_index, year_idx_again,
        "Right on expanded year should be a no-op"
    );

    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "03")
        .expect("03 should exist");
    app.navigation_state.selected_entry_index = march_idx;
    app.handle_key(KeyCode::Right);

    let march_idx_again = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "03")
        .expect("03 should still exist");
    app.navigation_state.selected_entry_index = march_idx_again;
    app.handle_key(KeyCode::Right);
    assert_eq!(
        app.navigation_state.selected_entry_index, march_idx_again,
        "Right on expanded month should be a no-op"
    );
}

#[test]
fn test_repeated_left_on_journal_walks_up_tiers_consistently() {
    let (_temp, mut app) = app_with_journal_entries();

    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .expect("History should exist");
    app.navigation_state.selected_entry_index = history_idx;
    app.handle_key(KeyCode::Right);
    app.handle_key(KeyCode::Right);

    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "03")
        .expect("03 should exist");
    app.navigation_state.selected_entry_index = march_idx;
    app.handle_key(KeyCode::Right);

    app.handle_key(KeyCode::Left);
    let selected = &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert_eq!(selected.name, "03");
    assert_eq!(
        selected.journal_path.as_ref(),
        Some(&vec!["2026".to_string(), "03".to_string()]),
        "Left from entry should select the exact parent month node"
    );

    app.handle_key(KeyCode::Left);
    let selected = &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert_eq!(selected.name, "2026");
    assert_eq!(
        selected.journal_path.as_ref(),
        Some(&vec!["2026".to_string()]),
        "Left from month should select the exact parent year node"
    );

    app.handle_key(KeyCode::Left);
    let selected = &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert_eq!(selected.name, "History");

    let history_idx_before = app.navigation_state.selected_entry_index;
    app.handle_key(KeyCode::Left);
    assert_eq!(
        app.navigation_state.selected_entry_index, history_idx_before,
        "Left on History should be a no-op"
    );
}

#[test]
fn test_enter_on_journal_entry_launches_editor() {
    let (_temp, mut app) = app_with_journal_entries();

    let history_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "History")
        .expect("History should exist");
    app.navigation_state.selected_entry_index = history_idx;
    app.handle_key(KeyCode::Right);
    app.handle_key(KeyCode::Right);

    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "03")
        .expect("03 should exist");
    app.navigation_state.selected_entry_index = march_idx;
    app.handle_key(KeyCode::Right);

    app.handle_key(KeyCode::Enter);

    assert_eq!(app.current_view, ViewType::TreeView);
}

#[test]
fn test_current_plan_selected_renders_in_treeview_and_enter_starts_task_navigation_mode() {
    let (_, mut app) = app_with_empty_workspace();
    app.planning_session.active = true;
    app.planning_session.tasks = vec![
        planned_task("task1", "New", "Program1", "Project1", "Milestone1"),
        planned_task("task2", "Active", "Program1", "Project1", "Milestone1"),
    ];

    let current_plan_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "Current Plan")
        .expect("Current Plan should exist in sidebar");
    app.navigation_state.selected_entry_index = current_plan_idx;
    app.current_view = ViewType::TreeView;

    app.handle_key(KeyCode::Enter);

    assert_eq!(app.mode, Mode::CurrentPlanNavigation);
    assert_eq!(app.current_view, ViewType::TreeView);
}

#[test]
fn test_current_plan_navigation_uses_task_selection_only() {
    let (_, mut app) = app_with_empty_workspace();
    app.planning_session.active = true;
    app.planning_session.tasks = vec![
        planned_task("task1", "New", "Program1", "Project1", "Milestone1"),
        planned_task("task2", "Active", "Program1", "Project1", "Milestone1"),
    ];
    app.mode = Mode::CurrentPlanNavigation;
    app.review_state.selection_index = 0;
    app.navigation_state.selected_entry_index = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "Current Plan")
        .expect("Current Plan should exist in sidebar");

    let before_sidebar_index = app.navigation_state.selected_entry_index;
    app.handle_key(KeyCode::Down);
    assert_eq!(app.review_state.selection_index, 1);
    assert_eq!(
        app.navigation_state.selected_entry_index, before_sidebar_index,
        "Task navigation mode should not move sidebar selection"
    );

    app.handle_key(KeyCode::Up);
    assert_eq!(app.review_state.selection_index, 0);
}

#[test]
fn test_backlog_assign_writes_task_file() {
    let (temp, mut app) = app_with_empty_workspace();
    let workspace = temp.path();

    let task_path = workspace.join("task.md");
    let content = r#"---
uuid: task-1
title: Task 1
status: New
creation_date: 2026-03-24
type: task
---

# Description
"#;
    fs::write(&task_path, content).expect("should write task fixture");

    app.config.owner = "me".to_string();
    app.planning_session.active = true;
    app.planning_session.tasks = vec![SelectedTask {
        uuid: "task-1".to_string(),
        path: task_path.clone(),
        program: "Program1".to_string(),
        project: "Project1".to_string(),
        milestone: "Milestone1".to_string(),
        task_name: "Task 1".to_string(),
        status: "New".to_string(),
        assigned_to: None,
        start_date: None,
        due_date: None,
        importance: None,
        description: None,
    }];

    let backlog_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "Backlog")
        .expect("Backlog should exist in sidebar");
    app.navigation_state.selected_entry_index = backlog_idx;
    app.handle_key(KeyCode::Enter);
    app.handle_key(KeyCode::Char('a'));

    let updated = fs::read_to_string(&task_path).expect("should read updated task");
    let element = parse_element(&updated)
        .expect("parse should succeed")
        .expect("element should be present");
    let chronicle::model::Element::Task(task) = element else {
        panic!("expected task element")
    };
    assert_eq!(task.assigned_to.as_deref(), Some("me"));
}
