pub mod kv;
pub mod queue;
pub mod model;
use std::sync::Arc;

use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use tracing::info;
use tracing::instrument;
use tokio::sync::RwLock;
use crate::config::StorageRedisConfig;
use crate::error::Result;

pub use kv::TokenMetadataKv;
pub use queue::TokenMetadataQueue;

pub type RedisPool = Pool<RedisConnectionManager>;

#[derive(Debug,Clone)]
pub struct RedisClient {
  pub kv: Arc<TokenMetadataKv>,
  pub queue: Arc<TokenMetadataQueue>,
}

#[instrument(level = "debug", skip(config))]
pub async fn make_redis_client(engine_name: &str, config: &StorageRedisConfig) -> Result<Arc<RedisClient>> {     
    let redis_url = format!("redis://{}:{}/?protocol=resp3", config.host, config.port);
    let client = redis::Client::open(redis_url.clone())?;
    let pubsub = client.get_async_pubsub().await?;
    let pubsub = Arc::new(RwLock::new(pubsub));
    let manager = RedisConnectionManager::new(redis_url)?;
    let pool = bb8::Pool::builder().max_size(config.pool_size).build(manager).await?;
    info!("redis::connection_established");

    let kv = Arc::new(TokenMetadataKv::new(pool.clone()));
    let queue = Arc::new(TokenMetadataQueue::new(pool, pubsub));

    Ok(Arc::new(RedisClient { kv, queue }))
}
