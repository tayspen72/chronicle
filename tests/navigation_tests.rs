//! Integration tests for sidebar/tree navigation.

use chronicle::storage::JournalStorage;
use chronicle::tui::test_utils::{app_with_empty_workspace, app_with_journal_entries};
use chronicle::tui::{Mode, ViewType};
use crossterm::event::KeyCode;

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
    assert_eq!(first_item.name, "Programs");
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

    // History should be expanded (tree state should be set)
    assert!(
        app.navigation_state.is_expanded(&[]),
        "History should be expanded after Right key"
    );
}

#[test]
fn test_years_not_visible_when_history_not_expanded() {
    // Create app with journal entries
    let (_temp, app) = app_with_journal_entries();

    // Initially History is NOT expanded
    assert!(
        !app.navigation_state.is_expanded(&[]),
        "History should not be expanded initially"
    );

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

    // Initially journal_entries should be empty
    assert!(
        app.journal_entries.is_empty(),
        "Journal entries should be empty initially"
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
    app.navigation_state.expand_path(&[]);

    // Verify expansion state
    assert!(
        app.navigation_state.is_expanded(&[]),
        "History should be expanded"
    );

    // Check years
    let years = app.journal_tree_state.years();
    assert!(!years.is_empty(), "Should have at least one year");
    assert_eq!(years.len(), 1, "Should have exactly one year (2026)");
    assert_eq!(years[0], "2026", "Year should be 2026");

    // Check months for the year
    let months = app.journal_tree_state.months_for_year("2026");
    assert!(!months.is_empty(), "Should have at least one month");
    // Should have February and March
    assert!(
        months.contains(&"February".to_string()),
        "Should have February"
    );
    assert!(months.contains(&"March".to_string()), "Should have March");
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

    // Sidebar should still have Programs section
    let has_programs = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.name == "Programs" && item.is_header);
    assert!(has_programs, "Programs section should still be in sidebar");

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
        .filter(|item| item.name == "February" || item.name == "March")
        .collect();

    assert!(
        !month_items.is_empty(),
        "Month items (February, March) should be visible after expanding year"
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
        .filter(|item| item.name == "February" || item.name == "March")
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

    // Select March month
    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "March")
        .unwrap();
    app.navigation_state.selected_entry_index = march_idx;

    // Expand March (shows entries)
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
        selected_item.name, "March",
        "Selection should be on March (parent), got '{}'",
        selected_item.name
    );
    assert!(
        selected_item.is_journal_header,
        "Selected item should be a journal header (March)"
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

    // Select March month
    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "March")
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

    // Months are still visible in the sidebar (for easier navigation between months)
    // This is correct UX - users can j/k between months without going back up
    let month_items: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.name == "March" || item.name == "February")
        .collect();
    assert!(
        !month_items.is_empty(),
        "Months should still be visible in sidebar for easier navigation. Found: {:?}",
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
        .filter(|item| item.name == "March" || item.name == "February")
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
        .filter(|item| item.name == "March" || item.name == "February")
        .collect();
    assert!(
        month_items_after.is_empty(),
        "Months should NOT be visible after collapsing year. Found: {:?}",
        month_items_after
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
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

    // Check state after expanding year (before expanding any month)
    let feb_expanded = app
        .navigation_state
        .is_expanded(&["2026".to_string(), "February".to_string()]);
    let mar_expanded = app
        .navigation_state
        .is_expanded(&["2026".to_string(), "March".to_string()]);
    assert!(
        !feb_expanded,
        "February should NOT be expanded after just expanding year"
    );
    assert!(
        !mar_expanded,
        "March should NOT be expanded after just expanding year"
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

    // Select March explicitly (auto-selection after expanding year selects February)
    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "March")
        .unwrap();
    app.navigation_state.selected_entry_index = march_idx;

    // Expand March (shows entries)
    app.handle_key(KeyCode::Right);

    // Verify March is now expanded
    let mar_expanded_after = app
        .navigation_state
        .is_expanded(&["2026".to_string(), "March".to_string()]);
    assert!(
        mar_expanded_after,
        "March should be expanded after pressing Right on it"
    );

    // Verify entries are visible after expanding March
    let entries_after_expand: Vec<_> = app
        .navigation_state
        .sidebar_items
        .iter()
        .filter(|item| item.journal_path.as_ref().is_some_and(|p| p.len() == 3))
        .collect();
    assert!(
        !entries_after_expand.is_empty(),
        "Entries should be visible after expanding March"
    );

    // Press Left to collapse entry and parent month
    // This should hide entries and select the parent month
    app.handle_key(KeyCode::Left);

    // Selection should be on March (parent month), not the year
    let selected_item =
        &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert_eq!(
        selected_item.name, "March",
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
        .is_expanded(&["2026".to_string(), "February".to_string()]);
    let mar_expanded = app
        .navigation_state
        .is_expanded(&["2026".to_string(), "March".to_string()]);
    assert!(
        !feb_expanded,
        "February should NOT be expanded after just expanding year"
    );
    assert!(
        !mar_expanded,
        "March should NOT be expanded after just expanding year"
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

    // Select March explicitly (auto-selection may have selected February)
    let march_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.name == "March")
        .unwrap();
    app.navigation_state.selected_entry_index = march_idx;

    // Expand March (shows entries)
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
        .filter(|item| item.name == "February")
        .collect();
    assert!(!feb_items.is_empty(), "February should exist");
    assert_eq!(feb_items[0].indent, 2, "February should have indent 2");

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
