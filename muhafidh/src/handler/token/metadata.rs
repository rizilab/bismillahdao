use crate::handler::shutdown::ShutdownSignal;
use crate::handler::token::TokenHandler;
use crate::model::token::TokenMetadata;
use crate::storage::StorageEngine;
use carbon_pumpfun_decoder::instructions::create::{Create, CreateInstructionAccounts};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error, debug};
use crate::err_with_loc;
use crate::error::HandlerError;
use crate::Result;

struct TokenHandlerMetadata {
    receiver: mpsc::Receiver<TokenHandler>,
    db: Arc<StorageEngine>,
    shutdown: ShutdownSignal,
}

impl TokenHandlerMetadata {
    fn new(
        receiver: mpsc::Receiver<TokenHandler>,
        db: Arc<StorageEngine>,
        shutdown: ShutdownSignal,
    ) -> Self {
        Self { receiver, db, shutdown }
    }

    async fn store_token(&self, token: TokenMetadata) -> Result<()> {
        // First check Redis cache
        let cached_token = self.db.redis.kv.get_token_metadata(&token.mint.to_string()).await?;
        
        // Skip if we already have this token with the same data
        if let Some(existing) = cached_token {
            if existing.name == token.name && existing.symbol == token.symbol && existing.uri == token.uri {
                debug!("already_cached_with_same_data::{}::{}", token.name, token.mint);
                return Ok(());
            }
        }
        
        // Store in Postgres
        self.db.postgres.db.insert_token_metadata(&token).await?;
        
        // Update Redis cache
        self.db.redis.kv.set_token_metadata(&token.mint.to_string(), &token).await?;
        
        // Publish event for cross-service communication
        self.db.redis.queue.publish_new_token_metadata(&token).await?;
        
        info!("Stored new token: {} ({})", token.name, token.mint);
        Ok(())
    }
}

async fn run_token_handler_metadata(mut token_creation_metadata: TokenHandlerMetadata) {
    info!("token_creation_metadata_started");
    
    loop {
        tokio::select! {
            Some(msg) = token_creation_metadata.receiver.recv() => {
                match msg {
                    TokenHandler::StoreToken { token_metadata } => {
                        if let Err(e) = token_creation_metadata.store_token(token_metadata).await {
                            error!("Failed to store token: {}", e);
                        }
                    },
                    // Only handle store token messages
                    _ => {}
                }
            },
            _ = token_creation_metadata.shutdown.wait_for_shutdown() => {
                info!("token_creation_metadata::received_shutdown_signal");
                break;
            },
            else => {
                info!("token_creation_metadata::all_senders_dropped");
                break;
            }
        }
    }
    
    info!("token_creation_metadata::shutdown");
}

#[derive(Debug, Clone)]
pub struct TokenHandlerMetadataOperator {
    sender: mpsc::Sender<TokenHandler>,
    shutdown: ShutdownSignal,
}

impl TokenHandlerMetadataOperator {
    pub fn new(db: Arc<StorageEngine>, shutdown: ShutdownSignal) -> Self {
        // Use Listen's approach with 1000 buffer
        let (sender, receiver) = mpsc::channel(1000);
        
        let receiver = TokenHandlerMetadata::new(receiver, db, shutdown.clone());
        // Spawn the actor
        tokio::spawn(run_token_handler_metadata(receiver));
        
        Self { sender, shutdown }
    }
    
    pub async fn store_token(
        &self, 
        create_data: &Create, 
        accounts: &CreateInstructionAccounts,
        block_time: u64
    ) -> Result<()> {
        let token_metadata = TokenMetadata::new(
            accounts.mint,
            create_data.name.clone(),
            create_data.symbol.clone(),
            create_data.uri.clone(),
            create_data.creator,
            block_time,
            Some(accounts.associated_bonding_curve),
            false, // is_bonded
            0,     // all_time_high_price
            block_time // all_time_high_price_at
        );
        
        // Use try_send for backpressure handling
        match self.sender.try_send(TokenHandler::StoreToken { token_metadata }) {
            Ok(()) => Ok(()),
            Err(e) => {
                error!("send_token_handler_failed: {}", e);
                Err(err_with_loc!(HandlerError::SendTokenHandlerError(format!("send_token_handler_failed:{}", e))))
            }
        }
    }
    
    pub fn shutdown(&self) {
        self.shutdown.shutdown();
    }
}