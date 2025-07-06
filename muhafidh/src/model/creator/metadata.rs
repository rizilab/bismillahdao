use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;
use serde::Serialize;
use solana_pubkey::Pubkey;
use tokio::sync::RwLock;
use tracing::debug;

use super::graph::SharedCreatorConnectionGraph;
use crate::storage::redis::model::NewTokenCache;

// Define account status for different processing stages
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AccountStatus {
    NewAccount,  // Fresh account from new token
    Unprocessed, // Account saved due to buffer overflow
    Failed,      // Failed due to rate limit or RPC errors
    BfsQueue,    // Failed during BFS processing
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SerializableBfsState {
//     pub visited_addresses: HashMap<Pubkey, (usize, Vec<Pubkey>)>,
//     pub history: Vec<Pubkey>,
//     pub queue: VecDeque<(Pubkey, usize, Vec<Pubkey>)>,
//     pub processed_cex: HashSet<Pubkey>,
// }

#[derive(Debug, Clone)]
pub struct SharedBfsState {
    pub visited_addresses: Arc<RwLock<BTreeSet<Pubkey>>>,
    pub history: Arc<RwLock<BTreeSet<Pubkey>>>,
    pub queue: Arc<RwLock<VecDeque<(Pubkey, usize, Pubkey)>>>, // (address, depth, parent_address)
    pub processed_cex: Arc<RwLock<HashSet<Pubkey>>>,
    // // Runtime-only state (not serialized)
    // pub processing_addresses: Arc<RwLock<HashSet<Pubkey>>>, // Track addresses currently being scanned
    // pub completion_sent: Arc<AtomicBool>, // Atomic flag to prevent duplicate MaxDepthReached events
    // pub cex_found: Arc<AtomicBool>, // Atomic flag to track if CEX connection was found
}

impl Default for SharedBfsState {
    fn default() -> Self {
        Self {
            visited_addresses: Arc::new(RwLock::new(BTreeSet::new())),
            history: Arc::new(RwLock::new(BTreeSet::new())),
            queue: Arc::new(RwLock::new(VecDeque::new())),
            processed_cex: Arc::new(RwLock::new(HashSet::new())),
            // processing_addresses: Arc::new(RwLock::new(HashSet::new())),
            // completion_sent: Arc::new(AtomicBool::new(false)),
            // cex_found: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl SharedBfsState {
    pub fn initialize(initial_address: Pubkey) -> Self {
        let mut visited_addresses = BTreeSet::new();
        visited_addresses.insert(initial_address);

        let mut queue = VecDeque::new();
        queue.push_back((initial_address, 0, initial_address));

        Self {
            visited_addresses: Arc::new(RwLock::new(visited_addresses)),
            history: Arc::new(RwLock::new(BTreeSet::new())),
            queue: Arc::new(RwLock::new(queue)),
            processed_cex: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    // // Convert to serializable state
    // pub async fn to_serializable(&self) -> SerializableBfsState {
    //     SerializableBfsState {
    //         visited_addresses: self.visited_addresses.read().await.clone(),
    //         history: self.history.read().await.clone(),
    //         queue: self.queue.read().await.clone(),
    //         processed_cex: self.processed_cex.read().await.clone(),
    //     }
    // }

    // // Create from serializable state
    // pub fn from_serializable(serializable: SerializableBfsState) -> Self {
    //     Self {
    //         visited_addresses: Arc::new(RwLock::new(serializable.visited_addresses)),
    //         history: Arc::new(RwLock::new(serializable.history)),
    //         queue: Arc::new(RwLock::new(serializable.queue)),
    //         processed_cex: Arc::new(RwLock::new(serializable.processed_cex)),
    //         processing_addresses: Arc::new(RwLock::new(HashSet::new())), // Always start fresh
    //         completion_sent: Arc::new(AtomicBool::new(false)), // Always start fresh
    //         cex_found: Arc::new(AtomicBool::new(false)), // Always start fresh
    //     }
    // }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorMetadata {
    // Token information
    pub mint: Pubkey, // The token mint address
    pub bonding_curve: Option<Pubkey>,
    pub token_name: String,
    pub token_symbol: String,
    pub token_uri: String,
    #[serde(skip)]
    pub analyzed_account: Arc<RwLock<Pubkey>>, // Current address being analyzed
    pub max_depth: usize,
    pub original_creator: Pubkey, // Original creator of the token

    // Processing metadata
    pub created_at: u64,
    pub latest_update: u64,
    pub retry_count: usize,
    pub status: AccountStatus,

    // Analysis results
    pub total_received: f64,
    pub cex_sources: Vec<Pubkey>,
    pub cex_updated_at: u64,
    pub wallet_connection: SharedCreatorConnectionGraph,
    
    // BFS state - serialized as SerializableBfsState
    // #[serde(with = "bfs_state_serde")] // TODO: Uncomment this when we have a better way to serialize the BFS state
    #[serde(skip)]
    pub bfs_state: SharedBfsState,

}

// Custom serde module for BFS state
// TODO: Uncomment this when we have a better way to serialize the BFS state
// mod bfs_state_serde {
//     use super::*;
//     use serde::{Deserialize, Deserializer, Serialize, Serializer};

//     pub fn serialize<S>(bfs_state: &SharedBfsState, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         // For serialization, we need to use tokio::task::block_in_place or return an error
//         // Since we can't use async in serde, we'll use a simpler approach
//         // In practice, you might want to handle this differently based on your async runtime
//         let serializable = tokio::task::block_in_place(|| {
//             tokio::runtime::Handle::current().block_on(bfs_state.to_serializable())
//         });
//         serializable.serialize(serializer)
//     }

//     pub fn deserialize<'de, D>(deserializer: D) -> Result<SharedBfsState, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let serializable = SerializableBfsState::deserialize(deserializer)?;
//         Ok(SharedBfsState::from_serializable(serializable))
//     }
// }

impl CreatorMetadata {
    // Create from NewTokenCache
    pub async fn initialize(
        token: NewTokenCache, 
        max_depth: usize
    ) -> Self {
        let metadata = Self {
            mint: token.mint,
            bonding_curve: token.bonding_curve,
            token_name: token.name,
            token_symbol: token.symbol,
            token_uri: token.uri,
            analyzed_account: Arc::new(RwLock::new(token.creator)),
            max_depth,
            original_creator: token.creator,
            created_at: token.created_at,
            latest_update: chrono::Utc::now().timestamp() as u64,
            retry_count: 0,
            status: AccountStatus::NewAccount,
            total_received: 0.0,
            cex_sources: Vec::new(),
            cex_updated_at: 0,
            wallet_connection: SharedCreatorConnectionGraph::new(),
            bfs_state: SharedBfsState::initialize(token.creator),
        };
        
        metadata
    }

    // Mark as failed and increment retry count
    pub async fn mark_as_failed(&mut self) {
        self.retry_count += 1;
        self.status = AccountStatus::Failed;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
    }

    // Mark as unprocessed (for buffer overflow)
    pub async fn mark_as_unprocessed(&mut self) {
        self.status = AccountStatus::Unprocessed;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
    }

    // Mark as BFS queue (failed during BFS)
    pub async fn mark_as_bfs_failed(&mut self) {
        self.status = AccountStatus::BfsQueue;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
    }

    // Helper methods for BFS operations
    pub async fn pop_from_queue(&self) -> Option<(Pubkey, usize, Pubkey)> {
        self.bfs_state.queue.write().await.pop_front()
    }

    pub async fn push_to_queue(
        &self,
        item: (Pubkey, usize, Pubkey),
    ) {
        self.bfs_state.queue.write().await.push_back(item);
    }
    
    pub async fn is_queue_empty(&self) -> bool {
        self.bfs_state.queue.read().await.is_empty()
    }
    
    pub async fn empty_queue(&self) {
        self.bfs_state.queue.write().await.clear();
    }

    pub async fn add_to_history(
        &self,
        address: Pubkey,
    ) {
        self.bfs_state.history.write().await.insert(address);
    }

    pub async fn get_history_front(&self) -> Option<Pubkey> {
        self.bfs_state.history.read().await.first().copied()
    }

    pub async fn mark_visited(
        &self,
        address: Pubkey,
    ) {
        self.bfs_state.visited_addresses.write().await.insert(address);
    }

    pub async fn is_visited(
        &self,
        address: &Pubkey,
    ) -> bool {
        self.bfs_state.visited_addresses.read().await.contains(address)
    }

    pub async fn set_analyzed_account(&self, analyzed_account: Pubkey) {
        *self.analyzed_account.write().await = analyzed_account;
    }
    
    pub async fn get_analyzed_account(&self) -> Pubkey {
        self.analyzed_account.read().await.clone()
    }
}
