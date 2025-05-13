pub mod in_memory;
pub mod migration;
pub mod postgres;
pub mod redis;

use std::sync::Arc;

use anyhow::Result;
use postgres::PostgresClient;
use redis::RedisClient;
use tracing::error;
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

  // Check if the database schema is at the expected version
  pub async fn check_schema_version(&self) -> Result<bool> {
    let migrator = Migrator::new(self.postgres.pool.clone());
    migrator.check_schema_version().await
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

  // Check schema version instead of running migrations
  let schema_valid = storage.check_schema_version().await?;
  if !schema_valid {
    error!("Database schema version mismatch. Please run migrations before starting services.");
    // You could choose to panic here or continue with a warning
    // panic!("Database schema version mismatch. Please run migrations before starting services.");
  }

  info!("schema_version::checked");
  Ok(storage)
}

// Create a special function for the migration CLI tool
#[instrument(level = "info", skip(config))]
pub async fn run_database_migrations(
  engine_name: &str,
  config: &Config,
) -> Result<()> {
  let postgres = make_postgres_client(engine_name, &config.storage_postgres).await?;
  info!("postgres::created_for_migration");

  let migrator = Migrator::new(postgres.pool.clone());
  info!("Starting database migrations...");
  migrator.run_migrations().await?;
  info!("Database migrations completed successfully");

  Ok(())
}
