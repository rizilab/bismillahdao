use thiserror::Error;

#[derive(Debug, Error)]
pub enum RedisClientError {
    #[error("[Redis] Failed to create connection manager: {0}")]
    CreateConnectionManagerError(#[from] bb8_redis::redis::RedisError),
    #[error("[Redis] Failed to connect: {0}")]
    GetConnectionError(#[from] bb8::RunError<bb8_redis::redis::RedisError>),
    #[error("[Redis] Failed to serialize: {0}")]
    SerializeError(#[from] serde_json::Error),
    #[error("[Redis] Failed to deserialize: {0}")]
    DeserializeError(serde_json::Error),
    #[error("[Redis] Redis error: {0}")]
    RedisError(bb8_redis::redis::RedisError),
    #[error("[Redis] Key not found: {0}")]
    KeyNotFound(String),
    #[error("[Redis] Subscribe error: {0}")]
    SubscribeError(String),
}
