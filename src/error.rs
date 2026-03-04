//! Layered error types for Chronicle library.
//!
//! This module provides domain-specific error types using `thiserror`.
//! Library code uses `crate::Result<T>`, while `main.rs` uses `anyhow::Result`.

use std::path::PathBuf;
use thiserror::Error;

/// Top-level error type that wraps all sub-errors.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Model error: {0}")]
    Model(#[from] ModelError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),
}

/// Convenience alias for Results using our Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Configuration-related errors.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// Storage-related errors.
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Element not found: {0}")]
    NotFound(String),
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    #[error("Failed to parse template fields: {0}")]
    TemplateParse(String),
    #[error("Invalid element path: {0}")]
    InvalidPath(String),
}

/// Model-related errors.
#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Element not found: {0}")]
    NotFound(String),
    #[error("Parse error: {0}")]
    Parse(String),
}
