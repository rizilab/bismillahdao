use std::sync::Arc;

use solana_pubkey::Pubkey;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;

use super::CreatorHandler;
use crate::HandlerError;
use crate::Result;
use crate::config::CreatorAnalyzerConfig;
use crate::err_with_loc;
use crate::handler::shutdown::ShutdownSignal;
use crate::model::cex::Cex;
use crate::model::creator::graph::SharedCreatorCexConnectionGraph;
use crate::model::creator::metadata::CreatorMetadata;
use crate::pipeline::crawler::creator::make_creator_crawler_pipeline;
use crate::pipeline::processor::creator::CreatorInstructionProcessor;
use crate::config::RpcConfig;
use crate::storage::StorageEngine;

pub struct CreatorHandlerMetadata {
    receiver: mpsc::Receiver<CreatorHandler>,
    db: Arc<StorageEngine>,
    shutdown: ShutdownSignal,
    rpc_config: Arc<RpcConfig>,
}

impl CreatorHandlerMetadata {
    pub fn new(
        receiver: mpsc::Receiver<CreatorHandler>,
        db: Arc<StorageEngine>,
        shutdown: ShutdownSignal,
        rpc_config: Arc<RpcConfig>,
    ) -> Self {
        Self {
            receiver,
            db,
            shutdown,
            rpc_config,
        }
    }

    async fn process_cex_connection(
        &self,
        cex: Cex,
        connection_graph: SharedCreatorCexConnectionGraph,
        mint: Pubkey,
        name: String,
        uri: String,
    ) -> Result<()> {
        // Update the token record with CEX source
        let cex_sources = vec![cex.address];
        let cex_updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let connection_graph = connection_graph.clone_graph().await;

        // Update in PostgreSQL
        self.db
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
            error!("record_cex_activity_postgres_failed::{}::mint::{}::error::{}", cex.name, mint, e);
            // Continue despite the error
        } else {
            debug!("record_cex_activity_postgres_success::{}::mint::{}", cex.name, mint);
        }

        // Store the connection graph in pgrouting
        if let Err(e) = self.db.postgres.graph.store_connection_graph(&mint, &connection_graph).await {
            error!("store_connection_graph_pgrouting_failed::{}::mint::{}::error::{}", cex.name, mint, e);
            // Continue despite the error
        } else {
            debug!("store_connection_graph_pgrouting_success::{}::mint::{}", cex.name, mint);
        }

        // Update Redis cache
        let token_key = mint.to_string();
        if let Ok(Some(mut token_metadata)) =
            self.db.redis.kv.get::<crate::model::token::TokenMetadata>(&token_key).await
        {
            token_metadata.cex_sources = Some(cex_sources.clone());
            token_metadata.cex_updated_at = Some(cex_updated_at);

            if let Err(e) = self.db.redis.kv.set(&token_key, &token_metadata).await {
                error!("update_token_redis_failed::{}::mint::{}::error::{}", cex.name, mint, e);
            } else {
                debug!("update_token_redis_success::{}::mint::{}", cex.name, mint);
            }
        }

        // Store connection graph in Redis
        let graph_key = format!("developer_connection_graph:{}", mint);
        if let Err(e) = self.db.redis.kv.set_graph(&graph_key, &connection_graph).await {
            error!("store_connection_graph_redis_failed::{}::mint::{}::error::{}", cex.name, mint, e);
        } else {
            debug!("store_connection_graph_redis_success::{}::mint::{}", cex.name, mint);
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
            error!("store_cex_data_redis_failed::{}::mint::{}::error::{}", cex.name, mint, e);
        } else {
            debug!("store_cex_data_redis_success::{}::mint::{}", cex.name, mint);
        }

        // Publish event
        let event_data = serde_json::json!({
          "mint": mint.to_string(),
          "name": name,
          "uri": uri,
          "cex_name": cex.name.to_string(),
          "cex_address": cex.address.to_string(),
          "cex_updated_at": cex_updated_at,
          "node_count": connection_graph.get_node_count(),
          "edge_count": connection_graph.get_edge_count(),
          "graph": connection_graph
        });

        if let Err(e) = self.db.redis.queue.publish("token_cex_updated", &event_data).await {
            error!("publish_token_cex_updated_event_failed::{}::mint::{}::error::{}", cex.name, mint, e);
        } else {
            debug!("publish_token_cex_updated_event_success::{}::mint::{}", cex.name, mint);
        }

        debug!("process_cex_connection_completed::{}::mint::{}", cex.name, mint);
        Ok(())
    }

    async fn process_bfs_level(
        &self,
        creator_metadata: Arc<CreatorMetadata>,
        _sender: Pubkey,
        child_token: CancellationToken,
        creator_analyzer_config: Arc<CreatorAnalyzerConfig>,
    ) -> Result<()> {
        let db_engine = self.db.clone();
        let shutdown_signal = self.shutdown.clone();
        let (operator_sender, operator_receiver) = mpsc::channel(1000);
        let rpc_config = self.rpc_config.clone();

        let creator_handler = Arc::new(CreatorHandlerOperator::new(
            db_engine.clone(),
            shutdown_signal.clone(),
            operator_receiver,
            operator_sender,
            rpc_config,
        ));

        let max_depth = creator_metadata.max_depth;
        let processor = CreatorInstructionProcessor::new(
            creator_handler.clone(),
            creator_metadata.clone(),
            child_token.clone(),
            creator_analyzer_config.clone(),
        );
        let rpc_config = self.rpc_config.clone();

        tokio::spawn(async move {
            match make_creator_crawler_pipeline(processor.clone(), child_token, max_depth, rpc_config).await {
                Ok(Some(mut pipeline)) => {
                    if let Err(e) = pipeline.run().await {
                        error!("pipeline_run_failed_on_bfs_level::mint::{}::error::{}", creator_metadata.mint, e);
                        // Handle failure by adding to failed queue
                        processor.handle_pipeline_failure().await;
                    }
                },
                Ok(None) => {
                    debug!("no_pipeline_created_for_bfs_level::mint::{}", creator_metadata.mint);
                },
                Err(e) => {
                    error!("pipeline_creation_failed_on_bfs_level::mint::{}::error::{}", creator_metadata.mint, e);
                    // Handle failure by adding to failed queue
                    processor.handle_pipeline_failure().await;
                },
            }
        });
        Ok(())
    }

    async fn process_recovered_account(
        &self,
        creator_metadata: Arc<CreatorMetadata>,
        child_token: CancellationToken,
        creator_analyzer_config: Arc<CreatorAnalyzerConfig>,
    ) -> Result<()> {
        let db_engine = self.db.clone();
        let shutdown_signal = self.shutdown.clone();
        let (operator_sender, operator_receiver) = mpsc::channel(1000);
        let rpc_config = self.rpc_config.clone();

        let creator_handler = Arc::new(CreatorHandlerOperator::new(
            db_engine.clone(),
            shutdown_signal.clone(),
            operator_receiver,
            operator_sender,
            rpc_config,
        ));

        let max_depth = creator_metadata.max_depth;
        let processor = CreatorInstructionProcessor::new(
            creator_handler.clone(),
            creator_metadata.clone(),
            child_token.clone(),
            creator_analyzer_config.clone(),
        );
        let rpc_config = self.rpc_config.clone();

        tokio::spawn(async move {
            match make_creator_crawler_pipeline(processor.clone(), child_token, max_depth, rpc_config).await {
                Ok(Some(mut pipeline)) => {
                    if let Err(e) = pipeline.run().await {
                        error!("recovery_pipeline_run_failed::mint::{}::error::{}", creator_metadata.mint, e);
                        // Handle failure by adding to failed queue
                        processor.handle_pipeline_failure().await;
                    }
                },
                Ok(None) => {
                    debug!("no_pipeline_created_for_recovery::mint::{}", creator_metadata.mint);
                },
                Err(e) => {
                    error!("recovery_pipeline_creation_failed::mint::{}::error::{}", creator_metadata.mint, e);
                    // Handle failure by adding to failed queue
                    processor.handle_pipeline_failure().await;
                },
            }
        });
        Ok(())
    }
}

async fn run_creator_handler_metadata(mut creator_handler_metadata: CreatorHandlerMetadata) {
    debug!("creator_handler_metadata::started");

    loop {
        tokio::select! {
            Some(msg) = creator_handler_metadata.receiver.recv() => {
                match msg {
                    CreatorHandler::ProcessBfsLevel { creator_metadata, sender, child_token, creator_analyzer_config } => {
                        if let Err(e) = creator_handler_metadata.process_bfs_level(creator_metadata.clone(), sender, child_token, creator_analyzer_config).await {
                            error!("failed_to_process_sender::error::{}", e);

                            // Add to failed queue when process_bfs_level fails
                            let mut failed_metadata = (*creator_metadata).clone();
                            failed_metadata.mark_as_bfs_failed();
                            if let Err(e) = creator_handler_metadata.db.redis.queue.add_failed_account(&failed_metadata).await {
                                error!("failed_to_add_to_failed_queue_after_bfs_failure::account::{}::error::{}",
                                    failed_metadata.address, e);
                            }
                        }
                    },
                    CreatorHandler::CexConnection { cex, cex_connection, mint, name, uri } => {
                        if let Err(e) = creator_handler_metadata.process_cex_connection(
                            cex.clone(), cex_connection, mint, name, uri
                        ).await {
                            error!("cex_failed::{}::mint::{}::error::{}", cex.clone().name, mint, e);
                        }
                    },
                    CreatorHandler::ProcessRecoveredAccount { creator_metadata, child_token, creator_analyzer_config } => {
                        if let Err(e) = creator_handler_metadata.process_recovered_account(
                            creator_metadata.clone(), child_token, creator_analyzer_config
                        ).await {
                            error!("failed_to_process_recovered_account::error::{}", e);

                            // Add back to failed queue when recovery fails
                            let mut failed_metadata = (*creator_metadata).clone();
                            failed_metadata.mark_as_failed();
                            if let Err(e) = creator_handler_metadata.db.redis.queue.add_failed_account(&failed_metadata).await {
                                error!("failed_to_requeue_failed_account_after_recovery_failure::account::{}::error::{}",
                                    failed_metadata.address, e);
                            }
                        }
                    },
                }
            },
            else => {
                // Channel closed, exit gracefully
                debug!("creator_handler_metadata::channel_closed::exiting");
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreatorHandlerOperator {
    db: Arc<StorageEngine>,
    pub sender: mpsc::Sender<CreatorHandler>,
    pub shutdown: ShutdownSignal,
}

impl CreatorHandlerOperator {
    pub fn new(
        db: Arc<StorageEngine>,
        shutdown: ShutdownSignal,
        receiver: mpsc::Receiver<CreatorHandler>,
        sender: mpsc::Sender<CreatorHandler>,
        rpc_config: Arc<RpcConfig>,
    ) -> Self {
        let metadata = CreatorHandlerMetadata::new(receiver, db.clone(), shutdown.clone(), rpc_config.clone());

        // Spawn the actor
        tokio::spawn(run_creator_handler_metadata(metadata));

        Self {
            db,
            sender,
            shutdown,
        }
    }

    pub async fn process_sender(
        &self,
        creator_metadata: Arc<CreatorMetadata>,
        sender: Pubkey,
        receiver: Pubkey,
        amount: f64,
        timestamp: i64,
        child_token: CancellationToken,
        creator_analyzer_config: Arc<CreatorAnalyzerConfig>,
    ) -> Result<()> {
        let wallet_connection = creator_metadata.wallet_connection.clone();

        // Get the depth of the receiver (the account being analyzed) from BFS state
        let receiver_depth = if let Some((depth, _)) = creator_metadata.get_visited(&receiver).await {
            depth
        } else {
            // If receiver is not visited yet (shouldn't happen in normal flow), use 0
            0
        };

        if let Some(cex_name) = Cex::get_exchange_name(sender) {
            let cex = Cex::new(cex_name, sender);
            wallet_connection.add_node(sender, true).await;
            wallet_connection.add_edge(sender, receiver, amount, timestamp).await;

            if let Err(e) = self.sender.try_send(CreatorHandler::CexConnection {
                cex: cex.clone(),
                cex_connection: wallet_connection,
                mint: creator_metadata.mint,
                name: creator_metadata.token_name.clone(),
                uri: creator_metadata.token_uri.clone(),
            }) {
                error!("failed_to_send_cex_connection_request::sender::{}::receiver::{}::amount::{}::timestamp::{}::error::{}", sender, receiver, amount, timestamp, e);
                return Err(err_with_loc!(HandlerError::SendCreatorHandlerError(format!(
                    "Failed to send cex connection request ({}): {}",
                    cex.name, e
                ))));
            }

            // cache receiver connection with cex
            let address_key = format!("address:{}", receiver);
            let address_data = serde_json::json!({
                "name": cex.name.to_string(),
                "address": cex.address.to_string(),
            });
            if let Err(e) = self.db.redis.kv.set(&address_key, &address_data).await {
                error!("store_address_data_redis_failed::{}::error::{}", receiver, e);
            }

            let cex_url = format!("https://axiom.trade/meme/{}", creator_metadata.bonding_curve.unwrap_or("<missing>"));
            info!(
                "cex_found::{}::name::{}::depth::{}::mint::{}::axiom::{}",
                cex.name, creator_metadata.token_name, receiver_depth, creator_metadata.mint, cex_url
            );
            child_token.cancel();
            return Ok(());
        }

        if let Ok(Some(cex_found)) = self.db.redis.kv.get::<Cex>(&sender.to_string()).await {
            wallet_connection.add_node(sender, false).await;
            wallet_connection.add_edge(sender, receiver, amount, timestamp).await;

            if let Err(e) = self.sender.try_send(CreatorHandler::CexConnection {
                cex: cex_found.clone(),
                cex_connection: wallet_connection,
                mint: creator_metadata.mint,
                name: creator_metadata.token_name.clone(),
                uri: creator_metadata.token_uri.clone(),
            }) {
                error!("failed_to_send_cex_connection_request::sender::{}::receiver::{}::amount::{}::timestamp::{}::error::{}", sender, receiver, amount, timestamp, e);
                return Err(err_with_loc!(HandlerError::SendCreatorHandlerError(format!(
                    "Failed to send cex connection request ({}): {}",
                    cex_found.name, e
                ))));
            }
            info!(
                "sender_cex_connection_found::mint::{}::cex::{}::depth::{}",
                creator_metadata.mint, cex_found.name, receiver_depth
            );
            child_token.cancel();
            return Ok(());
        }

        // Check if receiver was already visited and update BFS state
        if let Some((depth, mut neighbors)) = creator_metadata.get_visited(&receiver).await {
            neighbors.insert(0, sender);
            creator_metadata.mark_visited(sender, depth + 1, neighbors.clone()).await;
            creator_metadata.push_to_queue((sender, depth + 1, neighbors)).await;
        }

        wallet_connection.add_node(sender, false).await;
        wallet_connection.add_edge(sender, receiver, amount, timestamp).await;

        // start the pipeline
        if let Err(e) = self.sender.try_send(CreatorHandler::ProcessBfsLevel {
            creator_metadata,
            sender,
            child_token,
            creator_analyzer_config,
        }) {
            error!(
                "failed_to_send_process_sender_request::sender::{}::receiver::{}::amount::{}::timestamp::{}::error::{}",
                sender, receiver, amount, timestamp, e
            );
            return Err(err_with_loc!(HandlerError::SendCreatorHandlerError(format!(
                "Failed to send process sender request: {}",
                e
            ))));
        }

        Ok(())
    }

    pub async fn get_pending_account_counts(&self) -> Result<(usize, usize)> {
        self.db.redis.queue.get_pending_account_counts().await.map_err(|e| {
            error!("failed_to_get_pending_account_counts: {}", e);
            err_with_loc!(HandlerError::RedisQueryError(format!("Failed to get pending account counts: {}", e)))
        })
    }

    pub async fn add_failed_account(
        &self,
        account: &CreatorMetadata,
    ) -> Result<()> {
        self.db.redis.queue.add_failed_account(account).await.map_err(|e| {
            error!("failed_to_add_failed_account: {}", e);
            err_with_loc!(HandlerError::RedisQueryError(format!("Failed to add failed account: {}", e)))
        })
    }

    pub fn shutdown(&self) {
        self.shutdown.shutdown();
    }
}
