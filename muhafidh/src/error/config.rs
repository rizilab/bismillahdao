use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to open config file: {0}")]
    OpenFileError(String),

    #[error("Failed to parse config file: {0}")]
    ParseError(String),

    #[error("Failed to load config file: {0}")]
    LoadError(String),

    #[error("Unexpected error: {0}")]
    Other(String),
}
