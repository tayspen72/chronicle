use crate::theme::colors::{ColorError, parse_color};
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

use crate::theme::Theme;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to parse TOML: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Invalid color '{0}': {1}")]
    InvalidColor(String, ColorError),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlTheme {
    #[serde(default)]
    pub palette: HashMap<String, String>,
    #[serde(default)]
    pub ui: UiScopes,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UiScopes {
    #[serde(default)]
    pub background: Option<StyleDef>,
    #[serde(default)]
    pub surface: Option<StyleDef>,
    #[serde(default)]
    pub text: Option<TextScopes>,
    #[serde(default)]
    pub selection: Option<SelectionScopes>,
    #[serde(default)]
    pub border: Option<BorderScopes>,
    #[serde(default)]
    pub sidebar: Option<SidebarScopes>,
    #[serde(default)]
    pub status: Option<StatusScopes>,
    #[serde(default)]
    pub content: Option<ContentScopes>,
    #[serde(default)]
    pub wizard: Option<WizardScopes>,
    #[serde(default)]
    pub command: Option<CommandScopes>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TextScopes {
    #[serde(default)]
    pub primary: Option<StyleDef>,
    #[serde(default)]
    pub secondary: Option<StyleDef>,
    #[serde(default)]
    pub muted: Option<StyleDef>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SelectionScopes {
    #[serde(default)]
    pub bg: Option<String>,
    #[serde(default)]
    pub fg: Option<String>,
    #[serde(default)]
    pub active: Option<StyleDef>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BorderScopes {
    #[serde(default)]
    pub normal: Option<StyleDef>,
    #[serde(default)]
    pub focused: Option<StyleDef>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SidebarScopes {
    #[serde(default)]
    pub header: Option<StyleDef>,
    #[serde(default)]
    pub item: Option<StyleDef>,
    #[serde(default)]
    pub selected: Option<StyleDef>,
    #[serde(default)]
    pub create_action: Option<StyleDef>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct StatusScopes {
    #[serde(default)]
    pub normal: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub select: Option<String>,
    #[serde(default)]
    pub review: Option<String>,
    #[serde(default)]
    pub hierarchical_selection: Option<String>,
    #[serde(default)]
    pub planning_preview: Option<String>,
    #[serde(default)]
    pub task_detail_wizard: Option<String>,
    #[serde(default)]
    pub current_plan_navigation: Option<String>,
    #[serde(default)]
    pub theme_selection: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ContentScopes {
    #[serde(default)]
    pub title: Option<StyleDef>,
    #[serde(default)]
    pub header: Option<StyleDef>,
    #[serde(default)]
    pub table_header: Option<StyleDef>,
    #[serde(default)]
    pub table_border: Option<StyleDef>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WizardScopes {
    #[serde(default)]
    pub field_label: Option<StyleDef>,
    #[serde(default)]
    pub field_value: Option<StyleDef>,
    #[serde(default)]
    pub field_value_empty: Option<StyleDef>,
    #[serde(default)]
    pub field_value_auto: Option<StyleDef>,
    #[serde(default)]
    pub button_confirm: Option<StyleDef>,
    #[serde(default)]
    pub button_cancel: Option<StyleDef>,
    #[serde(default)]
    pub button_selected: Option<StyleDef>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CommandScopes {
    #[serde(default)]
    pub input: Option<StyleDef>,
    #[serde(default)]
    pub result: Option<StyleDef>,
    #[serde(default)]
    pub result_selected: Option<StyleDef>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StyleDef {
    #[serde(default)]
    pub fg: Option<String>,
    #[serde(default)]
    pub bg: Option<String>,
    #[serde(default)]
    pub modifiers: Option<Vec<String>>,
}

pub fn parse_theme(toml_content: &str) -> Result<Theme, ParseError> {
    let toml_theme: TomlTheme = toml::from_str(toml_content)?;
    resolve_theme(toml_theme)
}

fn resolve_theme(toml_theme: TomlTheme) -> Result<Theme, ParseError> {
    let palette = resolve_palette(&toml_theme.palette)?;
    let ui = resolve_ui_scopes(&toml_theme.ui, &palette)?;

    Ok(Theme { palette, ui })
}

fn resolve_palette(
    palette: &HashMap<String, String>,
) -> Result<HashMap<String, Color>, ParseError> {
    let mut resolved = HashMap::new();
    for (name, value) in palette {
        let color = parse_color(value).map_err(|e| ParseError::InvalidColor(value.clone(), e))?;
        resolved.insert(name.clone(), color);
    }
    Ok(resolved)
}

fn resolve_style(
    def: &Option<StyleDef>,
    palette: &HashMap<String, Color>,
) -> Result<Option<Style>, ParseError> {
    let def = match def {
        Some(d) => d,
        None => return Ok(None),
    };

    let fg = if let Some(fg) = &def.fg {
        Some(resolve_color(fg, palette)?)
    } else {
        None
    };

    let bg = if let Some(bg) = &def.bg {
        Some(resolve_color(bg, palette)?)
    } else {
        None
    };

    let mut modifiers = Modifier::empty();
    if let Some(mods) = &def.modifiers {
        for m in mods {
            match m.to_lowercase().as_str() {
                "bold" => modifiers |= Modifier::BOLD,
                "italic" => modifiers |= Modifier::ITALIC,
                "underline" => modifiers |= Modifier::UNDERLINED,
                "reverse" => modifiers |= Modifier::REVERSED,
                "hidden" => modifiers |= Modifier::HIDDEN,
                "dim" => modifiers |= Modifier::DIM,
                "crossed_out" => modifiers |= Modifier::CROSSED_OUT,
                _ => {}
            }
        }
    }

    let mut style = Style::default();
    if let Some(fg) = fg {
        style = style.fg(fg);
    }
    if let Some(bg) = bg {
        style = style.bg(bg);
    }
    if modifiers != Modifier::empty() {
        style = style.add_modifier(modifiers);
    }

    Ok(Some(style))
}

fn resolve_color(value: &str, palette: &HashMap<String, Color>) -> Result<Color, ParseError> {
    if let Some(c) = palette.get(value) {
        Ok(*c)
    } else {
        parse_color(value).map_err(|e| ParseError::InvalidColor(value.to_string(), e))
    }
}

fn resolve_ui_scopes(
    ui: &UiScopes,
    palette: &HashMap<String, Color>,
) -> Result<crate::theme::UiScopes, ParseError> {
    Ok(crate::theme::UiScopes {
        background: resolve_style(&ui.background, palette)?,
        surface: resolve_style(&ui.surface, palette)?,
        text: resolve_text_scopes(&ui.text, palette)?,
        selection: resolve_selection_scopes(&ui.selection, palette)?,
        border: resolve_border_scopes(&ui.border, palette)?,
        sidebar: resolve_sidebar_scopes(&ui.sidebar, palette)?,
        status: resolve_status_scopes(&ui.status, palette)?,
        content: resolve_content_scopes(&ui.content, palette)?,
        wizard: resolve_wizard_scopes(&ui.wizard, palette)?,
        command: resolve_command_scopes(&ui.command, palette)?,
    })
}

fn resolve_text_scopes(
    text: &Option<TextScopes>,
    palette: &HashMap<String, Color>,
) -> Result<crate::theme::TextScopes, ParseError> {
    let text = match text {
        Some(t) => t,
        None => return Ok(Default::default()),
    };

    Ok(crate::theme::TextScopes {
        primary: resolve_style(&text.primary, palette)?,
        secondary: resolve_style(&text.secondary, palette)?,
        muted: resolve_style(&text.muted, palette)?,
    })
}

fn resolve_selection_scopes(
    sel: &Option<SelectionScopes>,
    palette: &HashMap<String, Color>,
) -> Result<crate::theme::SelectionScopes, ParseError> {
    let sel = match sel {
        Some(s) => s,
        None => return Ok(Default::default()),
    };

    Ok(crate::theme::SelectionScopes {
        bg: if let Some(bg) = &sel.bg {
            Some(resolve_color(bg, palette)?)
        } else {
            None
        },
        fg: if let Some(fg) = &sel.fg {
            Some(resolve_color(fg, palette)?)
        } else {
            None
        },
        active: resolve_style(&sel.active, palette)?,
    })
}

fn resolve_border_scopes(
    border: &Option<BorderScopes>,
    palette: &HashMap<String, Color>,
) -> Result<crate::theme::BorderScopes, ParseError> {
    let border = match border {
        Some(b) => b,
        None => return Ok(Default::default()),
    };

    Ok(crate::theme::BorderScopes {
        normal: resolve_style(&border.normal, palette)?,
        focused: resolve_style(&border.focused, palette)?,
    })
}

fn resolve_sidebar_scopes(
    sidebar: &Option<SidebarScopes>,
    palette: &HashMap<String, Color>,
) -> Result<crate::theme::SidebarScopes, ParseError> {
    let sidebar = match sidebar {
        Some(s) => s,
        None => return Ok(Default::default()),
    };

    Ok(crate::theme::SidebarScopes {
        header: resolve_style(&sidebar.header, palette)?,
        item: resolve_style(&sidebar.item, palette)?,
        selected: resolve_style(&sidebar.selected, palette)?,
        create_action: resolve_style(&sidebar.create_action, palette)?,
    })
}

fn resolve_status_scopes(
    status: &Option<StatusScopes>,
    palette: &HashMap<String, Color>,
) -> Result<crate::theme::StatusScopes, ParseError> {
    let status = match status {
        Some(s) => s,
        None => return Ok(Default::default()),
    };

    Ok(crate::theme::StatusScopes {
        normal: if let Some(s) = &status.normal {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
        command: if let Some(s) = &status.command {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
        input: if let Some(s) = &status.input {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
        select: if let Some(s) = &status.select {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
        review: if let Some(s) = &status.review {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
        hierarchical_selection: if let Some(s) = &status.hierarchical_selection {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
        planning_preview: if let Some(s) = &status.planning_preview {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
        task_detail_wizard: if let Some(s) = &status.task_detail_wizard {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
        current_plan_navigation: if let Some(s) = &status.current_plan_navigation {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
        theme_selection: if let Some(s) = &status.theme_selection {
            Some(resolve_color(s, palette)?)
        } else {
            None
        },
    })
}

fn resolve_content_scopes(
    content: &Option<ContentScopes>,
    palette: &HashMap<String, Color>,
) -> Result<crate::theme::ContentScopes, ParseError> {
    let content = match content {
        Some(c) => c,
        None => return Ok(Default::default()),
    };

    Ok(crate::theme::ContentScopes {
        title: resolve_style(&content.title, palette)?,
        header: resolve_style(&content.header, palette)?,
        table_header: resolve_style(&content.table_header, palette)?,
        table_border: resolve_style(&content.table_border, palette)?,
    })
}

fn resolve_wizard_scopes(
    wizard: &Option<WizardScopes>,
    palette: &HashMap<String, Color>,
) -> Result<crate::theme::WizardScopes, ParseError> {
    let wizard = match wizard {
        Some(w) => w,
        None => return Ok(Default::default()),
    };

    Ok(crate::theme::WizardScopes {
        field_label: resolve_style(&wizard.field_label, palette)?,
        field_value: resolve_style(&wizard.field_value, palette)?,
        field_value_empty: resolve_style(&wizard.field_value_empty, palette)?,
        field_value_auto: resolve_style(&wizard.field_value_auto, palette)?,
        button_confirm: resolve_style(&wizard.button_confirm, palette)?,
        button_cancel: resolve_style(&wizard.button_cancel, palette)?,
        button_selected: resolve_style(&wizard.button_selected, palette)?,
    })
}

fn resolve_command_scopes(
    cmd: &Option<CommandScopes>,
    palette: &HashMap<String, Color>,
) -> Result<crate::theme::CommandScopes, ParseError> {
    let cmd = match cmd {
        Some(c) => c,
        None => return Ok(Default::default()),
    };

    Ok(crate::theme::CommandScopes {
        input: resolve_style(&cmd.input, palette)?,
        result: resolve_style(&cmd.result, palette)?,
        result_selected: resolve_style(&cmd.result_selected, palette)?,
    })
}
