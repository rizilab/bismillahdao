use std::path::Path;

use serde::Deserialize;
use serde::Serialize;
use toml;

use crate::rpc::config::RpcConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub storage_postgres: StoragePostgresConfig,
    pub storage_redis: StorageRedisConfig,
    pub rpc: RpcConfig,
    pub creator_analyzer: CreatorAnalyzerConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePostgresConfig {
    pub user: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub pool_size: u32,
    pub db_name: String,
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub client_identity_path: String,
    pub ca_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRedisConfig {
    pub host: String,
    pub port: u16,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorAnalyzerConfig {
    pub max_depth: usize,
    pub max_concurrent_requests: usize,
    pub max_signatures_to_check: usize,
    pub base_retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
    pub max_retries: usize,
}

impl Default for CreatorAnalyzerConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_concurrent_requests: 20,
            max_signatures_to_check: 250,
            base_retry_delay_ms: 500,
            max_retry_delay_ms: 30_000,
            max_retries: 5,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    // Directory where logs will be stored
    pub directory: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            directory: Some(".logs".to_string()),
        }
    }
}

pub fn load_config(path: impl AsRef<Path>) -> crate::Result<Config> {
    let config_str = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}
