use std::sync::Arc;

use solana_pubkey::Pubkey;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;

use super::CreatorHandler;
use crate::config::load_config;
use crate::err_with_loc;
use crate::error::HandlerError;
use crate::handler::shutdown::ShutdownSignal;
use crate::model::cex::Cex;
use crate::pipeline::crawler::creator::make_creator_crawler_pipeline;
use crate::storage::in_memory::creator::CreatorCexConnectionGraph;
use crate::storage::redis::model::NewTokenCache;
use crate::storage::StorageEngine;
use crate::Result;

pub struct CreatorHandlerMetadata {
  receiver:           mpsc::Receiver<CreatorHandler>,
  db:                 Arc<StorageEngine>,
  shutdown:           ShutdownSignal,
  rpc_url:            String,
  cancellation_token: CancellationToken,
}

impl CreatorHandlerMetadata {
  pub fn new(
    receiver: mpsc::Receiver<CreatorHandler>,
    db: Arc<StorageEngine>,
    shutdown: ShutdownSignal,
    rpc_url: String,
    cancellation_token: CancellationToken,
  ) -> Self {
    Self {
      receiver,
      db,
      shutdown,
      rpc_url,
      cancellation_token,
    }
  }

  async fn process_cex_connection(
    &self,
    cex: Cex,
    connection_graph: CreatorCexConnectionGraph,
    mint: Pubkey,
    creator: Pubkey,
  ) -> Result<()> {
    info!("process_cex_connection::{}::to::{}::mint::{}", cex.name, creator, mint);

    // Update the token record with CEX source
    let cex_sources = vec![cex.address];
    let cex_updated_at = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap_or_default()
      .as_secs();

    // Update in PostgreSQL
    self
      .db
      .postgres
      .db
      .update_token_cex_sources(&mint, &cex_sources, cex_updated_at)
      .await?;

    // Record CEX activity for analytics
    if let Err(e) = self
      .db
      .postgres
      .db
      .record_cex_activity(&cex.name.to_string(), &cex.address, &mint)
      .await
    {
      error!("record_cex_activity_postgres_failed::{}::to::{}::mint::{}::error::{}", cex.name, creator, mint, e);
      // Continue despite the error
    } else {
      debug!("record_cex_activity_postgres_success::{}::to::{}::mint::{}", cex.name, creator, mint);
    }

    // Store the connection graph in pgrouting
    if let Err(e) = self.db.postgres.graph.store_connection_graph(&mint, &connection_graph).await {
      error!("store_connection_graph_pgrouting_failed::{}::to::{}::mint::{}::error::{}", cex.name, creator, mint, e);
      // Continue despite the error
    } else {
      debug!("store_connection_graph_pgrouting_success::{}::to::{}::mint::{}", cex.name, creator, mint);
    }

    // Update Redis cache
    let token_key = mint.to_string();
    if let Ok(Some(mut token_metadata)) = self.db.redis.kv.get::<crate::model::token::TokenMetadata>(&token_key).await {
      token_metadata.cex_sources = Some(cex_sources.clone());
      token_metadata.cex_updated_at = Some(cex_updated_at);

      if let Err(e) = self.db.redis.kv.set(&token_key, &token_metadata).await {
        error!("update_token_redis_failed::{}::to::{}::mint::{}::error::{}", cex.name, creator, mint, e);
      } else {
        debug!("update_token_redis_success::{}::to::{}::mint::{}", cex.name, creator, mint);
      }
    }

    // Store connection graph in Redis
    let graph_key = format!("developer_connection_graph:{}", mint);
    if let Err(e) = self.db.redis.kv.set_graph(&graph_key, &connection_graph).await {
      error!("store_connection_graph_redis_failed::{}::to::{}::mint::{}::error::{}", cex.name, creator, mint, e);
    } else {
      debug!("store_connection_graph_redis_success::{}::to::{}::mint::{}", cex.name, creator, mint);
    }

    // Store CEX information in Redis for quick access
    let cex_key = format!("cex:{}", cex.address);
    let cex_data = serde_json::json!({
      "name": cex.name.to_string(),
      "address": cex.address.to_string(),
      "latest_mint": mint.to_string(),
      "updated_at": cex_updated_at
    });

    if let Err(e) = self.db.redis.kv.set(&cex_key, &cex_data).await {
      error!("store_cex_data_redis_failed::{}::to::{}::mint::{}::error::{}", cex.name, creator, mint, e);
    } else {
      debug!("store_cex_data_redis_success::{}::to::{}::mint::{}", cex.name, creator, mint);
    }

    // Publish event
    let event_data = serde_json::json!({
      "mint": mint.to_string(),
      "cex_name": cex.name.to_string(),
      "cex_address": cex.address.to_string(),
      "creator": creator.to_string(),
      "cex_updated_at": cex_updated_at,
      "node_count": connection_graph.get_node_count(),
      "edge_count": connection_graph.get_edge_count()
    });

    if let Err(e) = self.db.redis.queue.publish("token_cex_updated", &event_data).await {
      error!("publish_token_cex_updated_event_failed::{}::to::{}::mint::{}::error::{}", cex.name, creator, mint, e);
    } else {
      debug!("publish_token_cex_updated_event_success::{}::to::{}::mint::{}", cex.name, creator, mint);
    }

    info!("process_cex_connection_completed::{}::to::{}::mint::{}", cex.name, creator, mint);
    Ok(())
  }

  async fn process_bfs_level(
    &self,
    address: Pubkey,
    depth: usize,
    mint: Pubkey,
    connection_graph: CreatorCexConnectionGraph,
  ) -> Result<()> {
    debug!("process_bfs_level::address::{}::depth::{}::mint::{}", address, depth, mint);

    // Get configurable max depth
    let config = load_config("Config.toml")?;
    let max_bfs_depth = config.creator_analyzer.max_depth;
    // Skip if we've reached max depth
    if depth >= max_bfs_depth {
      info!("process_bfs_level_reached_max_depth::address::{}::depth::{}::mint::{}", address, depth, mint);
      return Ok(());
    }

    // Create a new pipeline to analyze this address
    let token = NewTokenCache {
      mint,
      name: String::new(),   // Not important for BFS
      symbol: String::new(), // Not important for BFS
      uri: String::new(),    // Not important for BFS
      creator: address,      // Use the current BFS address as target
    };

    let child_token = self.cancellation_token.child_token();

    // Create a new handler for this BFS level
    let handler =
      CreatorHandlerOperator::new(self.db.clone(), self.shutdown.clone(), self.rpc_url.clone(), child_token.clone());

    // Store connection graph in Redis for BFS level
    let graph_key = format!("bfs_connection_graph:{}:{}", mint, depth);
    if let Err(e) = self.db.redis.kv.set_graph(&graph_key, &connection_graph).await {
      error!(
        "store_bfs_connection_graph_redis_failed::address::{}::depth::{}::mint::{}::error::{}",
        address, depth, mint, e
      );
    }

    let handler = Arc::new(handler);

    let mut pipeline =
      make_creator_crawler_pipeline(self.rpc_url.clone(), handler.clone(), token, child_token.clone(), max_bfs_depth)?;

    // Run in background with proper cancellation handling
    tokio::spawn(async move {
      tokio::select! {
          result = pipeline.run() => {
              match result {
                  Ok(_) => {
                      info!("bfs_completed::address::{}::depth::{}::mint::{}", address, depth, mint);
                  },
                  Err(e) => {
                      error!("bfs_error::address::{}::depth::{}::mint::{}::error::{}", address, depth, mint, e);
                  }
              }
          },
          _ = child_token.cancelled() => {
              debug!("cancelling_bfs_pipeline::address::{}::depth::{}::mint::{}", address, depth, mint);
          }
      }
    });

    Ok(())
  }
}

async fn run_creator_handler_metadata(mut creator_handler_metadata: CreatorHandlerMetadata) {
  info!("creator_handler_metadata::started");

  loop {
    tokio::select! {
        Some(msg) = creator_handler_metadata.receiver.recv() => {
            match msg {
                CreatorHandler::StoreCreator { creator_metadata } => {
                    info!("store_creator_metadata::{}", creator_metadata.address);
                },
                CreatorHandler::CexConnection { cex, cex_connection, mint, creator } => {
                    if let Err(e) = creator_handler_metadata.process_cex_connection(
                        cex.clone(), cex_connection, mint, creator
                    ).await {
                        error!("cex_failed::{}::to::{}::mint::{}::error::{}", cex.clone().name, creator, mint, e);
                    }
                },
                CreatorHandler::ProcessBfsLevel { address, depth, mint, connection_graph } => {
                    if let Err(e) = creator_handler_metadata.process_bfs_level(
                        address, depth, mint, connection_graph
                    ).await {
                        error!("bfs_failed::address::{}::depth::{}::mint::{}::error::{}", address, depth, mint, e);
                    }
                }
            }
        },
        _ = creator_handler_metadata.shutdown.wait_for_shutdown() => {
            debug!("creator_handler_metadata::received_shutdown_signal");
            break;
        },
        else => {
            debug!("creator_handler_metadata::all_senders_dropped");
            break;
        }
    }
  }

  info!("creator_handler_metadata::shutdown");
}

#[derive(Debug, Clone)]
pub struct CreatorHandlerOperator {
  db:       Arc<StorageEngine>,
  sender:   mpsc::Sender<CreatorHandler>,
  shutdown: ShutdownSignal,
}

impl CreatorHandlerOperator {
  pub fn new(
    db: Arc<StorageEngine>,
    shutdown: ShutdownSignal,
    rpc_url: String,
    cancellation_token: CancellationToken,
  ) -> Self {
    let (sender, receiver) = mpsc::channel(1000);

    let metadata = CreatorHandlerMetadata::new(receiver, db.clone(), shutdown.clone(), rpc_url, cancellation_token);

    // Spawn the actor
    tokio::spawn(run_creator_handler_metadata(metadata));

    Self { db, sender, shutdown }
  }

  pub async fn record_cex_connection(
    &self,
    cex: Cex,
    connection_graph: CreatorCexConnectionGraph,
    mint: Pubkey,
    creator: Pubkey,
  ) -> Result<()> {
    debug!("record_cex_connection::{}::to:: {}", cex.name, creator);

    // Record the CEX connection directly in the database
    let cex_sources = vec![cex.address];
    let cex_updated_at = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap_or_default()
      .as_secs();

    // Update PostgreSQL with the CEX connection
    match self
      .db
      .postgres
      .db
      .update_token_cex_sources(&mint, &cex_sources, cex_updated_at)
      .await
    {
      Ok(_) => {
        debug!("record_cex_connection_postgres_success::{}::to::{}", cex.clone().name, creator);
      },
      Err(e) => {
        error!("record_cex_connection_postgres_failed::{}::to::{}::error::{}", cex.clone().name, creator, e);
        // Continue processing despite the error
      },
    }

    // Use try_send for backpressure handling
    match self.sender.try_send(CreatorHandler::CexConnection {
      cex: cex.clone(),
      cex_connection: connection_graph,
      mint,
      creator,
    }) {
      Ok(()) => {
        debug!("cex_connection_sent_for_processing::{}::to::{}::mint::{}", cex.clone().name, creator, mint);
        Ok(())
      },
      Err(e) => {
        error!("cex_connection_send_failed::{}::to::{}::mint::{}::error::{}", cex.clone().name, creator, mint, e);
        Err(err_with_loc!(HandlerError::SendCreatorHandlerError(format!("Failed to send CEX connection: {}", e))))
      },
    }
  }

  pub async fn process_next_bfs_level(
    &self,
    address: Pubkey,
    depth: usize,
    mint: Pubkey,
    connection_graph: CreatorCexConnectionGraph,
  ) -> Result<()> {
    debug!("process_next_bfs_level::address::{}::depth::{}::mint::{}", address, depth, mint);

    // Use try_send for backpressure handling
    match self
      .sender
      .try_send(CreatorHandler::ProcessBfsLevel { address, depth, mint, connection_graph })
    {
      Ok(()) => {
        debug!("bfs_level_processing_request_sent::address::{}::depth::{}::mint::{}", address, depth, mint);
        Ok(())
      },
      Err(e) => {
        error!(
          "bfs_level_processing_request_send_failed::address::{}::depth::{}::mint::{}::error::{}",
          address, depth, mint, e
        );
        Err(err_with_loc!(HandlerError::SendCreatorHandlerError(format!("Failed to send BFS level request: {}", e))))
      },
    }
  }

  pub fn shutdown(&self) { self.shutdown.shutdown(); }
}
