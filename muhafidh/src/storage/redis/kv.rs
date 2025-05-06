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
use crate::err_with_loc;
use crate::redis::RedisClientError;
use crate::model::token::TokenMetadata;
use bb8_redis::redis;
use bb8_redis::RedisConnectionManager;
use bb8::PooledConnection;
use crate::storage::redis::RedisStorage;

use tracing::error;
#[derive(Debug, Clone)]
pub struct TokenMetadataKv {
  pub pool: RedisPool,
}

#[async_trait::async_trait]
impl RedisStorage for TokenMetadataKv {
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

impl TokenMetadataKv {
  pub async fn get_token_metadata(&self, mint: &str) -> Result<Option<TokenMetadata>> {
    let mut conn = self.get_connection().await?;
        
        let token_json: Option<String> = redis::cmd("GET")
            .arg(mint)
            .query_async::<Option<String>>(&mut *conn)
            .await
            .map_err(|e| {
                error!("get_token_metadata_failed: {}", e);
                err_with_loc!(RedisClientError::RedisError(e))
            })?;
            
        if let Some(json) = token_json {
            match serde_json::from_str::<TokenMetadata>(&json) {
                Ok(token) => Ok(Some(token)),
                Err(e) => {
                    error!("deserialize_token_metadata_failed: {}", e);
                    Err(err_with_loc!(RedisClientError::DeserializeError(e)))
                }
            }
        } else {
            Ok(None)
        }
  }
  
  pub async fn set_token_metadata(&self, mint: &str, token_metadata: &TokenMetadata) -> Result<()> {
    let mut conn = self.get_connection().await?;
    let json = serde_json::to_string(token_metadata).map_err(|e| {
        error!("serialize_token_metadata_failed: {}", e); // <=== please see the format
        err_with_loc!(RedisClientError::SerializeError(e))
      })?;
    redis::cmd("SET")
        .arg(mint)
        .arg(json)
        .arg("EX")
        .arg(7200)
        .query_async::<()>(&mut *conn)
        .await
        .map_err(|e| {
            error!("set_token_metadata_failed: {}", e); // <=== please see the format
            err_with_loc!(RedisClientError::RedisError(e))
          })?;
    
    Ok(())
  }
}
