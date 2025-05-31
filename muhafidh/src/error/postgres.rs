use thiserror::Error;

#[derive(Error, Debug)]
pub enum PostgresClientError {
    #[error("Database connection error: {0}")]
    ConnectionError(#[from] tokio_postgres::Error),

    #[error("Pool initialization error: {0}")]
    PoolError(#[from] bb8::RunError<tokio_postgres::Error>),

    #[error("TLS configuration error: {0}")]
    TlsError(String),

    #[error("Database query error: {0}")]
    QueryError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Unexpected error: {0}")]
    Other(String),
}
