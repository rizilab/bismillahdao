use std::sync::Arc;

use tracing::info;

use crate::config::load_config;
use crate::config::Config;
use crate::error::Result;
use crate::setup_tracing;
use crate::storage::make_storage_engine;
use crate::storage::StorageEngine;
use crate::subscriber::pumpfun::make_pumpfun_subscriber_pipeline;

#[derive(Debug, Clone)]
pub struct Raqib {
  pub config: Config,
  pub db:     Arc<StorageEngine>,
}

impl Raqib {
  pub async fn run() -> Result<()> {
    info!("Starting Raqib (رقيب): The Watchful Guardian");

    setup_tracing("raqib");
    info!("raqib::run::setup_tracing");

    let config = load_config("Config.toml")?;
    info!("raqib::run::config loaded");

    let db_engine = make_storage_engine("raqib", &config).await?;
    info!("raqib::run::db_engine::created");

    let raqib = Raqib { config, db: Arc::new(db_engine) };

    let mut pipeline = make_pumpfun_subscriber_pipeline(raqib)?;
    pipeline.run().await?;

    Ok(())
  }
}
