use crate::error::{ConfigError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Key bindings for navigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationKeys {
    #[serde(default = "default_left")]
    pub left: char,
    #[serde(default = "default_right")]
    pub right: char,
    #[serde(default = "default_up")]
    pub up: char,
    #[serde(default = "default_down")]
    pub down: char,
}

impl Default for NavigationKeys {
    fn default() -> Self {
        NavigationKeys {
            left: default_left(),
            right: default_right(),
            up: default_up(),
            down: default_down(),
        }
    }
}

fn default_workflow() -> Vec<String> {
    vec![
        "New".into(),
        "Active".into(),
        "Blocked".into(),
        "Testing".into(),
        "Completed".into(),
        "Cancelled".into(),
    ]
}

fn default_navigator_width() -> u16 {
    60
}

fn default_planning_duration() -> String {
    "biweekly".into()
}

fn default_left() -> char {
    'h'
}

fn default_right() -> char {
    'l'
}

fn default_up() -> char {
    'k'
}

fn default_down() -> char {
    'j'
}

fn default_owner() -> String {
    String::new()
}

fn default_diagnostics_level() -> String {
    "debug".to_string()
}

fn default_diagnostics_enabled() -> bool {
    false
}

/// Diagnostics logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsConfig {
    #[serde(default = "default_diagnostics_enabled")]
    pub enabled: bool,
    #[serde(default = "default_diagnostics_level")]
    pub level: String,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            enabled: default_diagnostics_enabled(),
            level: default_diagnostics_level(),
        }
    }
}

/// User configuration for Chronicle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the chronicle workspace directory
    pub workspace: PathBuf,
    /// Editor command for opening files
    pub editor: String,
    /// Owner name for created elements
    #[serde(default = "default_owner")]
    pub owner: String,
    /// Workflow status values
    #[serde(default = "default_workflow")]
    pub workflow: Vec<String>,
    /// Width of the navigator panel in columns
    #[serde(default = "default_navigator_width")]
    pub navigator_width: u16,
    /// Planning iteration duration
    #[serde(default = "default_planning_duration")]
    pub planning_duration: String,
    /// Key bindings for navigation
    #[serde(default)]
    pub navigation_keys: NavigationKeys,
    /// Diagnostics logging for TUI debugging
    #[serde(default)]
    pub diagnostics: DiagnosticsConfig,
}

impl Default for Config {
    fn default() -> Self {
        let home = directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        Config {
            workspace: home.join("chronicle").join("workspace"),
            editor: "hx".to_string(),
            owner: default_owner(),
            workflow: default_workflow(),
            navigator_width: 60,
            planning_duration: "biweekly".to_string(),
            navigation_keys: NavigationKeys::default(),
            diagnostics: DiagnosticsConfig::default(),
        }
    }
}

impl Config {
    pub fn config_dir() -> Option<PathBuf> {
        directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .map(|home| home.join(".config").join("chronicle"))
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.toml"))
    }

    pub fn load_or_create() -> Result<Self> {
        let config_path = Self::config_path()
            .ok_or_else(|| ConfigError::NotFound(PathBuf::from("~/.config/chronicle")))?;

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            let mut config: Config = toml::from_str(&contents)?;

            if config.editor.is_empty() {
                config.editor = "hx".to_string();
            }

            Ok(config)
        } else {
            let config = Self::prompt_first_run()?;
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let contents = toml::to_string_pretty(&config)?;
            fs::write(&config_path, contents)?;
            Ok(config)
        }
    }

    fn prompt_first_run() -> Result<Config> {
        println!("\n=== First Run Setup ===\n");

        let home = directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let default_workspace = home.join("chronicle").join("workspace");

        print!(
            "Chronicle workspace directory [{}]: ",
            default_workspace.display()
        );
        io::stdout().flush()?;

        let mut workspace_input = String::new();
        io::stdin().read_line(&mut workspace_input)?;
        let workspace_input = workspace_input.trim().to_string();

        let workspace = if workspace_input.is_empty() {
            default_workspace
        } else {
            PathBuf::from(workspace_input)
        };

        print!("Text editor (e.g., hx, vim, code) [hx]: ");
        io::stdout().flush()?;

        let mut editor_input = String::new();
        io::stdin().read_line(&mut editor_input)?;
        let editor_input = editor_input.trim().to_string();

        let editor = if editor_input.is_empty() {
            "hx".to_string()
        } else {
            editor_input
        };

        print!("Your name (for created_by field) []: ");
        io::stdout().flush()?;

        let mut owner_input = String::new();
        io::stdin().read_line(&mut owner_input)?;
        let owner = owner_input.trim().to_string();

        println!("\n=== Setup Complete ===\n");
        println!("Workspace directory: {}", workspace.display());
        println!("Editor: {}", editor);
        println!("\nPress Enter to continue...");
        io::stdin().read_line(&mut String::new())?;

        Ok(Config {
            workspace,
            editor,
            owner,
            workflow: default_workflow(),
            navigator_width: 60,
            planning_duration: "biweekly".to_string(),
            navigation_keys: NavigationKeys::default(),
            diagnostics: DiagnosticsConfig::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_config() {
        let toml_content = r#"
workspace = "/home/user/chronicle"
editor = "helix"
owner = "Test User"
navigator_width = 60
planning_duration = "biweekly"

workflow = ["New", "Active", "Blocked", "Testing", "Completed", "Cancelled"]

[navigation_keys]
left = "h"
right = "l"
up = "k"
down = "j"
"#;
        let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

        assert_eq!(config.workspace, PathBuf::from("/home/user/chronicle"));
        assert_eq!(config.editor, "helix");
        assert_eq!(config.owner, "Test User");
        assert_eq!(config.navigator_width, 60);
        assert_eq!(config.planning_duration, "biweekly");
        assert_eq!(
            config.workflow,
            vec![
                "New",
                "Active",
                "Blocked",
                "Testing",
                "Completed",
                "Cancelled"
            ]
        );
        assert_eq!(config.navigation_keys.left, 'h');
        assert_eq!(config.navigation_keys.right, 'l');
        assert_eq!(config.navigation_keys.up, 'k');
        assert_eq!(config.navigation_keys.down, 'j');
        assert!(!config.diagnostics.enabled);
        assert_eq!(config.diagnostics.level, "debug");
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml_content = r#"
workspace = "/home/user/chronicle"
editor = "vim"
"#;
        let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

        assert_eq!(config.workspace, PathBuf::from("/home/user/chronicle"));
        assert_eq!(config.editor, "vim");
        // Check defaults are applied
        assert_eq!(config.owner, "");
        assert_eq!(config.navigator_width, 60);
        assert_eq!(config.planning_duration, "biweekly");
        assert_eq!(
            config.workflow,
            vec![
                "New",
                "Active",
                "Blocked",
                "Testing",
                "Completed",
                "Cancelled"
            ]
        );
        assert_eq!(config.navigation_keys.left, 'h');
        assert_eq!(config.navigation_keys.right, 'l');
        assert_eq!(config.navigation_keys.up, 'k');
        assert_eq!(config.navigation_keys.down, 'j');
        assert!(!config.diagnostics.enabled);
        assert_eq!(config.diagnostics.level, "debug");
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert!(config.workspace.to_string_lossy().contains("chronicle"));
        assert!(config.workspace.to_string_lossy().contains("workspace"));
        assert_eq!(config.editor, "hx");
        assert_eq!(config.owner, "");
        assert_eq!(config.navigator_width, 60);
        assert_eq!(config.planning_duration, "biweekly");
        assert_eq!(
            config.workflow,
            vec![
                "New",
                "Active",
                "Blocked",
                "Testing",
                "Completed",
                "Cancelled"
            ]
        );
    }

    #[test]
    fn test_navigation_keys_default() {
        let keys = NavigationKeys::default();

        assert_eq!(keys.left, 'h');
        assert_eq!(keys.right, 'l');
        assert_eq!(keys.up, 'k');
        assert_eq!(keys.down, 'j');
    }

    #[test]
    fn test_serialize_config() {
        let config = Config {
            workspace: PathBuf::from("/test/path"),
            editor: "code".to_string(),
            owner: "Test User".to_string(),
            workflow: vec!["todo".to_string(), "done".to_string()],
            navigator_width: 80,
            planning_duration: "weekly".to_string(),
            navigation_keys: NavigationKeys {
                left: 'a',
                right: 'd',
                up: 'w',
                down: 's',
            },
            diagnostics: DiagnosticsConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");

        assert!(toml_str.contains("workspace = \"/test/path\""));
        assert!(toml_str.contains("editor = \"code\""));
        assert!(toml_str.contains("owner = \"Test User\""));
        assert!(toml_str.contains("navigator_width = 80"));
        assert!(toml_str.contains("planning_duration = \"weekly\""));
        assert!(toml_str.contains("left = \"a\""));
        assert!(toml_str.contains("right = \"d\""));
    }
}
