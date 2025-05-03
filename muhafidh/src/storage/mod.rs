pub mod in_memory;
pub mod postgres;
pub mod redis;

use std::sync::Arc;

use anyhow::Result;
use postgres::PostgresClient;
use redis::RedisClient;
use tracing::info;
use tracing::instrument;

use crate::config::Config;
use crate::storage::postgres::make_postgres_client;
// use crate::storage::redis::make_redis_client;

#[derive(Debug, Clone)]
pub struct StorageEngine {
  pub postgres: Arc<PostgresClient>,
  // pub redis: Arc<RedisClient>,
}

impl StorageEngine {
  pub fn new(postgres: Arc<PostgresClient>) -> Self { Self { postgres } }
}

#[instrument(level = "info", skip(config))]
pub async fn make_storage_engine(
  engine_name: &str,
  config: &Config,
) -> Result<StorageEngine> {
  let postgres = make_postgres_client(engine_name, &config.storage_postgres).await?;
  info!("{}::run::postgres::created", engine_name);
  // let redis = make_redis_client(engine_name, config).await?;
  // debug!("{}::run::redis::created::{}", engine_name, redis);

  Ok(StorageEngine::new(postgres))
}
