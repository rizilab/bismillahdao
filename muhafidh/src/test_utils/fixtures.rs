use std::collections::{HashMap, HashSet, VecDeque};
use solana_pubkey::Pubkey;
use fake::{Fake, Faker};
use crate::model::creator::metadata::{CreatorMetadata, AccountStatus, SharedBfsState, SerializableBfsState};
use crate::model::creator::graph::SharedCreatorConnectionGraph;
use crate::storage::redis::model::NewTokenCache;

/// Test fixtures for creating consistent test data
pub struct TestFixtures;

impl TestFixtures {
    /// Create a sample Pubkey for testing
    pub fn sample_pubkey() -> Pubkey {
        Pubkey::new_unique()
    }

    /// Create multiple sample pubkeys
    pub fn sample_pubkeys(count: usize) -> Vec<Pubkey> {
        (0..count).map(|_| Pubkey::new_unique()).collect()
    }

    /// Create a test token cache
    pub fn sample_new_token_cache() -> NewTokenCache {
        let creator = Self::sample_pubkey();
        NewTokenCache {
            mint: Self::sample_pubkey(),
            name: Faker.fake::<String>(),
            symbol: "TEST".to_string(),
            uri: "https://example.com/metadata.json".to_string(),
            creator,
            bonding_curve: Some(Self::sample_pubkey()),
            created_at: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Create a test creator metadata with minimal setup
    pub async fn sample_creator_metadata() -> CreatorMetadata {
        let token = Self::sample_new_token_cache();
        CreatorMetadata::from_token(token, 5).await
    }

    /// Create a creator metadata with specific status
    pub async fn creator_metadata_with_status(status: AccountStatus) -> CreatorMetadata {
        let mut metadata = Self::sample_creator_metadata().await;
        metadata.status = status;
        metadata
    }

    /// Create a creator metadata with populated BFS state
    pub async fn creator_metadata_with_bfs_history(addresses: Vec<Pubkey>) -> CreatorMetadata {
        let mut metadata = Self::sample_creator_metadata().await;
        
        // Populate visited addresses
        for (i, addr) in addresses.iter().enumerate() {
            metadata.bfs_state.visited_addresses.write().await.insert(*addr, (i, vec![*addr]));
        }
        
        // Add to history
        for addr in addresses.iter().rev() {
            metadata.bfs_state.history.write().await.push(*addr);
        }
        
        metadata
    }

    /// Create a BFS state with circular transfer scenario
    pub async fn bfs_state_with_circular_transfers() -> SharedBfsState {
        let mut visited = HashMap::new();
        let addr_a = Self::sample_pubkey();
        let addr_b = Self::sample_pubkey();
        
        // A -> B at depth 1
        visited.insert(addr_a, (1, vec![addr_a]));
        // B -> A at depth 2 (circular)
        visited.insert(addr_b, (2, vec![addr_a, addr_b]));
        
        let mut queue = VecDeque::new();
        queue.push_back((addr_b, 2, vec![addr_a, addr_b]));
        
        SharedBfsState {
            visited_addresses: std::sync::Arc::new(tokio::sync::RwLock::new(visited)),
            history: std::sync::Arc::new(tokio::sync::RwLock::new(vec![addr_b, addr_a])),
            queue: std::sync::Arc::new(tokio::sync::RwLock::new(queue)),
            processed_cex: std::sync::Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            processing_addresses: std::sync::Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            completion_sent: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Create a serializable BFS state for testing serialization
    pub fn sample_serializable_bfs_state() -> SerializableBfsState {
        let mut visited = HashMap::new();
        let addresses = Self::sample_pubkeys(3);
        
        for (i, addr) in addresses.iter().enumerate() {
            visited.insert(*addr, (i, vec![*addr]));
        }
        
        let mut queue = VecDeque::new();
        queue.push_back((addresses[0], 0, vec![addresses[0]]));
        
        SerializableBfsState {
            visited_addresses: visited,
            history: addresses.clone(),
            queue,
            processed_cex: HashSet::new(),
        }
    }
} 