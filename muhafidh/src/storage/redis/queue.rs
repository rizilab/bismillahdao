use std::fmt;
use std::sync::Arc;

use bb8::PooledConnection;
use bb8_redis::RedisConnectionManager;
use bb8_redis::redis;
use redis::aio::PubSub;
use serde::Serialize;
use serde_json;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::error;

use crate::RedisClientError;
use crate::Result;
use crate::err_with_loc;
use crate::model::creator::metadata::CreatorMetadata;
use crate::storage::redis::RedisPool;

#[derive(Clone)]
pub struct TokenMetadataQueue {
    pub pool: RedisPool,
    pub pubsub: Arc<RwLock<PubSub>>,
}

impl fmt::Debug for TokenMetadataQueue {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("TokenMetadataQueue")
            .field("pool", &self.pool)
            .field("pubsub", &format_args!("Arc<RwLock<PubSub@{:p}>>", Arc::as_ptr(&self.pubsub)))
            .finish()
    }
}

impl TokenMetadataQueue {
    pub fn new(
        pool: RedisPool,
        pubsub: Arc<RwLock<PubSub>>,
    ) -> Self {
        Self {
            pool,
            pubsub,
        }
    }

    pub async fn get_connection(&self) -> Result<PooledConnection<'_, RedisConnectionManager>> {
        self.pool.get().await.map_err(|e| {
            error!("failed_to_get_redis_connection: {}", e);
            err_with_loc!(RedisClientError::GetConnectionError(e))
        })
    }

    pub async fn publish<T: Serialize + Send>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<()> {
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

    // Add an account to the unprocessed list
    pub async fn add_unprocessed_account(
        &self,
        account: &CreatorMetadata,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let json = serde_json::to_string(account).map_err(|e| {
            error!("serialize_account_failed: {}", e);
            err_with_loc!(RedisClientError::SerializeError(e))
        })?;

        let _: () = redis::cmd("RPUSH")
            .arg("unprocessed_accounts")
            .arg(json)
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                error!("redis_add_unprocessed_account_failed: {}", e);
                err_with_loc!(RedisClientError::RedisError(e))
            })?;

        debug!("redis_add_unprocessed_account_done::account::{}", account.get_analyzed_account().await);
        Ok(())
    }

    // Add an account to the failed list (high priority for retry)
    pub async fn add_failed_account(
        &self,
        failed: &CreatorMetadata,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let json = serde_json::to_string(failed).map_err(|e| {
            error!("serialize_failed_account_failed: {}", e);
            err_with_loc!(RedisClientError::SerializeError(e))
        })?;

        let _: () = redis::cmd("RPUSH")
            .arg("failed_accounts")
            .arg(json)
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                error!("redis_add_failed_account_failed: {}", e);
                err_with_loc!(RedisClientError::RedisError(e))
            })?;

        debug!("redis_add_failed_account_done::account::{}", failed.get_analyzed_account().await);
        Ok(())
    }

    // Get the next account from the failed list
    // We prioritize failed accounts over unprocessed ones for retry
    pub async fn get_next_failed_account(&self) -> Result<Option<CreatorMetadata>> {
        let mut conn = self.get_connection().await?;

        let json: Option<String> = redis::cmd("LPOP")
            .arg("failed_accounts")
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                error!("redis_get_next_failed_account_failed: {}", e);
                err_with_loc!(RedisClientError::RedisError(e))
            })?;

        match json {
            Some(json) => {
                let account = serde_json::from_str(&json).map_err(|e| {
                    error!("deserialize_failed_account_failed: {}", e);
                    err_with_loc!(RedisClientError::DeserializeError(e))
                })?;
                Ok(Some(account))
            },
            None => Ok(None),
        }
    }

    // Get the next account from the unprocessed list
    pub async fn get_next_unprocessed_account(&self) -> Result<Option<CreatorMetadata>> {
        let mut conn = self.get_connection().await?;

        let json: Option<String> = redis::cmd("LPOP")
            .arg("unprocessed_accounts")
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                error!("redis_get_next_unprocessed_account_failed: {}", e);
                err_with_loc!(RedisClientError::RedisError(e))
            })?;

        match json {
            Some(json) => {
                let account = serde_json::from_str(&json).map_err(|e| {
                    error!("deserialize_unprocessed_account_failed: {}", e);
                    err_with_loc!(RedisClientError::DeserializeError(e))
                })?;
                Ok(Some(account))
            },
            None => Ok(None),
        }
    }

    // Get counts of accounts in the pending queues
    pub async fn get_pending_account_counts(&self) -> Result<(usize, usize)> {
        let mut conn = self.get_connection().await?;

        // Get failed count
        let failed_count: usize = redis::cmd("LLEN")
            .arg("failed_accounts")
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                error!("redis_get_failed_account_count_failed: {}", e);
                err_with_loc!(RedisClientError::RedisError(e))
            })?;

        // Get unprocessed count
        let unprocessed_count: usize = redis::cmd("LLEN")
            .arg("unprocessed_accounts")
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                error!("redis_get_unprocessed_account_count_failed: {}", e);
                err_with_loc!(RedisClientError::RedisError(e))
            })?;

        Ok((failed_count, unprocessed_count))
    }
}
