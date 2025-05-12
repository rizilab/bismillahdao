pub mod db;
pub mod graph;
pub mod model;
pub mod time_series;

use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use native_tls::Certificate;
use native_tls::Identity;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use tokio_postgres::Config;
use tracing::error;
use tracing::info;
use tracing::instrument;

use crate::config::StoragePostgresConfig;
use crate::err_with_loc;
use crate::error::PostgresClientError;
use crate::error::Result;
use crate::storage::postgres::db::TokenMetadataDb;

#[async_trait::async_trait]
pub trait PostgresStorage {
  fn new(pg_pool: Arc<PostgresPool>) -> Self
  where
    Self: Sized;
  async fn health_check(&self) -> Result<()>;
  async fn initialize(&self) -> Result<()>;
}

pub type PostgresPool = Pool<PostgresConnectionManager<MakeTlsConnector>>;

#[derive(Debug, Clone)]
pub struct PostgresClient {
  pub pool: Arc<PostgresPool>,
  pub db:   Arc<TokenMetadataDb>,
  // TODO: add graph which will be used by baseer
  // pub graph: Arc<AddressRelationsGraphDb>,
  // TODO: add time series which will be used by siraaj
  // pub time_series: Arc<TokenPriceActivityDb>,
}

// this file is for normal postgres db
#[instrument(level = "debug", skip(config))]
pub async fn make_postgres_client(
  engine_name: &str,
  config: &StoragePostgresConfig,
) -> Result<Arc<PostgresClient>> {
  let mut db_config = Config::new();
  db_config
    .user(&config.user)
    .password(&config.password)
    .host(&config.host)
    .port(config.port)
    .dbname(&config.db_name);

  let mut ca_file = File::open(config.tls.ca_path.clone()).map_err(|e| {
    error!("failed_to_open_root_ca_file: {}", e);
    err_with_loc!(PostgresClientError::TlsError(format!("failed_to_open_root_ca_file: {}", e)))
  })?;

  let mut ca_data = vec![];
  ca_file.read_to_end(&mut ca_data).map_err(|e| {
    error!("failed_to_read_root_ca_file: {}", e);
    err_with_loc!(PostgresClientError::TlsError(format!("failed_to_read_root_ca_file: {}", e)))
  })?;

  let certificate = Certificate::from_pem(&ca_data).map_err(|e| {
    error!("failed_to_parse_root_ca_file: {}", e);
    err_with_loc!(PostgresClientError::TlsError(format!("failed_to_parse_root_ca_file: {}", e)))
  })?;

  let mut identity_file = File::open(config.tls.client_identity_path.clone()).map_err(|e| {
    error!("failed_to_open_identity_file: {}", e);
    err_with_loc!(PostgresClientError::TlsError(format!("failed_to_open_identity_file: {}", e)))
  })?;

  let mut identity_data = vec![];
  identity_file.read_to_end(&mut identity_data).map_err(|e| {
    error!("failed_to_read_identity_file: {}", e);
    err_with_loc!(PostgresClientError::TlsError(format!("failed_to_read_identity_file: {}", e)))
  })?;

  let identity = Identity::from_pkcs12(&identity_data, "").map_err(|e: native_tls::Error| {
    error!("invalid_identity_file: {}", e);
    err_with_loc!(PostgresClientError::TlsError(format!("invalid_identity_file: {}", e)))
  })?;

  let tls = TlsConnector::builder()
    .add_root_certificate(certificate)
    .identity(identity)
    .build()
    .map_err(|e| {
      error!("failed_to_build_tls_connector: {}", e);
      err_with_loc!(PostgresClientError::TlsError(format!("failed_to_build_tls_connector: {}", e)))
    })?;

  let connector = MakeTlsConnector::new(tls);

  let mgr = PostgresConnectionManager::new(db_config, connector);

  let pool = Pool::builder().max_size(config.pool_size).build(mgr).await.map_err(|e| {
    error!("failed_to_build_pool: {}", e);
    err_with_loc!(PostgresClientError::PoolError(bb8::RunError::User(e)))
  })?;

  let pool = Arc::new(pool);

  info!("postgres::connection_established");

  let db = Arc::new(TokenMetadataDb::new(pool.clone()));

  Ok(Arc::new(PostgresClient { pool, db }))
}
