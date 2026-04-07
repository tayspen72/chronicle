//! Chronicle - Markdown-native planner and journal TUI.
//!
//! This crate provides the library functionality for Chronicle.

pub mod commands;
pub mod config;
pub mod diagnostics;
pub mod error;
pub mod model;
pub mod storage;
pub mod theme;
pub mod tui;

pub use error::{Error, Result};
