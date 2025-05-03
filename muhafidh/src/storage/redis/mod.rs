pub mod kv;
pub mod queue;
use std::sync::Arc;

use anyhow::Result;
use bb8::Pool;
use bb8_redis::bb8::PooledConnection;
use bb8_redis::bb8::{self};
use bb8_redis::redis::cmd;
use bb8_redis::redis::pipe;
use bb8_redis::RedisConnectionManager;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::warn;

use crate::config::StorageRedisConfig;

pub type RedisPool = Arc<Pool<RedisConnectionManager>>;

pub struct RedisClient {
  pool: RedisPool,
}

// pub async fn make_redis_client(engine_name: &str, config: &StorageRedisConfig) -> Result<Arc<RedisClient>,
// RedisClientError> {     let redis_url = format!("redis://{}:{}", config.host, config.port);
//     let client = RedisClient::new(&redis_url).await?;
//     info!("{}::redis_client::connection_established: {}", engine_name, redis_url);
//     Ok(Arc::new(client))
// }
