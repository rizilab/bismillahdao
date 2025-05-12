// this file is for time series data type

// pub async fn make_timeseries_client(engine_name: &str) -> Result<Arc<RedisClient>, RedisClientError> {
//     let timeseries_url = std::env::var("TIMESERIES_URL").unwrap_or_else(|_| "http://127.0.0.1:8000/".to_string());
//     let client = RedisClient::new(&timeseries_url).await?;
//     info!("{}::timeseries_client::connection_established: {}", engine_name, timeseries_url);
//     Ok(Arc::new(client))
// }

// pub async fn make_kv_store() -> Result<Arc<RedisKVStore>> {
//     match is_local() {
//         true => {
//             let kv_store = RedisKVStore::new("redis://localhost:6379").await?;
//             Ok(Arc::new(kv_store))
//         }
//         false => {
//             let kv_store =
//                 RedisKVStore::new(must_get_env("REDIS_URL").as_str()).await?;
//             Ok(Arc::new(kv_store))
//         }
//     }
// }

use std::sync::Arc;

use tracing::error;
use tracing::info;

use crate::err_with_loc;
use crate::error::postgres::PostgresClientError;
use crate::error::Result;
use crate::storage::postgres::PostgresPool;
use crate::storage::postgres::PostgresStorage;

#[derive(Debug, Clone)]
pub struct TimeSeriesDb {
  pub pool: Arc<PostgresPool>,
}

#[async_trait::async_trait]
impl PostgresStorage for TimeSeriesDb {
  fn new(pool: Arc<PostgresPool>) -> Self { Self { pool } }

  async fn health_check(&self) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::PoolError(e))
    })?;

    conn.execute("SELECT 1", &[]).await.map_err(|e| {
      error!("failed_to_health_check: {}", e);
      err_with_loc!(PostgresClientError::QueryError(format!("failed_to_health_check: {}", e)))
    })?;
    Ok(())
  }

  async fn initialize(&self) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_get_client_pool_connection: {}", e)))
    })?;

    // Create time series tables
    conn
      .execute(
        "CREATE TABLE IF NOT EXISTS token_price_history (
                   id SERIAL PRIMARY KEY,
                   mint TEXT NOT NULL,
                   price BIGINT NOT NULL,
                   timestamp BIGINT NOT NULL,
                   created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                   UNIQUE(mint, timestamp)
                 );
        CREATE TABLE IF NOT EXISTS token_volume_history (
                   id SERIAL PRIMARY KEY,
                   mint TEXT NOT NULL,
                   volume BIGINT NOT NULL,
                   timestamp BIGINT NOT NULL,
                   created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                   UNIQUE(mint, timestamp)
                 );
        CREATE TABLE IF NOT EXISTS cex_activity_history (
                   id SERIAL PRIMARY KEY,
                   cex_address TEXT NOT NULL,
                   token_count BIGINT NOT NULL,
                   timestamp BIGINT NOT NULL,
                   created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                   UNIQUE(cex_address, timestamp)
                 );",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_time_series_tables: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_time_series_tables: {}", e)))
      })?;

    // Create indexes for time series tables
    conn
      .execute(
        "CREATE INDEX IF NOT EXISTS idx_token_price_history_mint ON token_price_history(mint);
                 CREATE INDEX IF NOT EXISTS idx_token_price_history_timestamp ON token_price_history(timestamp);
                 CREATE INDEX IF NOT EXISTS idx_token_volume_history_mint ON token_volume_history(mint);
                 CREATE INDEX IF NOT EXISTS idx_token_volume_history_timestamp ON token_volume_history(timestamp);
                 CREATE INDEX IF NOT EXISTS idx_cex_activity_history_cex ON cex_activity_history(cex_address);
                 CREATE INDEX IF NOT EXISTS idx_cex_activity_history_timestamp ON cex_activity_history(timestamp);",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_time_series_indexes: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_time_series_indexes: {}", e)))
      })?;

    info!("Time series database initialized");
    Ok(())
  }
}

impl TimeSeriesDb {
  // Add a token price record
  pub async fn add_token_price(
    &self,
    mint: &str,
    price: u64,
    timestamp: i64,
  ) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::PoolError(e))
    })?;

    conn
      .execute(
        "INSERT INTO token_price_history (mint, price, timestamp)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (mint, timestamp) DO UPDATE SET
                 price = EXCLUDED.price",
        &[&mint, &(price as i64), &timestamp],
      )
      .await
      .map_err(|e| {
        error!("failed_to_add_token_price: {}", e);
        err_with_loc!(PostgresClientError::QueryError(format!("failed_to_add_token_price: {}", e)))
      })?;

    Ok(())
  }

  // Add a token volume record
  pub async fn add_token_volume(
    &self,
    mint: &str,
    volume: u64,
    timestamp: i64,
  ) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::PoolError(e))
    })?;

    conn
      .execute(
        "INSERT INTO token_volume_history (mint, volume, timestamp)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (mint, timestamp) DO UPDATE SET
                 volume = EXCLUDED.volume",
        &[&mint, &(volume as i64), &timestamp],
      )
      .await
      .map_err(|e| {
        error!("failed_to_add_token_volume: {}", e);
        err_with_loc!(PostgresClientError::QueryError(format!("failed_to_add_token_volume: {}", e)))
      })?;

    Ok(())
  }

  // Add a CEX activity record
  pub async fn add_cex_activity(
    &self,
    cex_address: &str,
    token_count: u64,
    timestamp: i64,
  ) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::PoolError(e))
    })?;

    conn
      .execute(
        "INSERT INTO cex_activity_history (cex_address, token_count, timestamp)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (cex_address, timestamp) DO UPDATE SET
                 token_count = cex_activity_history.token_count + EXCLUDED.token_count",
        &[&cex_address, &(token_count as i64), &timestamp],
      )
      .await
      .map_err(|e| {
        error!("failed_to_add_cex_activity: {}", e);
        err_with_loc!(PostgresClientError::QueryError(format!("failed_to_add_cex_activity: {}", e)))
      })?;

    Ok(())
  }
}
