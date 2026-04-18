use std::{error::Error, fs, path::Path};

use tracing_appender::{non_blocking, non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

pub(crate) fn init() -> Result<WorkerGuard, Box<dyn Error + Send + Sync>> {
    let logs_dir = Path::new("logs");
    fs::create_dir_all(logs_dir)?;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("api=info,praxis_live=info,tower_http=info")
    });

    let stdout_layer = fmt::layer().compact();

    let file_appender = rolling::daily(logs_dir, "praxis-live.log");
    let (file_writer, guard) = non_blocking(file_appender);
    let file_layer = fmt::layer().with_ansi(false).with_writer(file_writer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}
