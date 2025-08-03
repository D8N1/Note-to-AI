pub mod audio;
pub mod config;
pub mod crypto;
pub mod identity;
pub mod logger;
pub mod scheduler;
pub mod signal;
pub mod swarm;
pub mod vault;

pub use config::Settings;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>; 