use tracing::{info, warn, error, debug};
use tracing_subscriber::{fmt, EnvFilter, prelude::*};
use anyhow::Result;
use crate::config::settings::LoggingConfig;

pub struct Logger {
    name: String,
}

impl Logger {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn info(&self, message: &str) {
        info!("[{}] {}", self.name, message);
    }

    pub fn warn(&self, message: &str) {
        warn!("[{}] {}", self.name, message);
    }

    pub fn error(&self, message: &str) {
        error!("[{}] {}", self.name, message);
    }

    pub fn debug(&self, message: &str) {
        debug!("[{}] {}", self.name, message);
    }
}

pub fn init(config: &LoggingConfig) -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer());

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| anyhow::anyhow!("Failed to set global subscriber: {}", e))?;

    Ok(())
}
