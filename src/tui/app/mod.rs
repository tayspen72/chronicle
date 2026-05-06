//! Per-concern `impl App` blocks split out of `tui/mod.rs`.
//!
//! The `App` struct itself, its constructor, and the type definitions
//! (`Mode`, `ViewType`, etc.) live in `tui/mod.rs`. Each submodule here
//! adds methods on `App` for one concern: theme selection, journal,
//! notes, planning, review, tree navigation, wizards, commands, and
//! the event loop.

pub mod commands;
pub mod journal;
pub mod notes;
pub mod review;
pub mod theme_select;
