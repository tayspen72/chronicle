use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub data_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let home = directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        Config {
            version: "1.0".to_string(),
            data_path: home.join("chronicle"),
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
            let config: Config = serde_yaml::from_str(&contents)?;
            Ok(config)
        } else {
            let config = Config::default();
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let contents = serde_yaml::to_string(&config)?;
            fs::write(&config_path, contents)?;
            Ok(config)
        }
    }
}
