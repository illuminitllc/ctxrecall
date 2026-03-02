use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

pub fn init_logging(log_dir: &Path) -> WorkerGuard {
    std::fs::create_dir_all(log_dir).expect("Failed to create log directory");

    let file_appender = tracing_appender::rolling::daily(log_dir, "ctxrecall.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true),
        )
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("ctxrecall=info")),
        )
        .init();

    guard
}
