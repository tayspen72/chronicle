use crate::error::{ConfigError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

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

fn default_importance() -> Vec<String> {
    vec!["low".into(), "medium".into(), "high".into()]
}

fn default_planning_duration() -> String {
    "weekly".into()
}

fn default_diagnostics_level() -> String {
    "debug".to_string()
}

fn default_diagnostics_enabled() -> bool {
    false
}

fn default_theme() -> String {
    "default_dark".to_string()
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
    pub owner: String,
    /// Workflow status values
    #[serde(default = "default_workflow")]
    pub workflow: Vec<String>,
    /// Task importance values for creation/edit workflows
    #[serde(default = "default_importance")]
    pub importance: Vec<String>,
    /// Planning iteration duration
    #[serde(default = "default_planning_duration")]
    pub planning_duration: String,
    /// Diagnostics logging for TUI debugging
    #[serde(default)]
    pub diagnostics: DiagnosticsConfig,
    /// Theme name to use for TUI
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        let home = directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        Config {
            workspace: home.join("chronicle").join("workspace"),
            editor: "hx".to_string(),
            owner: String::new(),
            workflow: default_workflow(),
            importance: default_importance(),
            planning_duration: "weekly".to_string(),
            diagnostics: DiagnosticsConfig::default(),
            theme: default_theme(),
        }
    }
}

impl Config {
    /// Validates that planning_duration is one of the allowed values
    pub fn validate_planning_duration(&self) -> crate::Result<()> {
        let valid_durations = ["weekly", "biweekly", "6weekly"];
        if !valid_durations.contains(&self.planning_duration.as_str()) {
            return Err(crate::Error::Config(ConfigError::InvalidPlanningDuration(
                self.planning_duration.clone(),
                valid_durations.join(", "),
            )));
        }
        Ok(())
    }

    fn validate_required_fields(&self) -> crate::Result<()> {
        if self.workspace.as_os_str().is_empty() {
            return Err(crate::Error::Config(ConfigError::Invalid(
                "workspace is required and cannot be empty".to_string(),
            )));
        }
        if self.editor.trim().is_empty() {
            return Err(crate::Error::Config(ConfigError::Invalid(
                "editor is required and cannot be empty".to_string(),
            )));
        }
        if self.owner.trim().is_empty() {
            return Err(crate::Error::Config(ConfigError::Invalid(
                "owner is required and cannot be empty".to_string(),
            )));
        }
        Ok(())
    }

    pub fn config_dir() -> Option<PathBuf> {
        directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .map(|home| home.join(".config").join("chronicle"))
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        let contents = toml::to_string_pretty(self)?;
        fs::write(&config_path, contents)?;
        Ok(())
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.toml"))
    }

    pub fn load_theme(&self) -> crate::Result<crate::theme::Theme> {
        crate::theme::load_theme(&self.theme).map_err(|e| {
            crate::Error::Config(crate::error::ConfigError::Invalid(format!(
                "Failed to load theme '{}': {}",
                self.theme, e
            )))
        })
    }

    pub fn load_or_create() -> Result<Self> {
        let config_path = Self::config_path()
            .ok_or_else(|| ConfigError::NotFound(PathBuf::from("~/.config/chronicle")))?;

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&contents)?;

            config.validate_required_fields()?;
            // Validate planning duration
            config.validate_planning_duration()?;

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

        let owner = loop {
            print!("Your name (required for created_by/assigned_to): ");
            io::stdout().flush()?;
            let mut owner_input = String::new();
            io::stdin().read_line(&mut owner_input)?;
            let owner = owner_input.trim().to_string();
            if owner.is_empty() {
                println!("Owner is required.");
                continue;
            }
            break owner;
        };

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
            importance: default_importance(),
            planning_duration: "weekly".to_string(),
            diagnostics: DiagnosticsConfig::default(),
            theme: default_theme(),
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
planning_duration = "weekly"

workflow = ["New", "Active", "Blocked", "Testing", "Completed", "Cancelled"]
importance = ["low", "medium", "high"]
"#;
        let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

        assert_eq!(config.workspace, PathBuf::from("/home/user/chronicle"));
        assert_eq!(config.editor, "helix");
        assert_eq!(config.owner, "Test User");
        assert_eq!(config.planning_duration, "weekly");
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
        assert_eq!(config.importance, vec!["low", "medium", "high"]);
        assert!(!config.diagnostics.enabled);
        assert_eq!(config.diagnostics.level, "debug");
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml_content = r#"
workspace = "/home/user/chronicle"
editor = "vim"
owner = "me"
"#;
        let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

        assert_eq!(config.workspace, PathBuf::from("/home/user/chronicle"));
        assert_eq!(config.editor, "vim");
        // Check defaults are applied
        assert_eq!(config.owner, "me");
        assert_eq!(config.planning_duration, "weekly");
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
        assert_eq!(config.importance, vec!["low", "medium", "high"]);
        assert!(!config.diagnostics.enabled);
        assert_eq!(config.diagnostics.level, "debug");
    }

    #[test]
    fn test_parse_missing_owner_fails() {
        let toml_content = r#"
workspace = "/home/user/chronicle"
editor = "vim"
"#;
        let parsed = toml::from_str::<Config>(toml_content);
        assert!(parsed.is_err(), "owner should be required in config.toml");
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert!(config.workspace.to_string_lossy().contains("chronicle"));
        assert!(config.workspace.to_string_lossy().contains("workspace"));
        assert_eq!(config.editor, "hx");
        assert_eq!(config.owner, "");
        assert_eq!(config.planning_duration, "weekly");
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
    fn test_serialize_config() {
        let config = Config {
            workspace: PathBuf::from("/test/path"),
            editor: "code".to_string(),
            owner: "Test User".to_string(),
            workflow: vec!["todo".to_string(), "done".to_string()],
            importance: vec!["low".to_string(), "medium".to_string(), "high".to_string()],
            planning_duration: "weekly".to_string(),
            diagnostics: DiagnosticsConfig::default(),
            theme: default_theme(),
        };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");

        assert!(toml_str.contains("workspace = \"/test/path\""));
        assert!(toml_str.contains("editor = \"code\""));
        assert!(toml_str.contains("owner = \"Test User\""));
        assert!(toml_str.contains("planning_duration = \"weekly\""));
    }

    #[test]
    fn test_validate_required_fields_rejects_empty_owner() {
        let config = Config {
            owner: String::new(),
            ..Config::default()
        };
        let result = config.validate_required_fields();
        assert!(result.is_err());
    }
}
