mod config;
mod model;
mod storage;
mod tui;

use anyhow::Result;

fn main() -> Result<()> {
    let config = config::Config::load_or_create()?;
    let mut app = tui::App::new(config);
    app.run()?;
    Ok(())
}
