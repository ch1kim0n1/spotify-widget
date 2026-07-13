use std::{fs, path::Path};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::error::{AppError, AppResult};

const LOG_FILE: &str = "companion.log";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const MAX_LOG_FILES: usize = 5;

pub fn initialize(log_dir: &Path) -> AppResult<WorkerGuard> {
    fs::create_dir_all(log_dir)?;
    rotate_if_needed(log_dir)?;
    let appender = tracing_appender::rolling::never(log_dir, LOG_FILE);
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            EnvFilter::new("spotify_companion_widget=debug,info")
        } else {
            EnvFilter::new("spotify_companion_widget=info,warn")
        }
    });
    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_target(true),
        )
        .try_init()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(guard)
}

fn rotate_if_needed(log_dir: &Path) -> AppResult<()> {
    let active = log_dir.join(LOG_FILE);
    let size = active
        .metadata()
        .map(|value| value.len())
        .unwrap_or_default();
    if size < MAX_LOG_BYTES {
        return Ok(());
    }
    let oldest = log_dir.join(format!("{LOG_FILE}.{MAX_LOG_FILES}"));
    if oldest.exists() {
        fs::remove_file(oldest)?;
    }
    for index in (1..MAX_LOG_FILES).rev() {
        let source = log_dir.join(format!("{LOG_FILE}.{index}"));
        let destination = log_dir.join(format!("{LOG_FILE}.{}", index + 1));
        if source.exists() {
            fs::rename(source, destination)?;
        }
    }
    fs::rename(active, log_dir.join(format!("{LOG_FILE}.1")))?;
    Ok(())
}
