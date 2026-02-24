use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub data_path: PathBuf,
    pub editor: String,
}

impl Default for Config {
    fn default() -> Self {
        let home = directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        Config {
            version: "1.0".to_string(),
            data_path: home.join("chronicle").join("workspace"),
            editor: "hx".to_string(),
        }
    }
}

impl Config {
    pub fn config_dir() -> Option<PathBuf> {
        directories::ProjectDirs::from("rs", "chronicle", "chronicle")
            .map(|dirs| dirs.config_dir().to_path_buf())
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.toml"))
    }

    pub fn load_or_create() -> Result<Self> {
        let config_path = Self::config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            let mut config: Config = serde_yaml::from_str(&contents)?;

            if config.editor.is_empty() {
                config.editor = "hx".to_string();
            }

            Ok(config)
        } else {
            let config = Self::prompt_first_run()?;
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let contents = serde_yaml::to_string(&config)?;
            fs::write(&config_path, contents)?;
            Ok(config)
        }
    }

    fn prompt_first_run() -> Result<Config> {
        println!("\n=== First Run Setup ===\n");

        let home = directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let default_data_path = home.join("chronicle").join("workspace");

        print!(
            "Chronicle data directory [{}]: ",
            default_data_path.display()
        );
        io::stdout().flush()?;

        let mut data_path_input = String::new();
        io::stdin().read_line(&mut data_path_input)?;
        let data_path_input = data_path_input.trim().to_string();

        let data_path = if data_path_input.is_empty() {
            default_data_path
        } else {
            PathBuf::from(data_path_input)
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

        println!("\n=== Setup Complete ===\n");
        println!("Data directory: {}", data_path.display());
        println!("Editor: {}", editor);
        println!("\nPress Enter to continue...");
        io::stdin().read_line(&mut String::new())?;

        Ok(Config {
            version: "1.0".to_string(),
            data_path,
            editor,
        })
    }
}
