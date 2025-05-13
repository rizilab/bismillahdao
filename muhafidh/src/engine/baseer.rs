use std::sync::Arc;
use std::time::Duration;

use async_stream;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;

use crate::config::load_config;
use crate::config::Config;
use crate::err_with_loc;
use crate::error::EngineError;
use crate::error::RedisClientError;
use crate::handler::shutdown::ShutdownSignal;
use crate::handler::token::creator::CreatorHandlerOperator;
use crate::pipeline::crawler::creator::make_creator_crawler_pipeline;
use crate::setup_tracing;
use crate::storage::make_storage_engine;
use crate::storage::redis::model::NewTokenCache;
use crate::storage::StorageEngine;
use crate::Result;

#[derive(Clone)]
pub struct Baseer {
  pub config:          Config,
  pub db:              Arc<StorageEngine>,
  pub creator_handler: Arc<CreatorHandlerOperator>,
}

impl Baseer {
  pub async fn run() -> Result<()> {
    info!("Starting Baseer (بصير): The Analyzer");

    setup_tracing("baseer");

    let config = load_config("Config.toml")?;

    let db_engine = Arc::new(make_storage_engine("baseer", &config).await?);
    info!("db_engine::created");

    let shutdown_signal = ShutdownSignal::new();
    let cancellation_token = CancellationToken::new();
    let creator_handler = Arc::new(CreatorHandlerOperator::new(
      db_engine.clone(),
      shutdown_signal.clone(),
      config.rpc.get_http_url(),
      cancellation_token.clone(),
    ));

    let baseer = Baseer {
      config,
      db: db_engine.clone(),
      creator_handler,
    };

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);

    let (sender, receiver) = mpsc::channel(1000);

    let new_token_creator_analyzer =
      baseer.spawn_new_token_creator_analyzer(shutdown_tx.clone(), receiver, cancellation_token.clone());

    let new_token_subscriber = baseer.spawn_new_token_subscriber(shutdown_signal.clone(), shutdown_tx.clone(), sender);

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
    shutdown_tx: mpsc::Sender<()>,
    mut receiver: mpsc::Receiver<NewTokenCache>,
    cancellation_token: CancellationToken,
  ) -> JoinHandle<Result<()>> {
    let baseer = self.clone();
    let creator_handler = self.creator_handler.clone();
    let max_concurrent_requests = self.config.creator_analyzer.max_concurrent_requests;

    tokio::spawn(async move {
      let creator_handler = creator_handler.clone();
      let creator_stream_task = async {
        let creator_stream = async_stream::stream! {
            while let Some(token) = receiver.recv().await {
                yield token;
            }
        };
        let rpc_url = baseer.config.rpc.get_http_url();
        let creator_handler = creator_handler.clone();
        let max_depth = baseer.config.creator_analyzer.max_depth;
        creator_stream
          .map(|token| {
            let child_token = cancellation_token.child_token();
            let rpc_url = rpc_url.clone();
            let creator_handler = creator_handler.clone();
            async move {
              let mut pipeline = make_creator_crawler_pipeline(
                rpc_url,
                creator_handler.clone(),
                token.clone(),
                child_token.clone(),
                max_depth,
              )?;

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
          .for_each(|result| {
            async move {
              tokio::time::sleep(Duration::from_millis(50)).await;
              if let Err(e) = result {
                error!("creator_analysis_failed: {}", e);
              }
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
    shutdown_signal: ShutdownSignal,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
    sender: mpsc::Sender<NewTokenCache>,
  ) -> JoinHandle<Result<()>> {
    let baseer = self.clone();
    tokio::spawn(async move {
      let mut subscriber = baseer.db.redis.queue.pubsub.as_ref().write().await;

      // Subscribe to the channel with a retry mechanism
      let mut retries = 0;
      const MAX_RETRIES: usize = 5;

      loop {
        match subscriber.subscribe("new_token_created").await {
          Ok(_) => {
            info!("Successfully subscribed to new_token_created channel");
            break;
          },
          Err(e) => {
            retries += 1;
            error!("Failed to subscribe to new_token_created (attempt {}/{}): {}", retries, MAX_RETRIES, e);

            if retries >= MAX_RETRIES {
              return Err(err_with_loc!(RedisClientError::SubscribeError(format!(
                "failed_to_subscribe_to_new_token_created after {} attempts: {}",
                MAX_RETRIES, e
              ))));
            }

            // Exponential backoff
            tokio::time::sleep(Duration::from_millis(100 * 2u64.pow(retries as u32))).await;
          },
        }
      }

      // Create a larger channel for buffering messages (to handle bursts)
      let (buffer_tx, mut buffer_rx) = mpsc::channel::<NewTokenCache>(1000);

      // Process messages
      let mut msg_stream = subscriber.on_message();

      // Create a future that completes when shutdown is signaled
      let shutdown_future = shutdown_signal.wait_for_shutdown();

      // Spawn a task to handle the message buffer
      let buffer_task = tokio::spawn(async move {
        while let Some(token) = buffer_rx.recv().await {
          match sender.send(token).await {
            Ok(_) => debug!("Token sent to processor"),
            Err(e) => {
              error!("Failed to send token to processor: {}", e);
              return;
            },
          }
        }
      });

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
                              match serde_json::from_str::<NewTokenCache>(&payload) {
                                  Ok(token) => {
                                      debug!("new_token_received: {}", token.mint);
                                      // Send to buffer instead of directly to processor
                                      if let Err(e) = buffer_tx.send(token).await {
                                          error!("Failed to buffer token: {}", e);
                                      }
                                  },
                                  Err(e) => {
                                      error!("Failed to parse token payload: {}", e);
                                  }
                              }
                          } else {
                              error!("Failed to get payload from message");
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

      // Clean up the buffer task
      buffer_task.abort();

      Ok(())
    })
  }
}
