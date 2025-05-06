use thiserror::Error;

#[derive(Error, Debug)]
pub enum HandlerError {
  #[error("Failed to send token to metadata handler: {0}")]
  SendTokenHandlerError(String),
}