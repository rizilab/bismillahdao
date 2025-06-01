pub mod creator;
pub mod log;
pub mod rpc;
pub mod storage;
pub mod discord;

use std::path::Path;

use serde::Deserialize;
use serde::Serialize;
use toml;

pub use creator::CreatorAnalyzerConfig;
pub use log::LoggingConfig;
pub use rpc::RpcConfig;
pub use rpc::RpcProviderConfig;
pub use rpc::RpcProviderRole;
pub use storage::StoragePostgresConfig;
pub use storage::StorageRedisConfig;
pub use discord::DiscordConfig;
pub use discord::DiscordChannel;
pub use discord::DiscordChannelConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub storage_postgres: StoragePostgresConfig,
    pub storage_redis: StorageRedisConfig,
    pub rpc: RpcConfig,
    pub creator_analyzer: CreatorAnalyzerConfig,
    pub logging: LoggingConfig,
    pub discord: DiscordConfig,
}

pub async fn load_config(path: impl AsRef<Path>) -> crate::Result<Config> {
    let config_str = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}
