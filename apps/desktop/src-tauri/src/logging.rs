use std::fs;
use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

pub fn initialize(log_dir: &Path) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    fs::create_dir_all(log_dir)?;
    let appender = tracing_appender::rolling::daily(log_dir, "opennever-forge.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(writer)
        .try_init();

    Ok(guard)
}
