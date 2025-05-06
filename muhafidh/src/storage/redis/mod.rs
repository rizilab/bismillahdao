pub mod kv;
pub mod queue;
use std::sync::Arc;

use bb8::Pool;
use bb8_redis::bb8::PooledConnection;
use bb8_redis::bb8::{self};
use bb8_redis::RedisConnectionManager;
use tracing::info;
use tracing::instrument;

use crate::config::StorageRedisConfig;
use crate::error::Result;

pub use kv::TokenMetadataKv;
pub use queue::TokenMetadataQueue;

pub type RedisPool = Pool<RedisConnectionManager>;

#[async_trait::async_trait]
pub trait RedisStorage {
  fn new(pool: RedisPool) -> Self
  where
    Self: Sized;
    // not sure if we need this
//   async fn health_check(&self) -> Result<()>;
  async fn get_connection(
    &self,
) -> Result<PooledConnection<'_, RedisConnectionManager>>;

}

#[derive(Debug, Clone)]
pub struct RedisClient {
  pub kv: Arc<TokenMetadataKv>,
  pub queue: Arc<TokenMetadataQueue>,
}

#[instrument(level = "debug", skip(config))]
pub async fn make_redis_client(engine_name: &str, config: &StorageRedisConfig) -> Result<Arc<RedisClient>> {     
    let redis_url = format!("redis://{}:{}", config.host, config.port);
    let manager = RedisConnectionManager::new(redis_url)?;
    let pool = bb8::Pool::builder().max_size(config.pool_size).build(manager).await?;
    info!("redis::connection_established");

    let kv = Arc::new(TokenMetadataKv::new(pool.clone()));
    let queue = Arc::new(TokenMetadataQueue::new(pool.clone()));

    Ok(Arc::new(RedisClient { kv, queue }))
}
