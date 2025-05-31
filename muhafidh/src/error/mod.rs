pub mod config;
pub mod engine;
pub mod handler;
pub mod postgres;
pub mod redis;

pub use anyhow::Context;
pub use anyhow::Error;
pub use anyhow::Result;
pub use anyhow::anyhow;
pub use engine::EngineError;
pub use handler::HandlerError;
pub use postgres::PostgresClientError;
pub use redis::RedisClientError;

// For consistent error handling with location info
#[macro_export]
macro_rules! err_with_loc {
    ($err:expr) => {
        anyhow::anyhow!($err).context(format!("at {}:{}", file!(), line!()))
    };
}
