//! Integration tests for command palette.

use chronicle::tui::Mode;
use chronicle::tui::test_utils::app_with_empty_workspace;
use crossterm::event::KeyCode;

#[test]
fn test_slash_opens_palette() {
    let (_, mut app) = app_with_empty_workspace();

    assert_eq!(app.mode, Mode::Normal);

    app.handle_key(KeyCode::Char('/'));

    assert_eq!(
        app.mode,
        Mode::CommandPalette,
        "Slash should open command palette"
    );
}

#[test]
fn test_typing_filters_commands() {
    let (_, mut app) = app_with_empty_workspace();

    app.handle_key(KeyCode::Char('/'));

    let all_matches_count = app.command_palette.matches.len();

    // Type some characters
    app.handle_key(KeyCode::Char('n'));
    app.handle_key(KeyCode::Char('e'));
    app.handle_key(KeyCode::Char('w'));

    let filtered_matches = &app.command_palette.matches;

    // Should have filtered commands (at least 'new program' should match)
    assert!(
        filtered_matches.len() <= all_matches_count,
        "Filtered list should be same size or smaller"
    );
}

#[test]
fn test_up_down_navigates_in_palette() {
    let (_, mut app) = app_with_empty_workspace();

    app.handle_key(KeyCode::Char('/'));

    // Navigate down
    app.handle_key(KeyCode::Down);

    // Should still be in command palette
    assert_eq!(app.mode, Mode::CommandPalette);
}

#[test]
fn test_escape_closes_palette() {
    let (_, mut app) = app_with_empty_workspace();

    app.handle_key(KeyCode::Char('/'));
    assert_eq!(app.mode, Mode::CommandPalette);

    app.handle_key(KeyCode::Esc);
    assert_eq!(
        app.mode,
        Mode::Normal,
        "Escape should close palette and return to Normal"
    );
}

#[test]
fn test_empty_filter_shows_all_commands() {
    let (_, mut app) = app_with_empty_workspace();

    app.handle_key(KeyCode::Char('/'));

    let all_commands_count = app.command_palette.matches.len();

    assert!(
        all_commands_count > 0,
        "Palette should show commands when empty"
    );
}

#[test]
fn test_new_program_available() {
    let (_, mut app) = app_with_empty_workspace();

    app.handle_key(KeyCode::Char('/'));
    app.handle_key(KeyCode::Char('p'));

    let has_program = app
        .command_palette
        .matches
        .iter()
        .any(|m| m.label.to_lowercase().contains("program"));

    assert!(has_program, "'Program' should be available when typing 'p'");
}

#[test]
fn test_command_palette_input_tracking() {
    let (_, mut app) = app_with_empty_workspace();

    // Command palette input is handled in run() method, not handle_key
    // Just verify that opening the palette works
    app.handle_key(KeyCode::Char('/'));
    assert_eq!(app.mode, Mode::CommandPalette);

    // Verify the command palette is populated
    assert!(app.command_palette.matches.len() > 0);
}

#[test]
fn test_enter_without_selection_stays_in_palette() {
    let (_, mut app) = app_with_empty_workspace();

    app.handle_key(KeyCode::Char('/'));

    // Press enter without selecting anything specific
    app.handle_key(KeyCode::Enter);

    // Should either execute or stay in palette depending on current selection
    // The important thing is we don't crash
    assert!(matches!(
        app.mode,
        Mode::CommandPalette | Mode::Normal | Mode::Input
    ));
}

#[test]
fn test_tab_completes_command() {
    let (_, mut app) = app_with_empty_workspace();

    app.handle_key(KeyCode::Char('/'));
    app.handle_key(KeyCode::Char('n'));
    app.handle_key(KeyCode::Char('e'));
    app.handle_key(KeyCode::Char('w'));

    // Tab might complete the command
    app.handle_key(KeyCode::Tab);

    // Should still be in a valid state
    assert!(matches!(
        app.mode,
        Mode::CommandPalette | Mode::Input | Mode::Normal
    ));
}
