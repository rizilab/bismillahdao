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

use crate::err_with_loc;
use crate::error::Result;
use crate::error::postgres::PostgresClientError;
use crate::storage::postgres::PostgresPool;
use crate::storage::postgres::PostgresStorage;

#[derive(Debug, Clone)]
pub struct TimeSeriesDb {
    pub pool: Arc<PostgresPool>,
}

#[async_trait::async_trait]
impl PostgresStorage for TimeSeriesDb {
    fn new(pool: Arc<PostgresPool>) -> Self {
        Self {
            pool,
        }
    }

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

    // No need to initialize tables here as this is now handled by migrations
    async fn initialize(&self) -> Result<()> {
        // Just do a health check to ensure the database is available
        self.health_check().await
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

        conn.execute(
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

        conn.execute(
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

        conn.execute(
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
