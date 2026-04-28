use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init(log_dir: PathBuf) -> tracing_appender::non_blocking::WorkerGuard {
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "claude-limits.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,claude_limits_lib=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();

    tracing::info!("Logging initialized at {:?}", log_dir);
    guard
}

pub fn log_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "claude-limits", "ClaudeLimits")
        .map(|p| p.data_local_dir().join("logs"))
        .unwrap_or_else(|| PathBuf::from(".claude-monitor/logs"))
}
