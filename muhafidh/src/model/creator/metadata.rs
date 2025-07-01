use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;
use serde::Serialize;
use solana_pubkey::Pubkey;
use tokio::sync::RwLock;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableBfsState {
    pub visited_addresses: HashMap<Pubkey, (usize, Vec<Pubkey>)>,
    pub history: Vec<Pubkey>,
    pub queue: VecDeque<(Pubkey, usize, Vec<Pubkey>)>,
    pub processed_cex: HashSet<Pubkey>,
}

#[derive(Debug, Clone)]
pub struct SharedBfsState {
    pub visited_addresses: Arc<RwLock<HashMap<Pubkey, (usize, Vec<Pubkey>)>>>,
    pub history: Arc<RwLock<Vec<Pubkey>>>,
    pub queue: Arc<RwLock<VecDeque<(Pubkey, usize, Vec<Pubkey>)>>>,
    pub processed_cex: Arc<RwLock<HashSet<Pubkey>>>,
    // Runtime-only state (not serialized)
    pub processing_addresses: Arc<RwLock<HashSet<Pubkey>>>, // Track addresses currently being scanned
    pub completion_sent: Arc<AtomicBool>, // Atomic flag to prevent duplicate MaxDepthReached events
}

impl Default for SharedBfsState {
    fn default() -> Self {
        Self {
            visited_addresses: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            queue: Arc::new(RwLock::new(VecDeque::new())),
            processed_cex: Arc::new(RwLock::new(HashSet::new())),
            processing_addresses: Arc::new(RwLock::new(HashSet::new())),
            completion_sent: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl SharedBfsState {
    pub fn new(initial_address: Pubkey) -> Self {
        let mut visited_addresses = HashMap::new();
        visited_addresses.insert(initial_address, (0, vec![initial_address]));

        let mut queue = VecDeque::new();
        queue.push_back((initial_address, 0, vec![initial_address]));

        Self {
            visited_addresses: Arc::new(RwLock::new(visited_addresses)),
            history: Arc::new(RwLock::new(Vec::new())),
            queue: Arc::new(RwLock::new(queue)),
            processed_cex: Arc::new(RwLock::new(HashSet::new())),
            processing_addresses: Arc::new(RwLock::new(HashSet::new())),
            completion_sent: Arc::new(AtomicBool::new(false)),
        }
    }

    // Convert to serializable state
    pub async fn to_serializable(&self) -> SerializableBfsState {
        SerializableBfsState {
            visited_addresses: self.visited_addresses.read().await.clone(),
            history: self.history.read().await.clone(),
            queue: self.queue.read().await.clone(),
            processed_cex: self.processed_cex.read().await.clone(),
        }
    }

    // Create from serializable state
    pub fn from_serializable(serializable: SerializableBfsState) -> Self {
        Self {
            visited_addresses: Arc::new(RwLock::new(serializable.visited_addresses)),
            history: Arc::new(RwLock::new(serializable.history)),
            queue: Arc::new(RwLock::new(serializable.queue)),
            processed_cex: Arc::new(RwLock::new(serializable.processed_cex)),
            processing_addresses: Arc::new(RwLock::new(HashSet::new())), // Always start fresh
            completion_sent: Arc::new(AtomicBool::new(false)), // Always start fresh
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorMetadata {
    // Token information
    pub mint: Pubkey, // The token mint address
    pub bonding_curve: Option<Pubkey>,
    pub token_name: String,
    pub token_symbol: String,
    pub token_uri: String,

    // Account information
    pub address: Pubkey,          // The creator/account being analyzed
    pub depth: usize,             // Current depth in BFS
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
    #[serde(with = "bfs_state_serde")]
    pub bfs_state: SharedBfsState,
    pub max_depth: usize,
}

// Custom serde module for BFS state
mod bfs_state_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bfs_state: &SharedBfsState, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // For serialization, we need to use tokio::task::block_in_place or return an error
        // Since we can't use async in serde, we'll use a simpler approach
        // In practice, you might want to handle this differently based on your async runtime
        let serializable = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(bfs_state.to_serializable())
        });
        serializable.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SharedBfsState, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serializable = SerializableBfsState::deserialize(deserializer)?;
        Ok(SharedBfsState::from_serializable(serializable))
    }
}

impl CreatorMetadata {
    pub async fn new(
        mint: Pubkey,
        bonding_curve: Option<Pubkey>,
        address: Pubkey,
        max_depth: usize,
    ) -> Self {
        let cex_sources = Vec::new();
        let cex_updated_at = 0;
        let total_received = 0.0;
        let wallet_connection = SharedCreatorConnectionGraph::new();
        wallet_connection.add_node(address, false).await;

        let bfs_state = SharedBfsState::new(address);
        let now = chrono::Utc::now().timestamp() as u64;

        Self {
            mint,
            bonding_curve,
            token_name: String::new(),
            token_symbol: String::new(),
            token_uri: String::new(),
            address,
            depth: 0,
            original_creator: address,
            created_at: now,
            latest_update: now,
            retry_count: 0,
            status: AccountStatus::NewAccount,
            total_received,
            cex_sources,
            cex_updated_at,
            wallet_connection,
            bfs_state,
            max_depth,
        }
    }

    // Keep for backwards compatibility if needed, but it's a no-op now
    pub async fn initialize_bfs_state(&mut self) {
        // No-op since proper serialization preserves state
        // If you need to reset state for some reason, you can uncomment:
        // self.bfs_state = SharedBfsState::new(self.address);
    }

    // Create from NewTokenCache
    pub async fn from_token(
        token: NewTokenCache, 
        max_depth: usize
    ) -> Self {
        let metadata = Self {
            mint: token.mint,
            bonding_curve: token.bonding_curve,
            token_name: token.name,
            token_symbol: token.symbol,
            token_uri: token.uri,
            address: token.creator,
            depth: 0,
            original_creator: token.creator,
            created_at: token.created_at,
            latest_update: chrono::Utc::now().timestamp() as u64,
            retry_count: 0,
            status: AccountStatus::NewAccount,
            total_received: 0.0,
            cex_sources: Vec::new(),
            cex_updated_at: 0,
            wallet_connection: SharedCreatorConnectionGraph::new(),
            bfs_state: SharedBfsState::new(token.creator),
            max_depth,
        };
        
        // Initialize wallet connection with the creator address
        metadata.wallet_connection.add_node(token.creator, false).await;
        
        metadata
    }

    // Mark as failed and increment retry count
    pub async fn mark_as_failed(&mut self) {
        self.retry_count += 1;
        self.status = AccountStatus::Failed;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
        // Clear processing addresses since this account might be retried
        self.bfs_state.processing_addresses.write().await.clear();
        // Reset completion flag since this account might be retried
        self.reset_completion_flag();
    }

    // Mark as unprocessed (for buffer overflow)
    pub async fn mark_as_unprocessed(&mut self) {
        self.status = AccountStatus::Unprocessed;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
        // Clear processing addresses since this account will be reprocessed
        self.bfs_state.processing_addresses.write().await.clear();
        // Reset completion flag since this account will be reprocessed
        self.reset_completion_flag();
    }

    // Mark as BFS queue (failed during BFS)
    pub async fn mark_as_bfs_failed(&mut self) {
        self.status = AccountStatus::BfsQueue;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
        // Clear processing addresses since this account failed during BFS
        self.bfs_state.processing_addresses.write().await.clear();
        // Reset completion flag since this account failed during BFS
        self.reset_completion_flag();
    }

    // Helper methods for BFS operations
    pub async fn pop_from_queue(&self) -> Option<(Pubkey, usize, Vec<Pubkey>)> {
        self.bfs_state.queue.write().await.pop_front()
    }

    pub async fn push_to_queue(
        &self,
        item: (Pubkey, usize, Vec<Pubkey>),
    ) {
        self.bfs_state.queue.write().await.push_back(item);
    }

    pub async fn add_to_history(
        &self,
        address: Pubkey,
    ) {
        self.bfs_state.history.write().await.insert(0, address);
    }

    pub async fn get_history_front(&self) -> Option<Pubkey> {
        self.bfs_state.history.read().await.first().copied()
    }

    pub async fn mark_visited(
        &self,
        address: Pubkey,
        depth: usize,
        path: Vec<Pubkey>,
    ) {
        let mut visited = self.bfs_state.visited_addresses.write().await;
        
        // Check if address is already visited
        if let Some((existing_depth, _)) = visited.get(&address) {
            // Only update if the new depth is smaller (shorter path found)
            // or if it's the same depth but with a different path
            if depth <= *existing_depth {
                visited.insert(address, (depth, path));
            }
            // Don't overwrite with a larger depth - this prevents circular transfer issues
        } else {
            // First time visiting this address
            visited.insert(address, (depth, path));
        }
    }

    pub async fn get_visited(
        &self,
        address: &Pubkey,
    ) -> Option<(usize, Vec<Pubkey>)> {
        self.bfs_state.visited_addresses.read().await.get(address).cloned()
    }

    pub async fn is_visited(
        &self,
        address: &Pubkey,
    ) -> bool {
        self.bfs_state.visited_addresses.read().await.contains_key(address)
    }

    // Check if the queue is empty
    pub async fn is_queue_empty(&self) -> bool {
        self.bfs_state.queue.read().await.is_empty()
    }

    // Mark an address as currently being processed
    pub async fn mark_processing(&self, address: Pubkey) -> bool {
        let mut processing = self.bfs_state.processing_addresses.write().await;
        processing.insert(address)
    }

    // Mark an address as done processing
    pub async fn mark_done_processing(&self, address: Pubkey) -> bool {
        let mut processing = self.bfs_state.processing_addresses.write().await;
        processing.remove(&address)
    }

    // Check if an address is currently being processed
    pub async fn is_processing(&self, address: &Pubkey) -> bool {
        self.bfs_state.processing_addresses.read().await.contains(address)
    }

    // Check if an address should be skipped (either visited or currently being processed)
    pub async fn should_skip_address(&self, address: &Pubkey) -> bool {
        let visited = self.is_visited(address).await;
        let processing = self.is_processing(address).await;
        visited || processing
    }

    // Check if BFS is truly complete (no queue items AND no addresses being processed)
    pub async fn is_bfs_complete(&self) -> bool {
        let queue_empty = self.is_queue_empty().await;
        let no_processing = self.bfs_state.processing_addresses.read().await.is_empty();
        queue_empty && no_processing
    }

    // Atomically check if BFS is complete and claim the completion if it is
    // Returns true only if this thread successfully claimed the completion
    pub async fn try_claim_completion(&self) -> bool {
        if self.is_bfs_complete().await {
            // Try to atomically set completion_sent from false to true
            // This will only succeed for the first thread that calls it
            self.bfs_state.completion_sent.compare_exchange(
                false, // expected value
                true,  // new value
                Ordering::SeqCst,
                Ordering::SeqCst
            ).is_ok()
        } else {
            false
        }
    }

    // Check if completion has already been sent
    pub fn is_completion_sent(&self) -> bool {
        self.bfs_state.completion_sent.load(Ordering::SeqCst)
    }

    // Reset completion flag (for retry scenarios)
    pub fn reset_completion_flag(&self) {
        self.bfs_state.completion_sent.store(false, Ordering::SeqCst);
    }

    // Get count of addresses currently being processed (for debugging)
    pub async fn get_processing_count(&self) -> usize {
        self.bfs_state.processing_addresses.read().await.len()
    }
}

// Implement From trait for NewTokenCache
impl From<NewTokenCache> for CreatorMetadata {
    fn from(token: NewTokenCache) -> Self {
        // Note: This is a sync version, for async use from_token method
        let bfs_state = SharedBfsState::new(token.creator);
        let now = chrono::Utc::now().timestamp() as u64;

        Self {
            mint: token.mint,
            bonding_curve: token.bonding_curve,
            token_name: token.name,
            token_symbol: token.symbol,
            token_uri: token.uri,
            address: token.creator,
            depth: 0,
            original_creator: token.creator,
            created_at: token.created_at,
            latest_update: now,
            retry_count: 0,
            status: AccountStatus::NewAccount,
            total_received: 0.0,
            cex_sources: Vec::new(),
            cex_updated_at: 0,
            wallet_connection: SharedCreatorConnectionGraph::new(),
            bfs_state,
            max_depth: 0, // This should be set later
        }
    }
}
