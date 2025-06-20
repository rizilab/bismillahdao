use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;

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

#[derive(Debug, Clone, Default)]
pub struct SharedBfsState {
    pub visited_addresses: Arc<RwLock<HashMap<Pubkey, (usize, Vec<Pubkey>)>>>,
    pub history: Arc<RwLock<Vec<Pubkey>>>,
    pub queue: Arc<RwLock<VecDeque<(Pubkey, usize, Vec<Pubkey>)>>>,
    pub processed_cex: Arc<RwLock<HashSet<Pubkey>>>,
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

    // BFS state - using SharedBfsState for async mutability
    #[serde(skip)]
    pub bfs_state: SharedBfsState,
    pub max_depth: usize,
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

    // Create from NewTokenCache
    pub async fn from_token(
        token: NewTokenCache,
        max_depth: usize,
    ) -> Self {
        let mut metadata = Self::new(token.mint, token.bonding_curve, token.creator, max_depth).await;
        metadata.token_name = token.name;
        metadata.token_symbol = token.symbol;
        metadata.token_uri = token.uri;
        metadata.created_at = token.created_at;

        // Ensure the initial creator is marked as visited at depth 0
        metadata.mark_visited(token.creator, 0, vec![token.creator]).await;

        metadata
    }

    // Mark as failed and increment retry count
    pub fn mark_as_failed(&mut self) {
        self.retry_count += 1;
        self.status = AccountStatus::Failed;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
    }

    // Mark as unprocessed (for buffer overflow)
    pub fn mark_as_unprocessed(&mut self) {
        self.status = AccountStatus::Unprocessed;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
    }

    // Mark as BFS queue (failed during BFS)
    pub fn mark_as_bfs_failed(&mut self) {
        self.status = AccountStatus::BfsQueue;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
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
        self.bfs_state.visited_addresses.write().await.insert(address, (depth, path));
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
