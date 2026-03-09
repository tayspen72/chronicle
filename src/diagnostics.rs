use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer as _;

use crate::config::Config;

pub struct DiagnosticsGuard {
    _guard: Option<WorkerGuard>,
}

impl DiagnosticsGuard {
    pub fn disabled() -> Self {
        Self { _guard: None }
    }
}

pub fn init(config: &Config) -> crate::Result<DiagnosticsGuard> {
    if !diagnostics_enabled(config) {
        return Ok(DiagnosticsGuard::disabled());
    }

    let log_path = diagnostics_log_path(config);
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let (writer, guard) = tracing_appender::non_blocking(file);

    let fmt_layer = Layer::default()
        .with_ansi(false)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_writer(BoxMakeWriter::new(writer))
        .with_filter(level_filter(config));

    let init_result = tracing_subscriber::registry().with(fmt_layer).try_init();
    if init_result.is_err() {
        return Ok(DiagnosticsGuard::disabled());
    }

    tracing::info!(
        log_path = %log_path.display(),
        "chronicle diagnostics logging enabled"
    );
    log_workspace_snapshot(config);
    Ok(DiagnosticsGuard {
        _guard: Some(guard),
    })
}

fn diagnostics_enabled(config: &Config) -> bool {
    std::env::var("CHRONICLE_DIAGNOSTICS")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
        .unwrap_or(config.diagnostics.enabled)
}

fn level_filter(config: &Config) -> LevelFilter {
    match std::env::var("CHRONICLE_DIAGNOSTICS_LEVEL")
        .ok()
        .unwrap_or_else(|| config.diagnostics.level.clone())
        .to_ascii_lowercase()
        .as_str()
    {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        _ => LevelFilter::INFO,
    }
}

fn diagnostics_log_path(config: &Config) -> PathBuf {
    if let Ok(path) = std::env::var("CHRONICLE_DIAGNOSTICS_LOG") {
        return PathBuf::from(path);
    }
    Config::config_dir()
        .unwrap_or_else(|| config.workspace.join(".chronicle"))
        .join("logs")
        .join("diagnostics.log")
}

fn log_workspace_snapshot(config: &Config) {
    let workspace = &config.workspace;
    let mut file_count = 0usize;
    let mut md_count = 0usize;
    let mut dir_count = 0usize;
    let mut programs = 0usize;
    walk_workspace(workspace, &mut |path, is_dir| {
        if is_dir {
            dir_count += 1;
            return;
        }
        file_count += 1;
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            md_count += 1;
        }
        if path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("programs")
            && path.extension().and_then(|ext| ext.to_str()) == Some("md")
        {
            programs += 1;
        }
    });

    tracing::info!(
        workspace = %workspace.display(),
        files = file_count,
        markdown_files = md_count,
        directories = dir_count,
        top_level_program_files = programs,
        "workspace snapshot"
    );
}

fn walk_workspace(root: &Path, cb: &mut dyn FnMut(&Path, bool)) {
    let Ok(read_dir) = fs::read_dir(root) else {
        tracing::warn!(workspace = %root.display(), "failed to read workspace root");
        return;
    };

    for entry in read_dir.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            cb(&path, true);
            walk_workspace(&path, cb);
        } else {
            cb(&path, false);
        }
    }
}
