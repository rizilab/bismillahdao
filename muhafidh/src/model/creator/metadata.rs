use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use solana_pubkey::Pubkey;
use tokio::sync::RwLock;

use super::graph::SharedCreatorCexConnectionGraph;

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
    pub mint: Pubkey,
    pub address: Pubkey,
    pub total_received: f64,
    pub cex_sources: Vec<Pubkey>,
    pub cex_updated_at: u64,
    pub wallet_connection: SharedCreatorCexConnectionGraph,
    // BFS state - using SharedBfsState for async mutability
    #[serde(skip)]
    pub bfs_state: SharedBfsState,
    pub max_depth: usize,
}

impl CreatorMetadata {
    pub async fn new(
        mint: Pubkey,
        address: Pubkey,
        max_depth: usize,
    ) -> Self {
        let cex_sources = Vec::new();
        let cex_updated_at = 0;
        let total_received = 0.0;
        let wallet_connection = SharedCreatorCexConnectionGraph::new();
        wallet_connection.add_node(address, false).await;

        let bfs_state = SharedBfsState::new(address);

        Self {
            mint,
            address,
            total_received,
            cex_sources,
            cex_updated_at,
            wallet_connection,
            bfs_state,
            max_depth,
        }
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
