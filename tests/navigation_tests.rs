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

    let initial = app.navigation_state.selected_entry_index;
    app.handle_key(KeyCode::Down);
    let after = app.navigation_state.selected_entry_index;

    // Should move to next item (unless at end)
    let total_items = app.navigation_state.sidebar_items.len();
    if initial < total_items - 1 {
        assert!(after > initial, "Down should move to next item");
    }
}

#[test]
fn test_navigate_up_moves_selection() {
    let (_, mut app) = app_with_empty_workspace();

    // Move down first
    app.handle_key(KeyCode::Down);

    let before = app.navigation_state.selected_entry_index;
    app.handle_key(KeyCode::Up);
    let after = app.navigation_state.selected_entry_index;

    assert!(
        after <= before,
        "Up should move to previous item or stay at top"
    );
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
        app.journal_tree_state.is_expanded(&[]),
        "History should be expanded after Right key"
    );
}

#[test]
fn test_years_not_visible_when_history_not_expanded() {
    // Create app with journal entries
    let (_temp, app) = app_with_journal_entries();

    // Initially History is NOT expanded
    assert!(
        !app.journal_tree_state.is_expanded(&[]),
        "History should not be expanded initially"
    );

    // Years should NOT be in the sidebar
    let has_year = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.journal_path.as_ref().map_or(false, |p| p.len() == 1));

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
    app.journal_tree_state.expand(&[]);

    // Verify expansion state
    assert!(
        app.journal_tree_state.is_expanded(&[]),
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
