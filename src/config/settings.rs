use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub logging: LoggingConfig,
    pub vault: VaultConfig,
    pub ai: AIConfig,
    pub crypto: CryptoConfig,
    pub swarm: SwarmConfig,
    pub signal: SignalConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub path: PathBuf,
    pub auto_sync: bool,
    pub index_interval: u64,
    pub cache_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub model_path: PathBuf,
    pub embeddings_path: PathBuf,
    pub context_window: usize,
    pub model_registry: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    pub pq_enabled: bool,
    pub key_path: PathBuf,
    pub hybrid_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    pub bootstrap_nodes: Vec<String>,
    pub private_key_path: PathBuf,
    pub swarm_key_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    pub enabled: bool,
    pub phone_number: Option<String>,
    pub device_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub encrypted: bool,
}

impl Settings {
    pub fn load(path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix("NOTE_TO_AI"))
            .build()?;

        settings.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_settings_serialization() {
        let settings = Settings {
            logging: LoggingConfig {
                level: "info".to_string(),
                file: Some(PathBuf::from("test.log")),
            },
            vault: VaultConfig {
                path: PathBuf::from("./vault"),
                auto_sync: true,
                index_interval: 300,
                cache_size: 1000,
            },
            ai: AIConfig {
                model_path: PathBuf::from("./models"),
                embeddings_path: PathBuf::from("./models/embeddings"),
                context_window: 4096,
                model_registry: PathBuf::from("./models/registry.toml"),
            },
            crypto: CryptoConfig {
                pq_enabled: true,
                key_path: PathBuf::from("./keys"),
                hybrid_mode: true,
            },
            swarm: SwarmConfig {
                bootstrap_nodes: vec![],
                private_key_path: PathBuf::from("./swarm.key"),
                swarm_key_path: PathBuf::from("./swarm.key"),
            },
            signal: SignalConfig {
                enabled: false,
                phone_number: None,
                device_id: Some(1),
            },
            database: DatabaseConfig {
                path: PathBuf::from("./db/notetoai.db"),
                encrypted: true,
            },
        };

        let serialized = serde_json::to_string(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(settings.logging.level, deserialized.logging.level);
        assert_eq!(settings.vault.auto_sync, deserialized.vault.auto_sync);
        assert_eq!(settings.ai.context_window, deserialized.ai.context_window);
        assert_eq!(settings.crypto.pq_enabled, deserialized.crypto.pq_enabled);
    }

    #[test]
    fn test_logging_config() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            file: Some(PathBuf::from("debug.log")),
        };
        
        assert_eq!(config.level, "debug");
        assert!(config.file.is_some());
    }

    #[test]
    fn test_vault_config() {
        let config = VaultConfig {
            path: PathBuf::from("./test-vault"),
            auto_sync: true,
            index_interval: 600,
            cache_size: 2000,
        };
        
        assert_eq!(config.auto_sync, true);
        assert_eq!(config.index_interval, 600);
        assert_eq!(config.cache_size, 2000);
    }
}
