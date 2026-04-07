//! Theme module for Chronicle TUI.
//!
//! Provides a theming system that allows users to customize the look and feel
//! of the terminal UI. Themes are defined in TOML files following a similar
//! format to helix-editor.

pub mod colors;
pub mod defaults;
pub mod loader;
pub mod parser;

use ratatui::style::{Color, Style};
use std::collections::HashMap;

pub use colors::parse_color;
pub use defaults::default_theme;
pub use loader::{get_theme_dir, list_available_themes, load_theme, validate_theme};
pub use parser::parse_theme;

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub palette: HashMap<String, Color>,
    pub ui: UiScopes,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UiScopes {
    pub background: Option<Style>,
    pub surface: Option<Style>,
    pub text: TextScopes,
    pub selection: SelectionScopes,
    pub border: BorderScopes,
    pub sidebar: SidebarScopes,
    pub status: StatusScopes,
    pub content: ContentScopes,
    pub wizard: WizardScopes,
    pub command: CommandScopes,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TextScopes {
    pub primary: Option<Style>,
    pub secondary: Option<Style>,
    pub muted: Option<Style>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SelectionScopes {
    pub bg: Option<Color>,
    pub fg: Option<Color>,
    pub active: Option<Style>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BorderScopes {
    pub normal: Option<Style>,
    pub focused: Option<Style>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SidebarScopes {
    pub header: Option<Style>,
    pub item: Option<Style>,
    pub selected: Option<Style>,
    pub create_action: Option<Style>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StatusScopes {
    pub normal: Option<Color>,
    pub command: Option<Color>,
    pub input: Option<Color>,
    pub select: Option<Color>,
    pub review: Option<Color>,
    pub hierarchical_selection: Option<Color>,
    pub planning_preview: Option<Color>,
    pub task_detail_wizard: Option<Color>,
    pub current_plan_navigation: Option<Color>,
    pub theme_selection: Option<Color>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ContentScopes {
    pub title: Option<Style>,
    pub header: Option<Style>,
    pub table_header: Option<Style>,
    pub table_border: Option<Style>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WizardScopes {
    pub field_label: Option<Style>,
    pub field_value: Option<Style>,
    pub field_value_empty: Option<Style>,
    pub field_value_auto: Option<Style>,
    pub button_confirm: Option<Style>,
    pub button_cancel: Option<Style>,
    pub button_selected: Option<Style>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CommandScopes {
    pub input: Option<Style>,
    pub result: Option<Style>,
    pub result_selected: Option<Style>,
}
