use crate::theme::{Theme, parse_theme};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

const EMBEDDED_AYU_EVOLVE: &str = include_str!("../../themes/ayu_evolve.toml");
const EMBEDDED_DEFAULT_DARK: &str = include_str!("../../themes/default_dark.toml");
const EMBEDDED_DRACULA: &str = include_str!("../../themes/dracula.toml");
const EMBEDDED_NORD: &str = include_str!("../../themes/nord.toml");

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("Failed to read theme file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse theme: {0}")]
    ParseError(#[from] crate::theme::parser::ParseError),
    #[error("Theme not found: {0}")]
    NotFound(String),
}

pub fn load_theme(name: &str) -> Result<Theme, LoadError> {
    if let Some(user_theme) = load_user_theme(name)? {
        return Ok(user_theme);
    }

    if let Some(built_in) = load_built_in_theme(name)? {
        return Ok(built_in);
    }

    if let Some(embedded) = load_embedded_theme(name)? {
        return Ok(embedded);
    }

    Err(LoadError::NotFound(name.to_string()))
}

fn load_embedded_theme(name: &str) -> Result<Option<Theme>, LoadError> {
    let content = match name {
        "ayu_evolve" => Some(EMBEDDED_AYU_EVOLVE),
        "default_dark" => Some(EMBEDDED_DEFAULT_DARK),
        "dracula" => Some(EMBEDDED_DRACULA),
        "nord" => Some(EMBEDDED_NORD),
        _ => None,
    };

    match content {
        Some(toml) => {
            let theme = parse_theme(toml)?;
            tracing::info!("Loaded embedded built-in theme: {}", name);
            Ok(Some(theme))
        }
        None => Ok(None),
    }
}

pub fn validate_theme(theme: &Theme) -> Vec<String> {
    let mut warnings = Vec::new();

    if theme.ui.text.primary.is_none() {
        warnings.push("ui.text.primary not defined".to_string());
    }
    if theme.ui.text.secondary.is_none() {
        warnings.push("ui.text.secondary not defined".to_string());
    }
    if theme.ui.sidebar.item.is_none() {
        warnings.push("ui.sidebar.item not defined".to_string());
    }
    if theme.ui.sidebar.selected.is_none() {
        warnings.push("ui.sidebar.selected not defined".to_string());
    }
    if theme.ui.sidebar.header.is_none() {
        warnings.push("ui.sidebar.header not defined".to_string());
    }
    if theme.ui.border.normal.is_none() {
        warnings.push("ui.border.normal not defined".to_string());
    }
    if theme.ui.border.focused.is_none() {
        warnings.push("ui.border.focused not defined".to_string());
    }
    if theme.ui.status.normal.is_none() {
        warnings.push("ui.status.normal not defined".to_string());
    }

    warnings
}

fn load_user_theme(name: &str) -> Result<Option<Theme>, LoadError> {
    let config_dir = get_user_config_dir()?;
    let theme_path = config_dir.join("themes").join(format!("{}.toml", name));

    if theme_path.exists() {
        let content = fs::read_to_string(&theme_path)?;
        let theme = parse_theme(&content)?;
        tracing::info!("Loaded user theme from: {:?}", theme_path);
        return Ok(Some(theme));
    }

    Ok(None)
}

fn load_built_in_theme(name: &str) -> Result<Option<Theme>, LoadError> {
    let theme_path = get_built_in_theme_path(name);

    if theme_path.exists() {
        let content = fs::read_to_string(&theme_path)?;
        let theme = parse_theme(&content)?;
        tracing::info!("Loaded built-in theme from: {:?}", theme_path);
        return Ok(Some(theme));
    }

    Ok(None)
}

fn get_built_in_theme_path(name: &str) -> PathBuf {
    let mut path = PathBuf::new();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    tracing::debug!("CARGO_MANIFEST_DIR: {}", manifest_dir);
    path.push(manifest_dir);
    path.push("themes");
    path.push(format!("{}.toml", name));
    tracing::debug!("Looking for theme at: {:?}", path);
    path
}

fn get_user_config_dir() -> Result<PathBuf, LoadError> {
    ProjectDirs::from("com", "chronicle", "chronicle")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or_else(|| LoadError::NotFound("Could not determine config directory".to_string()))
}

pub fn list_available_themes() -> Vec<String> {
    let mut themes = Vec::new();

    if let Ok(user_dir) = get_user_config_dir() {
        let themes_dir = user_dir.join("themes");
        if let Ok(entries) = fs::read_dir(themes_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "toml")
                    && let Some(stem) = entry.path().file_stem()
                {
                    themes.push(stem.to_string_lossy().to_string());
                }
            }
        }
    }

    let mut built_in_dir = PathBuf::new();
    built_in_dir.push(env!("CARGO_MANIFEST_DIR"));
    built_in_dir.push("themes");

    if let Ok(entries) = fs::read_dir(built_in_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "toml")
                && let Some(stem) = entry.path().file_stem()
            {
                let name = stem.to_string_lossy();
                if !themes.contains(&name.to_string()) {
                    themes.push(name.to_string());
                }
            }
        }
    }

    // Always guarantee the embedded built-ins appear, even if filesystem scans fail
    for name in ["ayu_evolve", "default_dark", "dracula", "nord"] {
        let name = name.to_string();
        if !themes.contains(&name) {
            themes.push(name);
        }
    }

    themes.sort();
    themes
}

pub fn get_theme_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "chronicle", "chronicle").map(|dirs| {
        let themes_dir = dirs.config_dir().join("themes");
        if !themes_dir.exists() {
            let _ = fs::create_dir_all(&themes_dir);
        }
        themes_dir
    })
}
