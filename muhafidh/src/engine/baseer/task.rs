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
use crate::handler::token::CreatorHandler;
use crate::model::creator::metadata::CreatorMetadata;
use crate::pipeline::crawler::creator::make_creator_crawler_pipeline;
use crate::pipeline::processor::creator::CreatorInstructionProcessor;
use crate::storage::redis::model::NewTokenCache;

impl Baseer {
    pub fn spawn_new_token_subscriber(
        &self,
        shutdown_signal: ShutdownSignal,
        sender: mpsc::Sender<NewTokenCache>,
    ) -> JoinHandle<()> {
        let db = self.db.clone();
        let max_depth = self.config.creator_analyzer.max_depth;
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
                        let mut creator_metadata: CreatorMetadata = token.clone().into();
                        creator_metadata.max_depth = max_depth;
                        creator_metadata.mark_as_unprocessed().await;
                        if let Err(e) = db_for_buffer.redis.queue.add_unprocessed_account(&creator_metadata).await {
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
                        }
                    }
                  },
                  _ = shutdown_fut.wait_for_shutdown() => {
                    debug!("token_subscriber::shutdown_signal_received::ending_task");
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
        sender: mpsc::Sender<CreatorHandler>,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<Result<()>> {
        let baseer = self.clone();
        let rpc_config = self.rpc_config.clone();
        let creator_analyzer_config = self.config.creator_analyzer.clone();
        let creator_analyzer_config = Arc::new(creator_analyzer_config);
        let sender = sender.clone();

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
                        let creator_metadata = CreatorMetadata::from_token(token.clone(), max_depth).await;
                        let sender = sender.clone();

                        tokio::spawn(async move {
                            let creator_metadata = Arc::new(creator_metadata);
                            let processor = CreatorInstructionProcessor::new(creator_handler.clone(), creator_metadata.clone(), child_token.clone(), creator_analyzer_config);

                            match make_creator_crawler_pipeline(
                                processor.clone(),
                                child_token.clone(),
                                max_depth,
                                rpc_config_clone,
                                sender.clone()
                            ).await {
                                Ok(Some((mut pipeline, analyzed_account))) => {
                                    // Run the pipeline
                                    let pipeline_result = pipeline.run().await;
                                    
                                    // Mark this address as done processing
                                    creator_metadata.mark_done_processing(analyzed_account).await;
                                    
                                    // After marking done, atomically check if BFS is now complete and claim completion
                                    // This prevents race conditions with the crawler
                                    if creator_metadata.try_claim_completion().await {
                                        debug!("bfs_completed_after_processing::mint::{}", creator_metadata.mint);
                                        if let Err(e) = sender.try_send(CreatorHandler::MaxDepthReached {
                                            creator_metadata: Arc::clone(&creator_metadata),
                                            child_token: child_token.clone(),
                                        }) {
                                            error!("failed_to_send_max_depth_reached_after_processing::mint::{}::error::{}", creator_metadata.mint, e);
                                        }
                                    }
                                    
                                    println!("Creator pipeline completed successfully for mint: {}", creator_metadata.mint);
                                    
                                    // Handle pipeline result
                                    if let Err(e) = pipeline_result {
                                        error!("pipeline_run_failed::mint::{}::error::{}", token.mint, e);
                                        // Handle failure by adding to failed queue
                                        processor.handle_pipeline_failure().await;
                                    }
                                },
                                Ok(None) => {
                                    debug!("no_pipeline_created::mint::{}", token.mint);
                                    // Add to unprocessed queue when no pipeline is created
                                    let mut unprocessed_metadata = (*creator_metadata).clone();
                                    unprocessed_metadata.mark_as_unprocessed().await;
                                    if let Err(e) = creator_handler.add_failed_account(&unprocessed_metadata).await {
                                        error!("failed_to_add_to_unprocessed_queue::mint::{}::error::{}", token.mint, e);
                                    }
                                },
                                Err(e) => {
                                    error!("pipeline_creation_failed::mint::{}::error::{}", token.mint, e);
                                    // Handle failure by adding to failed queue
                                    processor.handle_pipeline_failure().await;
                                }
                            }
                        });
                    },
                    _ = cancellation_token.cancelled() => {
                        // Application-wide shutdown requested
                        debug!("creator_analyzer_task_cancelled::shutting_down");
                        // All child tokens are automatically cancelled when parent is cancelled
                        break;
                    },
                    else => {
                        // Channel closed, exit gracefully
                        debug!("creator_analyzer_task_ending::channel_closed");
                        break;
                    }
                }
            }

            Ok(())
        })
    }

    // Simplified account recovery task - just fetches and sends to actor
    pub fn spawn_account_recovery(
        &self,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<Result<()>> {
        let db = self.db.clone();
        let creator_handler = self.creator_handler.clone();
        let shutdown_signal = creator_handler.shutdown.clone();
        let creator_analyzer_config = Arc::new(self.config.creator_analyzer.clone());

        tokio::spawn(async move {
            debug!("account_recovery_task::started");

            // Define recovery interval - start with faster checks
            let base_interval = Duration::from_secs(5); // Check every 5 seconds when active
            let idle_interval = Duration::from_secs(30); // Check every 30 seconds when idle
            let mut current_interval = base_interval;
            let mut recovery_timer = tokio::time::interval(current_interval);
            recovery_timer.tick().await;

            let mut consecutive_empty_checks = 0;

            loop {
                tokio::select! {
                    _ = recovery_timer.tick() => {
                        let mut found_work = false;

                        // First try to process failed accounts (higher priority)
                        match db.redis.queue.get_next_failed_account().await {
                            Ok(Some(account)) => {
                                found_work = true;
                                debug!("processing_failed_account::account::{}::mint::{}::retry_count::{}",
                                    account.address, account.mint, account.retry_count);

                                // Check if we've exceeded max retries
                                if account.retry_count >= 3 {
                                    // warn!("max_retries_exceeded::account::{}::mint::{}::moving_to_dead_letter",
                                    //     account.address, account.mint);
                                    // <TODO> implement dead letter queue here if needed
                                    continue;
                                }

                                // Send to actor for processing
                                let child_token = cancellation_token.child_token();
                                let creator_metadata = Arc::new(account);

                                if let Err(e) = creator_handler.sender.try_send(CreatorHandler::ProcessRecoveredAccount {
                                    creator_metadata: creator_metadata.clone(),
                                    child_token,
                                    creator_analyzer_config: creator_analyzer_config.clone(),
                                }) {
                                    error!("failed_to_send_recovery_request::mint::{}::error::{}",
                                        creator_metadata.mint, e);

                                    // Re-add to failed queue
                                    let mut failed_account = (*creator_metadata).clone();
                                    failed_account.mark_as_failed().await;
                                    if let Err(e) = db.redis.queue.add_failed_account(&failed_account).await {
                                        error!("failed_to_requeue_failed_account::account::{}::error::{}",
                                            failed_account.address, e);
                                    }
                                }
                            },
                            Ok(None) => {
                                // No failed accounts, try unprocessed
                                match db.redis.queue.get_next_unprocessed_account().await {
                                    Ok(Some(account)) => {
                                        found_work = true;
                                        debug!("processing_unprocessed_account::account::{}::mint::{}",
                                            account.address, account.mint);

                                        // Send to actor for processing
                                        let child_token = cancellation_token.child_token();
                                        let creator_metadata = Arc::new(account);

                                        if let Err(e) = creator_handler.sender.try_send(CreatorHandler::ProcessRecoveredAccount {
                                            creator_metadata: creator_metadata.clone(),
                                            child_token,
                                            creator_analyzer_config: creator_analyzer_config.clone(),
                                        }) {
                                            error!("failed_to_send_unprocessed_request::mint::{}::error::{}",
                                                creator_metadata.mint, e);

                                            // Mark as failed and add to failed queue
                                            let mut failed_account = (*creator_metadata).clone();
                                            failed_account.mark_as_failed().await;
                                            if let Err(e) = db.redis.queue.add_failed_account(&failed_account).await {
                                                error!("failed_to_add_to_failed_queue::account::{}::error::{}",
                                                    failed_account.address, e);
                                            }
                                        }
                                    },
                                    Ok(None) => {
                                        // Log periodically that we're checking but no accounts to recover
                                        debug!("no_accounts_to_recover::checking_again_in_{}s", current_interval.as_secs());
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

                        // Adjust interval based on activity
                        if found_work {
                            consecutive_empty_checks = 0;
                            if current_interval != base_interval {
                                current_interval = base_interval;
                                recovery_timer = tokio::time::interval(current_interval);
                                recovery_timer.tick().await; // Reset the timer
                                debug!("recovery_task::switching_to_active_mode::interval_{}s", current_interval.as_secs());
                            }
                        } else {
                            consecutive_empty_checks += 1;
                            // Switch to idle mode after 3 empty checks
                            if consecutive_empty_checks >= 3 && current_interval != idle_interval {
                                current_interval = idle_interval;
                                recovery_timer = tokio::time::interval(current_interval);
                                recovery_timer.tick().await; // Reset the timer
                                debug!("recovery_task::switching_to_idle_mode::interval_{}s", current_interval.as_secs());
                            }
                        }
                    },
                    _ = shutdown_signal.wait_for_shutdown() => {
                        warn!("account_recovery_task::shutdown_signal_received");
                        break;
                    }
                }
            }

            info!("account_recovery_task::ended");
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
