use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePostgresConfig {
    pub user: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub pool_size: u32,
    pub db_name: String,
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub client_identity_path: String,
    pub ca_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRedisConfig {
    pub host: String,
    pub port: u16,
    pub pool_size: u32,
}
