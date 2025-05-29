use thiserror::Error;

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("Failed to send token to metadata handler: {0}")]
    SendTokenHandlerError(String),
    #[error("Failed to send creator to metadata handler: {0}")]
    SendCreatorHandlerError(String),
    #[error("Failed to analyze CEX: {0}")]
    CexAnalysisError(String),
    #[error("Failed to analyze RPC: {0}")]
    RpcError(String),
    #[error("Failed to process account: {0}")]
    AccountProcessingError(String),
    #[error("Failed to create pipeline: {0}")]
    PipelineCreationError(String),
    #[error("Failed to query Redis: {0}")]
    RedisQueryError(String),
}
