use serde_json;
use serde::Serialize;
use crate::Result;
use crate::storage::redis::RedisPool;
use bb8_redis::redis;
use crate::err_with_loc;
use crate::RedisClientError;
use bb8_redis::RedisConnectionManager;
use bb8::PooledConnection;
use redis::aio::PubSub;
use tracing::debug;
use tracing::error;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::fmt;

#[derive(Clone)]
pub struct TokenMetadataQueue {
  pub pool: RedisPool,
  pub pubsub: Arc<RwLock<PubSub>>,
}

impl fmt::Debug for TokenMetadataQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenMetadataQueue")
            .field("pool", &self.pool)
            .field("pubsub", &format_args!("Arc<RwLock<PubSub@{:p}>>", Arc::as_ptr(&self.pubsub)))
            .finish()
    }
}

impl TokenMetadataQueue {
    pub fn new(pool: RedisPool, pubsub: Arc<RwLock<PubSub>>) -> Self {
        Self { pool, pubsub }
      }
      
     pub async fn get_connection(&self) -> Result<PooledConnection<'_, RedisConnectionManager>> {
        self.pool
            .get()
            .await
            .map_err(|e| {
                error!("failed_to_get_redis_connection: {}", e);
                err_with_loc!(RedisClientError::GetConnectionError(e))
              })
      }
        
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
        
    debug!("redis_publish_done::{}", key);
    Ok(())
  }

//   pub async fn subscribe(&self, key: &str) -> Result<()> {
//     let mut conn = self.pubsub;
//     conn.subscribe(key).await?;
    
//     let msg_stream = conn.on_message();

    
//   }

}