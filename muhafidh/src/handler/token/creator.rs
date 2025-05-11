use crate::handler::shutdown::ShutdownSignal;
use crate::storage::StorageEngine;
use crate::Result;
use super::CreatorHandler;
use crate::model::creator::CreatorMetadata;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error, debug};
use crate::err_with_loc;
use crate::error::HandlerError;

pub struct CreatorHandlerMetadata {
    receiver: mpsc::Receiver<CreatorHandler>,
    db: Arc<StorageEngine>,
    shutdown: ShutdownSignal,
}

impl CreatorHandlerMetadata {
    pub fn new(receiver: mpsc::Receiver<CreatorHandler>, db: Arc<StorageEngine>, shutdown: ShutdownSignal) -> Self {
        Self { receiver, db, shutdown }
    }
}

async fn run_creator_handler_metadata(mut creator_handler_metadata: CreatorHandlerMetadata) {
    info!("Creator handler metadata started");
    
    loop {
        tokio::select! {
            Some(msg) = creator_handler_metadata.receiver.recv() => {
                match msg {
                    CreatorHandler::StoreCreator { creator_metadata } => {
                        info!("store_creator_metadata: {}", creator_metadata.address);
                    },
                    // Only handle store token messages
                    _ => {}
                }
            },
            _ = creator_handler_metadata.shutdown.wait_for_shutdown() => {
                info!("creator_handler_metadata::received_shutdown_signal");
                break;
            },
            else => {
                info!("creator_handler_metadata::all_senders_dropped");
                break;
            }
        }
    }
    
    info!("creator_handler_metadata::shutdown");
}

#[derive(Debug, Clone)]
pub struct CreatorHandlerOperator {
    sender: mpsc::Sender<CreatorHandler>,
    shutdown: ShutdownSignal,
}

impl CreatorHandlerOperator {
    pub fn new(db: Arc<StorageEngine>, shutdown: ShutdownSignal) -> Self {
        let (sender, receiver) = mpsc::channel(1000);
        
        let receiver = CreatorHandlerMetadata::new(receiver, db, shutdown.clone());
        
        // Spawn the actor
        tokio::spawn(run_creator_handler_metadata(receiver));
        
        Self { sender, shutdown }
    }
    
    pub fn shutdown(&self) {
        self.shutdown.shutdown();
    }
}