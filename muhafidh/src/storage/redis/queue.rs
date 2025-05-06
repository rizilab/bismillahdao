// pub async fn make_message_queue(engine_name: &str) -> Result<Arc<RedisClient>, RedisClientError> {
//     let message_queue_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
//     let client = RedisClient::new(&message_queue_url).await?;
//     info!("{}::message_queue::connection_established: {}", engine_name, message_queue_url);
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
use serde_json;
use crate::Result;
use crate::storage::redis::RedisPool;
use bb8_redis::redis;
use crate::err_with_loc;
use crate::RedisClientError;
use crate::model::token::TokenMetadata;
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
  pub async fn publish_new_token_metadata(&self, token: &TokenMetadata) -> Result<()> {
    let mut conn = self.get_connection().await?;
    
    let token_json = serde_json::to_string(token)?;
    
    redis::cmd("PUBLISH")
        .arg("new_token_created")
        .arg(token_json)
        .query_async::<()>(&mut *conn)
        .await
        .map_err(|e| {
            error!("publish_new_token_created_failed: {}", e);
            err_with_loc!(RedisClientError::RedisError(e))
        })?;
        
    info!("publish_new_token_created::{}::{}", token.name, token.mint);
    Ok(())
}
}