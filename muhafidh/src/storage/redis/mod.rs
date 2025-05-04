pub mod kv;
pub mod queue;
use std::sync::Arc;

use bb8::Pool;
use bb8_redis::bb8::PooledConnection;
use bb8_redis::bb8::{self};
use bb8_redis::redis::cmd;
use bb8_redis::redis::pipe;
use bb8_redis::RedisConnectionManager;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::info;
use tracing::instrument;

use crate::config::StorageRedisConfig;
use crate::error::Result;

pub type RedisPool = Pool<RedisConnectionManager>;

#[derive(Debug, Clone)]
pub struct RedisClient {
  pool: RedisPool,
}

#[instrument(level = "debug", skip(config))]
pub async fn make_redis_client(engine_name: &str, config: &StorageRedisConfig) -> Result<Arc<RedisClient>> {     
    let redis_url = format!("redis://{}:{}", config.host, config.port);
    let manager = RedisConnectionManager::new(redis_url)?;
    let pool = bb8::Pool::builder().max_size(config.pool_size).build(manager).await?;
    info!("redis::connection_established");
    Ok(Arc::new(RedisClient { pool }))
}
