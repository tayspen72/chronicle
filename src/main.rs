//! Chronicle - Markdown-native planner and journal TUI.
//!
//! Binary entry point.

use anyhow::Result;
use chronicle::{commands, config, diagnostics, tui};

#[derive(Debug)]
enum CliCommand {
    Tui,
    Init,
    Jot {
        text: String,
    },
    Extract,
    NewTask {
        title: String,
        scope: Option<String>,
    },
}

fn parse_cli_command() -> Result<CliCommand> {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        return Ok(CliCommand::Tui);
    };

    match cmd.as_str() {
        "init" => Ok(CliCommand::Init),
        "jot" => {
            let text = args.collect::<Vec<_>>().join(" ");
            if text.trim().is_empty() {
                anyhow::bail!("Usage: chronicle jot <text>");
            }
            Ok(CliCommand::Jot { text })
        }
        "extract" => Ok(CliCommand::Extract),
        "new-task" => {
            let mut title_parts = Vec::new();
            let mut scope: Option<String> = None;

            while let Some(arg) = args.next() {
                if arg == "--scope" {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Missing value for --scope"))?;
                    scope = Some(value);
                    continue;
                }
                title_parts.push(arg);
            }

            let title = title_parts.join(" ");
            if title.trim().is_empty() {
                anyhow::bail!("Usage: chronicle new-task <title> [--scope <path>]");
            }

            Ok(CliCommand::NewTask { title, scope })
        }
        "help" | "--help" | "-h" => {
            println!(
                "Chronicle\n\nCommands:\n  chronicle                 Launch TUI\n  chronicle init            Initialize workspace directories\n  chronicle jot <text>      Append a line to today's journal\n  chronicle extract         Extract /todo lines from journals into backlog tasks\n  chronicle new-task <title> [--scope <path>]\n                            Create a task markdown file from template"
            );
            std::process::exit(0);
        }
        _ => anyhow::bail!(
            "Unknown command '{}'. Run 'chronicle --help' for usage.",
            cmd
        ),
    }
}

fn main() -> Result<()> {
    match parse_cli_command()? {
        CliCommand::Init => commands::init::run().map_err(|e| anyhow::anyhow!("{e}"))?,
        CliCommand::Jot { text } => {
            commands::jot::run(&text).map_err(|e| anyhow::anyhow!("{e}"))?
        }
        CliCommand::Extract => commands::extract::run().map_err(|e| anyhow::anyhow!("{e}"))?,
        CliCommand::NewTask { title, scope } => {
            commands::new_task::run(&title, scope.as_deref()).map_err(|e| anyhow::anyhow!("{e}"))?
        }
        CliCommand::Tui => {
            let config = config::Config::load_or_create().map_err(|e| anyhow::anyhow!("{e}"))?;
            let _diagnostics = diagnostics::init(&config).map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut app = tui::App::new(config);
            app.run().map_err(|e| anyhow::anyhow!("{e}"))?;
        }
    }
    Ok(())
}
