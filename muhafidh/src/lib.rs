pub mod config;
pub mod constants;
pub mod engine;
pub mod error;
pub mod handler;
pub mod metric;
pub mod model;
pub mod pipeline;
pub mod storage;
pub mod tracing;
pub mod utils;

pub use engine::*;
pub use error::*;

pub use error::{HandlerError, PipelineError, RpcError, StorageError};

// Test utilities - only compiled during testing
#[cfg(test)]
pub mod test_utils {
    pub mod fixtures;
    pub mod mocks;
    pub mod helpers;
    pub mod assertions;
}

// Integration test helpers - available for integration tests
#[cfg(any(test, feature = "testing"))]
pub mod testing {
    pub mod database;
    pub mod redis;
    pub mod rpc_mock;
    pub mod token_factory;
}

pub use error::Result;
