use crate::config::Config;
use crate::config::load_config;
use crate::storage::redis::model::NewTokenCache;
use crate::Result;
use crate::error::EngineError;
use crate::error::RedisClientError;
use crate::setup_tracing;
use crate::storage::make_storage_engine;
use crate::storage::StorageEngine;
use crate::handler::shutdown::ShutdownSignal;
use crate::handler::token::creator::CreatorHandlerOperator;
use std::sync::Arc;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use std::time::Duration;

use crate::pipeline::crawler::creator::make_creator_crawler_pipeline;
use tokio::task::JoinHandle;

use tracing::info;
use tracing::debug;
use tracing::error;
use crate::err_with_loc;

use async_stream;

#[derive(Clone)]
pub struct Baseer {
  pub config: Config,
  pub db:     Arc<StorageEngine>,
  pub creator_handler: Arc<CreatorHandlerOperator>,
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
    
    let creator_handler = Arc::new(CreatorHandlerOperator::new(
        db_engine.clone(),
        shutdown_signal.clone(),
    ));
    
    let baseer = Baseer { 
        config, 
        db: db_engine.clone(),
        creator_handler,
    };

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);

    let db_engine = db_engine.clone();
            
    let (sender, receiver) = mpsc::channel(1000);
    
    let new_token_creator_analyzer = baseer.spawn_new_token_creator_analyzer(
        db_engine.clone(),
        shutdown_tx.clone(),
        receiver,
        10
    );

    let new_token_subscriber = baseer.spawn_new_token_subscriber(
        db_engine.clone(),
        shutdown_signal.clone(),
        shutdown_tx.clone(),
        sender,
    );
    
    tokio::select! {
        _ = new_token_creator_analyzer => {
            info!("new_token_creator_analyzer::completed");
            shutdown_signal.shutdown();
            let _ = shutdown_tx.send(()).await;
        },
        _ = new_token_subscriber => {
            info!("new_token_subscriber::completed");
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
            let _ = shutdown_tx.send(()).await;
        }
    }

    info!("all_component_shutdown");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    info!("baseer::shutdown");
    
    Ok(())
  }
  
  fn spawn_new_token_creator_analyzer(
    &self,
    db_engine: Arc<StorageEngine>,
    shutdown_tx: mpsc::Sender<()>,
    mut receiver: mpsc::Receiver<NewTokenCache>,
    max_concurrent_requests: usize,
  ) -> JoinHandle<Result<()>> {
    let baseer = self.clone();
    let creator_handler = self.creator_handler.clone();
    let cancellation_token = CancellationToken::new();
    
    tokio::spawn(async move {
        let creator_stream_task = async {
            let creator_stream = async_stream::stream! {
                while let Some(token) = receiver.recv().await {
                    yield token;
                }
            };

            creator_stream
                .map(|token| {
                    let baseer = baseer.clone();
                    let child_token = cancellation_token.child_token();
                    async move {
                        let mut pipeline = make_creator_crawler_pipeline(
                            baseer.clone(),
                            token.clone(),
                            child_token.clone())?;

                        tokio::select! {
                            result = pipeline.run() => {
                                match result {
                                    Ok(_) => {
                                        info!("pipeline_completed::creator::{}::{}", token.name, token.creator);
                                        Ok(())
                                    },
                                    Err(e) => {
                                        error!("pipeline_error: {}", e);
                                        Err(err_with_loc!(EngineError::EngineError(e)))
                                    }
                                }
                            },
                            _ = child_token.cancelled() => {
                                info!("cancelling_pipeline::creator::{}::{}", token.name, token.creator);
                                Ok(())
                            }
                        }
                    }
                })
                .buffer_unordered(max_concurrent_requests)
                .for_each(|result| async move {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    if let Err(e) = result {
                        error!("creator_analysis_failed: {}", e);
                    }                   
                })
                .await;
        };

        tokio::select! {
            _ = creator_stream_task => {
                info!("creator_stream_task::completed");
                creator_handler.shutdown();
                let _ = shutdown_tx.send(()).await;
            },
            _ = tokio::signal::ctrl_c() => {
                info!("termination_signal::graceful_shutdown");
                cancellation_token.cancel();
                creator_handler.shutdown();
                let _ = shutdown_tx.send(()).await;
            },
        }
        Ok(())
    })
  }
        
  fn spawn_new_token_subscriber(
    &self,
    db_engine: Arc<StorageEngine>,
    shutdown_signal: ShutdownSignal,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
    sender: mpsc::Sender<NewTokenCache>,
) -> JoinHandle<Result<()>> {
    let baseer = self.clone();
    tokio::spawn(async move {
        let mut subscriber = db_engine.redis.queue.pubsub.as_ref().write().await;
        
        // Subscribe to the channel
        subscriber.subscribe("new_token_created").await.map_err(|e| {
            error!("failed_to_subscribe_to_new_token_created: {}", e);
            err_with_loc!(RedisClientError::SubscribeError(format!("failed_to_subscribe_to_new_token_created: {}", e)))
        })?;
        
        // Process messages
        let mut msg_stream = subscriber.on_message();
        
        // Create a future that completes when shutdown is signaled
        let shutdown_future = shutdown_signal.wait_for_shutdown();
        
        // Process messages until shutdown
        tokio::select! {
            _ = shutdown_future => {
                info!("Token analyzer received shutdown signal");
                let _ = shutdown_tx.send(()).await;
            }
            _ = async {
                loop {
                    match msg_stream.next().await {
                        Some(msg) => {
                            if let Ok(payload) = msg.get_payload::<String>() {
                                if let Ok(token) = serde_json::from_str::<NewTokenCache>(&payload) {
                                    debug!("new_token_received: {}", token.mint);
                                    if let Err(e) = sender.try_send(token) {
                                        error!("failed_to_send_token: {}", e);
                                        let _ = shutdown_tx.send(()).await;
                                        break;
                                    }
                                }
                            }
                        }
                        None => {
                            info!("Token analyzer message stream ended");
                            let _ = shutdown_tx.send(()).await;
                            break;
                        }
                    }
                }
            } => {
                info!("Token analyzer message stream ended");
            }
        }
        Ok(())
    })
  }
}
