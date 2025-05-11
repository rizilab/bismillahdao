use std::sync::Arc;

use tracing::info;
use tracing::error;
use crate::err_with_loc;
use crate::config::load_config;
use crate::config::Config;
use crate::Result;
use crate::setup_tracing;
use crate::storage::make_storage_engine;
use crate::storage::StorageEngine;
use crate::storage::postgres::PostgresStorage;
use crate::pipeline::subscriber::pumpfun::make_pumpfun_subscriber_pipeline;
use crate::handler::token::metadata::TokenHandlerMetadataOperator;
use crate::handler::shutdown::ShutdownSignal;
use crate::error::EngineError;

#[derive(Clone)]
pub struct Raqib {
  pub config: Config,
  pub db:     Arc<StorageEngine>,
  pub token_handler: Arc<TokenHandlerMetadataOperator>,
}

impl Raqib {
  pub async fn run() -> Result<()> {
    info!("Starting Raqib (رقيب): The Watchful Guardian");

    setup_tracing("raqib");
    info!("setup_tracing");

    let config = load_config("Config.toml")?;

    let db_engine = Arc::new(make_storage_engine("raqib", &config).await?);
    info!("db_engine::created");
    
    let shutdown_signal = ShutdownSignal::new();
    
    db_engine.postgres.db.health_check().await?;
    info!("postgres::health_check::ok");
    db_engine.postgres.db.initialize().await?;
    info!("postgres::initialize::ok");

    let token_handler = Arc::new(TokenHandlerMetadataOperator::new(
        db_engine.clone(), shutdown_signal.clone()));

    let raqib = Raqib { config, db: db_engine, token_handler: token_handler.clone() };
    

    let mut pipeline = make_pumpfun_subscriber_pipeline(raqib)?;
    
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);
    
    tokio::select! {
        result = pipeline.run() => {
            token_handler.shutdown();
            let _ = shutdown_tx.send(()).await;
            result.map_err(|e| {
                error!("pipeline_error: {}", e);
                err_with_loc!(EngineError::EngineError(e))
              })?
        },
        _ = tokio::signal::ctrl_c() => {
            info!("termination_signal::graceful_shutdown");
            
            token_handler.shutdown();
            let _ = shutdown_tx.send(()).await;
        },
        _ = shutdown_rx.recv() => {
            info!("shutdown_signal::other_component");
            
            token_handler.shutdown();
        }
    }

    info!("all_component_shutdown");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    info!("raqib::shutdown");
    
    Ok(())
  }
}
