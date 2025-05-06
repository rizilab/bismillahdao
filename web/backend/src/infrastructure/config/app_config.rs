use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiration_hours: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub auth_server: ServerConfig,
    pub api_server: ServerConfig,
    pub landing_server: ServerConfig,
    pub auth: AuthConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        
        let s = Config::builder()
            // Start with default settings
            .set_default("database.url", "postgres://r4gmi:r4gmi@localhost:5432/r4gmi_db")?
            .set_default("auth_server.host", "0.0.0.0")?
            .set_default("auth_server.port", 8080)?
            .set_default("api_server.host", "0.0.0.0")?
            .set_default("api_server.port", 8081)?
            .set_default("landing_server.host", "0.0.0.0")?
            .set_default("landing_server.port", 8082)?
            .set_default("auth.jwt_secret", "super_secret_key_please_change_in_production")?
            .set_default("auth.token_expiration_hours", 24)?
            
            // Add in settings from config file if it exists
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            
            // Add in settings from environment variables with prefix R4GMI_
            // E.g. `R4GMI_DATABASE_URL=foo ./target/app` would set `database.url`
            .add_source(Environment::with_prefix("r4gmi").separator("_"))
            
            .build()?;
            
        // Deserialize configuration
        s.try_deserialize()
    }
} 