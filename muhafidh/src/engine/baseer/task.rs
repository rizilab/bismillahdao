use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::Baseer;
use crate::Result;
use crate::handler::shutdown::ShutdownSignal;
use crate::model::creator::metadata::CreatorMetadata;
use crate::pipeline::crawler::creator::make_creator_crawler_pipeline;
use crate::pipeline::processor::creator::CreatorInstructionProcessor;
use crate::storage::redis::model::AccountToAnalyze;
use crate::storage::redis::model::NewTokenCache;

impl Baseer {
    pub fn spawn_new_token_subscriber(
        &self,
        shutdown_signal: ShutdownSignal,
        sender: mpsc::Sender<NewTokenCache>,
    ) -> JoinHandle<()> {
        let db = self.db.clone();
        tokio::spawn(async move {
            // Clone the db here to avoid borrowing conflicts
            let db_for_subscriber = db.clone();
            let mut subscriber = db_for_subscriber.redis.queue.pubsub.as_ref().write().await;

            if let Err(e) = subscriber.subscribe("new_token_created").await {
                error!("failed_to_subscribe_to_new_token_created::error::{}", e);
            }

            // Create a channel for buffering messages - with good capacity for performance
            let (buffer_tx, mut buffer_rx) = mpsc::channel::<NewTokenCache>(1000);
            // Process messages
            let mut msg_stream = subscriber.on_message();

            // Clone db for the buffer task
            let db_for_buffer = db.clone();
            let shutdown_fut = shutdown_signal.clone();
            loop {
                tokio::select! {
                  Some(token) = buffer_rx.recv() => {
                    // Store the mint before sending the token
                    let mint = token.mint;
                    if buffer_rx.capacity() < 900 {
                        error!("low_capacity_on_buffer::mint::{}", mint);
                        if let Err(e) = db_for_buffer.redis.queue.add_unprocessed_account(&AccountToAnalyze::from(token.clone())).await {
                            error!("failed_to_add_token_to_redis::mint::{}::error::{}", mint, e);
                        }
                    }

                    if let Err(e) = sender.try_send(token.clone()) {
                        error!("failed_to_send_token_to_processor::mint::{}::error::{}", mint, e);
                    }
                  },
                  Some(message) = msg_stream.next() => {
                    if let Ok(msg) = message.get_payload::<String>() {
                        if let Ok(token) = serde_json::from_str::<NewTokenCache>(&msg) {
                            debug!("new_token_received::mint::{}::name::{}::creator::{}",
                                token.mint, token.name, token.creator);
                            if let Err(e) = buffer_tx.try_send(token.clone()) {
                                error!("failed_to_send_token_to_buffer::mint::{}::error::{}", token.mint, e);
                            }
                            debug!("token_sent_to_buffer::mint::{}", token.mint);
                        }
                    }
                  },
                  _ = shutdown_fut.wait_for_shutdown() => {
                    info!("token_subscriber::shutdown_signal_received::ending_task");
                    break;
                  }
                }
            }
            debug!("token_subscriber::buffer_task_ending");
        })
    }

    pub fn spawn_new_token_creator_analyzer(
        &self,
        mut receiver: mpsc::Receiver<NewTokenCache>,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<Result<()>> {
        let baseer = self.clone();
        let rpc_config = self.rpc_config.clone();
        let creator_analyzer_config = self.config.creator_analyzer.clone();
        let creator_analyzer_config = Arc::new(creator_analyzer_config);

        tokio::spawn(async move {
            let max_depth = baseer.config.creator_analyzer.max_depth;
            // Process tokens using select for better control
            loop {
                let creator_analyzer_config = creator_analyzer_config.clone();
                let creator_handler = baseer.creator_handler.clone();
                tokio::select! {
                    Some(token) = receiver.recv() => {
                        debug!("new_token_received::mint::{}::name::{}::creator::{}", token.mint, token.name, token.creator);
                        let child_token = cancellation_token.child_token();
                        let rpc_config_clone = rpc_config.clone();
                        let creator_metadata = CreatorMetadata::new(token.mint, token.creator, max_depth).await;

                        tokio::spawn(async move {
                            let creator_metadata = Arc::new(creator_metadata);
                            let processor = CreatorInstructionProcessor::new(creator_handler.clone(), creator_metadata, child_token.clone(), creator_analyzer_config);

                            if let Ok(Some(mut pipeline)) = make_creator_crawler_pipeline(
                                processor,
                                child_token,
                                max_depth,
                                rpc_config_clone
                            ).await {
                                if let Err(e) = pipeline.run().await {
                                    error!("pipeline_run_failed::mint::{}::error::{}", token.mint, e);
                                }
                            }
                        });
                    },
                    _ = cancellation_token.cancelled() => {
                        // Application-wide shutdown requested
                        info!("creator_analyzer_task_cancelled::shutting_down");
                        // All child tokens are automatically cancelled when parent is cancelled
                        break;
                    },
                    else => {
                        // Channel closed, exit gracefully
                        info!("creator_analyzer_task_ending::channel_closed");
                        break;
                    }
                }
            }

            Ok(())
        })
    }

    // New method to spawn a task for processing failed and unprocessed accounts
    pub fn spawn_account_recovery(
        &self,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<Result<()>> {
        let db = self.db.clone();
        let creator_handler = self.creator_handler.clone();
        let rpc_config = self.rpc_config.clone();
        let creator_analyzer_config = Arc::new(self.config.creator_analyzer.clone());
        let shutdown_signal = creator_handler.shutdown.clone();

        tokio::spawn(async move {
            debug!("account_recovery_task::started");

            // Define a single recovery interval - all operations use this timer
            let recovery_interval = Duration::from_secs(10);

            // Create single timer
            let mut recovery_timer = tokio::time::interval(recovery_interval);

            // Start the timer immediately
            recovery_timer.tick().await;

            // Loop until shutdown
            loop {
                let db = db.clone();
                tokio::select! {
                    _ = recovery_timer.tick() => {
                        // First try to process failed accounts (higher priority)
                        match db.redis.queue.get_next_failed_account().await {
                            Ok(Some(account)) => {
                                info!("processing_failed_account::account::{}::depth::{}::retry_count::{}",
                                    account.account, account.depth, account.retry_count);

                                // Check if we've exceeded max retries
                                if account.retry_count >= 3 {
                                    error!("max_retries_exceeded::account::{}::moving_to_dead_letter", account.account);
                                    // Could implement dead letter queue here if needed
                                    continue;
                                }

                                // Create a new token cache from the account
                                let token = NewTokenCache {
                                    mint: account.parent_mint,
                                    name: format!("Recovery-{}", account.parent_mint),
                                    symbol: "REC".to_string(),
                                    uri: "".to_string(),
                                    creator: account.account,
                                    created_at: account.created_at,
                                };

                                // Process the account using the same logic as new tokens
                                let child_token = cancellation_token.child_token();
                                let creator_metadata = CreatorMetadata::new(
                                    token.mint,
                                    token.creator,
                                    creator_analyzer_config.max_depth
                                ).await;

                                let creator_handler_clone = creator_handler.clone();
                                let rpc_config_clone = rpc_config.clone();
                                let creator_analyzer_config_clone = creator_analyzer_config.clone();
                                let max_depth = creator_analyzer_config_clone.max_depth;

                                tokio::spawn(async move {
                                    let creator_metadata = Arc::new(creator_metadata);
                                    let processor = CreatorInstructionProcessor::new(
                                        creator_handler_clone.clone(),
                                        creator_metadata,
                                        child_token.clone(),
                                        creator_analyzer_config_clone
                                    );

                                    if let Ok(Some(mut pipeline)) = make_creator_crawler_pipeline(
                                        processor,
                                        child_token,
                                        max_depth,
                                        rpc_config_clone
                                    ).await {
                                        if let Err(e) = pipeline.run().await {
                                            error!("recovery_pipeline_failed::mint::{}::error::{}", token.mint, e);

                                            // Mark as failed and re-add to queue
                                            let mut failed_account = account.clone();
                                            failed_account.mark_as_failed();

                                            if let Err(e) = db.redis.queue.add_failed_account(&failed_account).await {
                                                error!("failed_to_requeue_failed_account::account::{}::error::{}",
                                                    failed_account.account, e);
                                            }
                                        } else {
                                            info!("recovery_pipeline_success::mint::{}::account::{}",
                                                token.mint, token.creator);
                                        }
                                    }
                                });
                            },
                            Ok(None) => {
                                // No failed accounts, try unprocessed
                                match db.redis.queue.get_next_unprocessed_account().await {
                                    Ok(Some(account)) => {
                                        info!("processing_unprocessed_account::account::{}::depth::{}",
                                            account.account, account.depth);

                                        // Create a new token cache from the account
                                        let token = NewTokenCache {
                                            mint: account.parent_mint,
                                            name: format!("Unprocessed-{}", account.parent_mint),
                                            symbol: "UNP".to_string(),
                                            uri: "".to_string(),
                                            creator: account.account,
                                            created_at: account.created_at,
                                        };

                                        // Process the account
                                        let child_token = cancellation_token.child_token();
                                        let creator_metadata = CreatorMetadata::new(
                                            token.mint,
                                            token.creator,
                                            creator_analyzer_config.max_depth
                                        ).await;

                                        let creator_handler_clone = creator_handler.clone();
                                        let rpc_config_clone = rpc_config.clone();
                                        let creator_analyzer_config_clone = creator_analyzer_config.clone();
                                        let max_depth = creator_analyzer_config_clone.max_depth;

                                        tokio::spawn(async move {
                                            let creator_metadata = Arc::new(creator_metadata);
                                            let processor = CreatorInstructionProcessor::new(
                                                creator_handler_clone.clone(),
                                                creator_metadata,
                                                child_token.clone(),
                                                creator_analyzer_config_clone
                                            );

                                            if let Ok(Some(mut pipeline)) = make_creator_crawler_pipeline(
                                                processor,
                                                child_token,
                                                max_depth,
                                                rpc_config_clone
                                            ).await {
                                                if let Err(e) = pipeline.run().await {
                                                    error!("unprocessed_pipeline_failed::mint::{}::error::{}",
                                                        token.mint, e);

                                                    // Mark as failed and add to failed queue
                                                    let mut failed_account = account.clone();
                                                    failed_account.mark_as_failed();

                                                    if let Err(e) = db.redis.queue.add_failed_account(&failed_account).await {
                                                        error!("failed_to_add_to_failed_queue::account::{}::error::{}",
                                                            failed_account.account, e);
                                                    }
                                                } else {
                                                    info!("unprocessed_pipeline_success::mint::{}::account::{}",
                                                        token.mint, token.creator);
                                                }
                                            }
                                        });
                                    },
                                    Ok(None) => {
                                        debug!("no_accounts_to_recover");
                                    },
                                    Err(e) => {
                                        error!("failed_to_get_unprocessed_account::error::{}", e);
                                    }
                                }
                            },
                            Err(e) => {
                                error!("failed_to_get_failed_account::error::{}", e);
                            }
                        }
                    },
                    _ = shutdown_signal.wait_for_shutdown() => {
                        warn!("account_recovery_task::shutdown_signal_received");
                        break;
                    }
                }
            }

            debug!("account_recovery_task::ended");
            Ok(())
        })
    }

    // New method to spawn a task for queue reporting
    pub fn spawn_account_queue_reporting(&self) -> JoinHandle<Result<()>> {
        let creator_handler = self.creator_handler.clone();
        let shutdown_signal = creator_handler.shutdown.clone();

        tokio::spawn(async move {
            debug!("account_queue_reporting_task::started");

            // Define a single reporting interval
            let reporting_interval = Duration::from_secs(5);

            // Create single timer
            let mut reporting_timer = tokio::time::interval(reporting_interval);

            // Start the timer immediately
            reporting_timer.tick().await;

            // Loop until shutdown
            loop {
                tokio::select! {
                    _ = reporting_timer.tick() => {
                        // Get queue counts
                        match creator_handler.get_pending_account_counts().await {
                            Ok((failed_count, unprocessed_count)) => {
                                let total = failed_count + unprocessed_count;

                                if total > 0 {
                                    info!("queue_status::failed::{}::unprocessed::{}::total::{}",
                                        failed_count, unprocessed_count, total);

                                    // Log warning if queues are getting too large
                                    if total > 1000 {
                                        warn!("queue_backlog_high::total::{}::consider_scaling", total);
                                    }

                                    if failed_count > 100 {
                                        warn!("high_failure_rate::failed_count::{}::check_rpc_health", failed_count);
                                    }
                                } else {
                                    debug!("queue_status::all_queues_empty");
                                }
                            },
                            Err(e) => {
                                error!("failed_to_get_queue_counts::error::{}", e);
                            }
                        }
                    },
                    _ = shutdown_signal.wait_for_shutdown() => {
                        warn!("account_queue_reporting_task::shutdown_signal_received");
                        break;
                    }
                }
            }

            debug!("account_queue_reporting_task::ended");
            Ok(())
        })
    }
}
