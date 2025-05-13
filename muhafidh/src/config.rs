use std::fs::File;
use std::io::Read;

use serde::Deserialize;
use toml;
use tracing::error;

use crate::err_with_loc;
use crate::error::config::ConfigError;
use crate::error::Result;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
  pub storage_postgres: StoragePostgresConfig,
  pub storage_redis:    StorageRedisConfig,
  pub rpc:              RpcConfig,
  pub creator_analyzer: CreatorAnalyzerConfig,
  pub logging:          LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoragePostgresConfig {
  pub user:      String,
  pub password:  String,
  pub port:      u16,
  pub host:      String,
  pub pool_size: u32,
  pub db_name:   String,
  pub tls:       TlsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TlsConfig {
  pub client_identity_path: String,
  pub ca_path:              String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageRedisConfig {
  pub host:      String,
  pub port:      u16,
  pub pool_size: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcConfig {
  pub http_url:     String,
  pub ws_url:       String,
  pub http_api_key: String,
  pub ws_api_key:   String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatorAnalyzerConfig {
  pub max_depth: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
  // Directory where logs will be stored
  pub directory: Option<String>,
}

impl Default for LoggingConfig {
  fn default() -> Self { Self { directory: Some(".logs".to_string()) } }
}

impl RpcConfig {
  pub fn get_http_url(&self) -> String {
    if !self.http_api_key.is_empty() {
      format!("https://{}/{}", self.http_url, self.http_api_key)
    } else {
      format!("https://{}/", self.http_url)
    }
  }

  pub fn get_ws_url(&self) -> String {
    if !self.ws_api_key.is_empty() {
      format!("wss://{}/{}", self.ws_url, self.ws_api_key)
    } else {
      format!("wss://{}/", self.ws_url)
    }
  }
}

pub fn load_config(file_path: &str) -> Result<Config> {
  let mut file = File::open(file_path).map_err(|e| {
    error!("failed_to_open_config_file: {}", e);
    err_with_loc!(ConfigError::OpenFileError(format!("failed_to_open_config_file: {}", e)))
  })?;
  let mut contents = String::new();

  let _ = file.read_to_string(&mut contents).map_err(|e| {
    error!("failed_to_load_config_file: {}", e);
    err_with_loc!(ConfigError::LoadError(format!("failed_to_load_config_file: {}", e)))
  })?;

  let config: Config = toml::de::from_str(&contents).map_err(|e| {
    error!("failed_to_parse_config_file: {}", e);
    err_with_loc!(ConfigError::ParseError(format!("failed_to_parse_config_file: {}", e)))
  })?;

  Ok(config)
}
