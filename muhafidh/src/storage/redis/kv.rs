use bb8::PooledConnection;
use bb8_redis::redis;
use bb8_redis::RedisConnectionManager;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;
use tracing::debug;
use tracing::error;

use crate::err_with_loc;
use crate::redis::RedisClientError;
use crate::storage::in_memory::creator::CreatorCexConnectionGraph;
use crate::storage::redis::RedisPool;
use crate::Result;

#[derive(Debug, Clone)]
pub struct TokenMetadataKv {
  pub pool: RedisPool,
}

impl TokenMetadataKv {
  pub fn new(pool: RedisPool) -> Self { Self { pool } }

  pub async fn get_connection(&self) -> Result<PooledConnection<'_, RedisConnectionManager>> {
    self.pool.get().await.map_err(|e| {
      error!("failed_to_get_redis_connection: {}", e);
      err_with_loc!(RedisClientError::GetConnectionError(e))
    })
  }

  pub async fn get<T: DeserializeOwned + Send>(
    &self,
    key: &str,
  ) -> Result<Option<T>> {
    let mut conn = self.get_connection().await?;

    let value: Option<String> = redis::cmd("GET").arg(key).query_async(&mut *conn).await.map_err(|e| {
      error!("redis_get_failed: {}", e);
      err_with_loc!(RedisClientError::RedisError(e))
    })?;

    match value {
      Some(json) => {
        serde_json::from_str::<T>(&json)
          .map_err(|e| {
            error!("redis_deserialize_failed: {}", e);
            err_with_loc!(RedisClientError::DeserializeError(e))
          })
          .map(Some)
      },
      None => Ok(None),
    }
  }

  pub async fn set<T: Serialize + Send + Sync>(
    &self,
    key: &str,
    value: &T,
  ) -> Result<()> {
    let mut conn = self.get_connection().await?;
    let json = serde_json::to_string(value).map_err(|e| {
      error!("serialize_failed: {}", e); // <=== please see the format
      err_with_loc!(RedisClientError::SerializeError(e))
    })?;
    let _: () = redis::cmd("SET")
      .arg(key)
      .arg(json)
      .query_async(&mut *conn)
      .await
      .map_err(|e| {
        error!("redis_set_failed: {}", e); // <=== please see the format
        err_with_loc!(RedisClientError::RedisError(e))
      })?;
    debug!("redis_set_done::{}", key);
    Ok(())
  }

  pub async fn set_graph(
    &self,
    key: &str,
    graph: &CreatorCexConnectionGraph,
  ) -> Result<()> {
    let mut conn = self.get_connection().await?;
    let json = serde_json::to_string(graph).map_err(|e| {
      error!("serialize_graph_failed: {}", e);
      err_with_loc!(RedisClientError::SerializeError(e))
    })?;

    let _: () = redis::cmd("SET")
      .arg(key)
      .arg(json)
      .query_async(&mut *conn)
      .await
      .map_err(|e| {
        error!("redis_set_graph_failed: {}", e);
        err_with_loc!(RedisClientError::RedisError(e))
      })?;

    debug!("redis_set_graph_done::{}", key);
    Ok(())
  }

  pub async fn get_graph(
    &self,
    key: &str,
  ) -> Result<Option<CreatorCexConnectionGraph>> {
    let mut conn = self.get_connection().await?;

    let json: Option<String> = redis::cmd("GET").arg(key).query_async(&mut *conn).await.map_err(|e| {
      error!("redis_get_graph_failed: {}", e);
      err_with_loc!(RedisClientError::RedisError(e))
    })?;

    match json {
      Some(json) => {
        let graph = serde_json::from_str(&json).map_err(|e| {
          error!("deserialize_graph_failed: {}", e);
          err_with_loc!(RedisClientError::DeserializeError(e))
        })?;
        Ok(Some(graph))
      },
      None => Ok(None),
    }
  }
}
