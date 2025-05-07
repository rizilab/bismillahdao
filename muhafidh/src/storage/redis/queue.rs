use serde_json;
use serde::Serialize;
use crate::Result;
use crate::storage::redis::RedisPool;
use bb8_redis::redis;
use crate::err_with_loc;
use crate::RedisClientError;
use bb8_redis::RedisConnectionManager;
use bb8::PooledConnection;
use crate::storage::redis::RedisStorage;
use tracing::info;
use tracing::error;

#[derive(Debug, Clone)]
pub struct TokenMetadataQueue {
  pub pool: RedisPool,
}

#[async_trait::async_trait]
impl RedisStorage for TokenMetadataQueue {
  fn new(pool: RedisPool) -> Self {
    Self { pool }
  }
  
  async fn get_connection(&self) -> Result<PooledConnection<'_, RedisConnectionManager>> {
    self.pool
        .get()
        .await
        .map_err(|e| {
            error!("failed_to_get_redis_connection: {}", e);
            err_with_loc!(RedisClientError::GetConnectionError(e))
          })
  }
}

impl TokenMetadataQueue {
  pub async fn publish<T: Serialize + Send>(&self, key: &str, value: &T) -> Result<()> {
    let mut conn = self.get_connection().await?;
    
    let token_json = serde_json::to_string(value)?;
    
    let _: () = redis::cmd("PUBLISH")
        .arg(key)
        .arg(token_json)
        .query_async(&mut *conn)
        .await
        .map_err(|e| {
            error!("redis_publish_failed: {}", e);
            err_with_loc!(RedisClientError::RedisError(e))
        })?;
        
    info!("redis_publish_done::{}", key);
    Ok(())
}
}