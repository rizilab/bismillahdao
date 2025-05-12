use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
  #[error("Engine error: {0}")]
  EngineError(#[from] carbon_core::error::Error),
}
