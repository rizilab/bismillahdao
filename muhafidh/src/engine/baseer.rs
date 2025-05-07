use crate::config::Config;
use crate::config::load_config;
use crate::storage::redis::model::NewTokenCache;
use crate::Result;
use crate::setup_tracing;
use crate::storage::make_storage_engine;
use crate::storage::StorageEngine;
use crate::handler::shutdown::ShutdownSignal;
use std::sync::Arc;
use futures_util::StreamExt;

use tracing::info;

#[derive(Clone)]
pub struct Baseer {
  pub config: Config,
  pub db:     Arc<StorageEngine>,
}

impl Baseer {
  pub async fn run() -> Result<()> {
    info!("Starting Baseer (بصير): The Analyzer");

    setup_tracing("baseer");
    info!("setup_tracing");

    let config = load_config("Config.toml")?;

    let db_engine = Arc::new(make_storage_engine("baseer", &config).await?);
    info!("db_engine::created");
    
    let shutdown_signal = ShutdownSignal::new();
    
    // db_engine.postgres.graph.health_check().await?;
    // db_engine.postgres.graph.initialize().await?;

    // let token_handler = Arc::new(TokenHandlerMetadataOperator::new(
    //     db_engine.clone(), shutdown_signal.clone()));


    
    let baseer = Baseer { config, db: db_engine.clone() };

    // let mut pipeline = make_account_crawler_pipeline(baseer)?;
    
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);

    let db_engine = db_engine.clone();

    let analyzer_handle = baseer.spawn_token_analyzer(
        db_engine.clone(),
        shutdown_signal.clone(),
        shutdown_tx.clone()
    );
    
    tokio::select! {
        _ = analyzer_handle => {
            shutdown_signal.shutdown();
            let _ = shutdown_tx.send(()).await;
        },
        _ = tokio::signal::ctrl_c() => {
            info!("termination_signal::graceful_shutdown");
            
            shutdown_signal.shutdown();
            let _ = shutdown_tx.send(()).await;
        },
        _ = shutdown_rx.recv() => {
            info!("shutdown_signal::other_component");
            
            shutdown_signal.shutdown();
        }
    }

    info!("all_component_shutdown");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    info!("baseer::shutdown");
    
    Ok(())
  }
  
  fn spawn_token_analyzer(
    &self,
    db_engine: Arc<StorageEngine>,
    shutdown_signal: ShutdownSignal,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let mut subscriber = db_engine.redis.queue.pubsub.as_ref().write().await;
        
        // Subscribe to the channel
        subscriber.subscribe("new_token_created").await?;
        
        // Process messages
        let mut msg_stream = subscriber.on_message();
        
        // Create a future that completes when shutdown is signaled
        let shutdown_future = shutdown_signal.wait_for_shutdown();
        
        // Process messages until shutdown
        tokio::select! {
            _ = shutdown_future => {
                info!("Token analyzer received shutdown signal");
                Ok(())
            }
            _ = async {
                while let Some(msg) = msg_stream.next().await {
                    if let Ok(payload) = msg.get_payload::<String>() {
                        if let Ok(token) = serde_json::from_str::<NewTokenCache>(&payload) {
                            info!("new_token_created: {}", token.mint);
                            
                            // Here you would trigger your CEX analysis
                            // This will be handled by the Baseer engine
                        }
                    }
                }
                Ok::<(), crate::Error>(())
            } => {
                info!("Token analyzer message stream ended");
                let _ = shutdown_tx.send(()).await;
                Ok::<(), crate::Error>(())
            }
        }
    })
}
}
