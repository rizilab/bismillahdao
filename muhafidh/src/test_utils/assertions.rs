use std::collections::HashSet;
use solana_pubkey::Pubkey;
use crate::model::creator::metadata::{CreatorMetadata, AccountStatus};
use crate::model::creator::graph::CreatorConnectionGraph;

/// Custom assertions for domain-specific testing
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that BFS state is consistent
    pub async fn assert_bfs_state_consistent(metadata: &CreatorMetadata) {
        let visited = metadata.bfs_state.visited_addresses.read().await;
        let queue = metadata.bfs_state.queue.read().await;
        let processing = metadata.bfs_state.processing_addresses.read().await;
        
        // Assert: All queue addresses should have valid depth progression
        for (addr, depth, path) in queue.iter() {
            assert!(*depth <= metadata.max_depth, 
                "Queue item {} has depth {} exceeding max_depth {}", 
                addr, depth, metadata.max_depth);
            
            assert!(!path.is_empty(), "Queue item {} has empty path", addr);
            assert_eq!(path.last(), Some(addr), "Queue item {} path doesn't end with itself", addr);
        }
        
        // Assert: Visited addresses should have consistent depth and path
        for (addr, (depth, path)) in visited.iter() {
            assert!(*depth <= metadata.max_depth, 
                "Visited address {} has depth {} exceeding max_depth {}", 
                addr, depth, metadata.max_depth);
            
            assert!(!path.is_empty(), "Visited address {} has empty path", addr);
            assert_eq!(path.last(), Some(addr), "Visited address {} path doesn't end with itself", addr);
        }
        
        // Assert: Processing addresses should exist in either visited or queue
        for addr in processing.iter() {
            let in_visited = visited.contains_key(addr);
            let in_queue = queue.iter().any(|(q_addr, _, _)| q_addr == addr);
            
            assert!(in_visited || in_queue, 
                "Processing address {} not found in visited or queue", addr);
        }
    }

    /// Assert that completion can only be claimed once
    pub async fn assert_completion_atomicity(metadata: &CreatorMetadata) {
        // Ensure BFS is actually complete
        assert!(metadata.is_bfs_complete().await, "BFS is not complete");
        
        // Try to claim completion multiple times
        let first_claim = metadata.try_claim_completion().await;
        let second_claim = metadata.try_claim_completion().await;
        
        assert!(first_claim, "First completion claim should succeed");
        assert!(!second_claim, "Second completion claim should fail");
        assert!(metadata.is_completion_sent(), "Completion flag should be set");
    }

    /// Assert circular transfer handling
    pub async fn assert_circular_transfer_handled(
        metadata: &CreatorMetadata,
        addr_a: Pubkey,
        addr_b: Pubkey,
    ) {
        let visited = metadata.bfs_state.visited_addresses.read().await;
        
        if let (Some((depth_a, _)), Some((depth_b, _))) = (visited.get(&addr_a), visited.get(&addr_b)) {
            // In circular transfers, the first visited should maintain lower depth
            if *depth_a < *depth_b {
                // A was visited first, B should reference A in its path
                if let Some((_, path_b)) = visited.get(&addr_b) {
                    assert!(path_b.contains(&addr_a), 
                        "Circular transfer: B's path should contain A");
                }
            }
        }
    }

    /// Assert that the graph is consistent
    pub fn assert_graph_consistency(graph: &CreatorConnectionGraph) {
        let nodes = graph.get_node_count();
        let edges = graph.get_edge_count();
        
        assert!(nodes > 0, "Graph should have at least one node");
        
        // Assert that edges don't exceed theoretical maximum for nodes
        let max_edges = nodes * (nodes - 1);
        assert!(edges <= max_edges, 
            "Edge count {} exceeds maximum possible {} for {} nodes", 
            edges, max_edges, nodes);
    }

    /// Assert that account status transitions are valid
    pub fn assert_valid_status_transition(from: AccountStatus, to: AccountStatus) {
        match (from, to) {
            // Valid transitions
            (AccountStatus::NewAccount, AccountStatus::Failed) => {},
            (AccountStatus::NewAccount, AccountStatus::Unprocessed) => {},
            (AccountStatus::NewAccount, AccountStatus::BfsQueue) => {},
            (AccountStatus::Failed, AccountStatus::NewAccount) => {}, // Retry
            (AccountStatus::Failed, AccountStatus::Failed) => {}, // Re-fail
            (AccountStatus::Unprocessed, AccountStatus::NewAccount) => {}, // Reprocess
            (AccountStatus::Unprocessed, AccountStatus::Failed) => {},
            (AccountStatus::BfsQueue, AccountStatus::Failed) => {},
            (AccountStatus::BfsQueue, AccountStatus::NewAccount) => {}, // Retry
            
            // Invalid transitions
            _ => panic!("Invalid status transition from {:?} to {:?}", from, to),
        }
    }

    /// Assert that retry count is within reasonable bounds
    pub fn assert_retry_count_valid(metadata: &CreatorMetadata) {
        assert!(metadata.retry_count <= 10, 
            "Retry count {} exceeds reasonable maximum", metadata.retry_count);
        
        // If retry count > 0, status should be Failed
        if metadata.retry_count > 0 {
            assert_eq!(metadata.status, AccountStatus::Failed,
                "Account with retry_count > 0 should have Failed status");
        }
    }

    /// Assert that depth progression is monotonic
    pub async fn assert_depth_progression(metadata: &CreatorMetadata, addresses: &[Pubkey]) {
        let visited = metadata.bfs_state.visited_addresses.read().await;
        
        for window in addresses.windows(2) {
            if let (Some((depth1, _)), Some((depth2, _))) = 
                (visited.get(&window[0]), visited.get(&window[1])) {
                
                assert!(*depth2 >= *depth1, 
                    "Depth should be non-decreasing: {} (depth {}) -> {} (depth {})",
                    window[0], depth1, window[1], depth2);
            }
        }
    }

    /// Assert that no duplicate addresses exist in queue
    pub async fn assert_no_queue_duplicates(metadata: &CreatorMetadata) {
        let queue = metadata.bfs_state.queue.read().await;
        let mut seen = HashSet::new();
        
        for (addr, _, _) in queue.iter() {
            assert!(seen.insert(*addr), "Duplicate address {} found in queue", addr);
        }
    }

    /// Assert that processing state is clean after operations
    pub async fn assert_clean_processing_state(metadata: &CreatorMetadata) {
        let processing = metadata.bfs_state.processing_addresses.read().await;
        assert!(processing.is_empty(), 
            "Processing addresses should be empty after operations: {:?}", 
            processing.iter().collect::<Vec<_>>());
    }

    /// Assert that timestamps are reasonable
    pub fn assert_reasonable_timestamps(metadata: &CreatorMetadata) {
        let now = chrono::Utc::now().timestamp() as u64;
        let one_year_ago = now - (365 * 24 * 60 * 60);
        
        assert!(metadata.created_at >= one_year_ago && metadata.created_at <= now,
            "Created timestamp {} is not reasonable", metadata.created_at);
        
        assert!(metadata.latest_update >= metadata.created_at,
            "Latest update {} should be >= created_at {}", 
            metadata.latest_update, metadata.created_at);
        
        assert!(metadata.latest_update <= now,
            "Latest update {} should not be in the future", metadata.latest_update);
    }
} 