//! Chronicle - Markdown-native planner and journal TUI.
//!
//! Binary entry point.

use anyhow::Result;
use chronicle::{config, tui};

fn main() -> Result<()> {
    let config = config::Config::load_or_create().map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut app = tui::App::new(config);
    app.run().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
