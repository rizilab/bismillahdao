use std::sync::Arc;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::redis::Redis;
use redis::{Client, Connection, AsyncCommands};
use crate::Result;

/// Redis test environment
pub struct TestRedis {
    pub client: Arc<Client>,
    pub container: ContainerAsync<Redis>,
}

impl TestRedis {
    /// Create a new test Redis instance
    pub async fn new() -> Result<Self> {
        let container = Redis::default()
            .start()
            .await
            .map_err(|e| crate::error::StorageError::ConnectionError(format!("Failed to start test Redis: {}", e)))?;

        let host_port = container.get_host_port_ipv4(6379).await
            .map_err(|e| crate::error::StorageError::ConnectionError(format!("Failed to get Redis port: {}", e)))?;

        let redis_url = format!("redis://127.0.0.1:{}", host_port);
        let client = Client::open(redis_url)
            .map_err(|e| crate::error::StorageError::ConnectionError(format!("Failed to create Redis client: {}", e)))?;

        Ok(Self {
            client: Arc::new(client),
            container,
        })
    }

    /// Get a new async connection
    pub async fn get_async_connection(&self) -> Result<redis::aio::Connection> {
        self.client
            .get_async_connection()
            .await
            .map_err(|e| crate::error::StorageError::ConnectionError(format!("Failed to get Redis connection: {}", e)))
    }

    /// Clean all data
    pub async fn flush_all(&self) -> Result<()> {
        let mut conn = self.get_async_connection().await?;
        conn.flushall()
            .await
            .map_err(|e| crate::error::StorageError::QueryError(format!("Failed to flush Redis: {}", e)))?;
        Ok(())
    }

    /// Set up test data patterns
    pub async fn setup_test_queues(&self) -> Result<()> {
        let mut conn = self.get_async_connection().await?;
        
        // Set up queue keys
        let queue_keys = vec![
            "failed_accounts",
            "unprocessed_accounts",
            "test_queue_1",
            "test_queue_2",
        ];

        for key in queue_keys {
            conn.del(key)
                .await
                .map_err(|e| crate::error::StorageError::QueryError(format!("Failed to delete key {}: {}", key, e)))?;
        }

        Ok(())
    }

    /// Get Redis info for monitoring
    pub async fn get_info(&self) -> Result<String> {
        let mut conn = self.get_async_connection().await?;
        conn.info()
            .await
            .map_err(|e| crate::error::StorageError::QueryError(format!("Failed to get Redis info: {}", e)))
    }
} 