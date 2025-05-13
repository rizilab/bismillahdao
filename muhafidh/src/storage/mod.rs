pub mod in_memory;
pub mod migration;
pub mod postgres;
pub mod redis;

use std::sync::Arc;

use anyhow::Result;
use postgres::PostgresClient;
use redis::RedisClient;
use tracing::info;
use tracing::instrument;

use crate::config::Config;
use crate::storage::migration::Migrator;
use crate::storage::postgres::make_postgres_client;
use crate::storage::redis::make_redis_client;

#[derive(Debug, Clone)]
pub struct StorageEngine {
  pub postgres: Arc<PostgresClient>,
  pub redis:    Arc<RedisClient>,
}

impl StorageEngine {
  pub fn new(
    postgres: Arc<PostgresClient>,
    redis: Arc<RedisClient>,
  ) -> Self {
    Self { postgres, redis }
  }

  // Run migrations on the storage engine
  pub async fn run_migrations(&self) -> Result<()> {
    let migrator = Migrator::new(self.postgres.pool.clone());
    migrator.run_migrations().await?;
    Ok(())
  }
}

#[instrument(level = "info", skip(config))]
pub async fn make_storage_engine(
  engine_name: &str,
  config: &Config,
) -> Result<StorageEngine> {
  let postgres = make_postgres_client(engine_name, &config.storage_postgres).await?;
  info!("postgres::created");
  let redis = make_redis_client(engine_name, &config.storage_redis).await?;
  info!("redis::created");

  let storage = StorageEngine::new(postgres, redis);

  // Run migrations
  storage.run_migrations().await?;
  info!("migrations::completed");

  Ok(storage)
}
