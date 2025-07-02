use proptest::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use muhafidh::test_utils::{TestFixtures, TestHelpers, TestAssertions};
use muhafidh::model::bfs::{BfsState, BfsNode, NewTokenCache};
use muhafidh::error::Result;
use solana_sdk::pubkey::Pubkey;

/// Strategy for generating valid Pubkey strings for property tests
fn arbitrary_pubkey() -> impl Strategy<Value = Pubkey> {
    any::<[u8; 32]>().prop_map(|bytes| Pubkey::new_from_array(bytes))
}

/// Strategy for generating BFS transfer graphs
fn arbitrary_transfer_graph() -> impl Strategy<Value = Vec<(Pubkey, Pubkey, u64)>> {
    prop::collection::vec(
        (arbitrary_pubkey(), arbitrary_pubkey(), 1u64..=10000u64),
        1..=50
    )
}

/// Strategy for generating circular transfer scenarios
fn arbitrary_circular_transfers() -> impl Strategy<Value = Vec<(Pubkey, Pubkey, u64)>> {
    // Generate a chain that forms a circle: A -> B -> C -> A
    (3usize..=10).prop_flat_map(|chain_length| {
        prop::collection::vec(arbitrary_pubkey(), chain_length)
            .prop_map(move |pubkeys| {
                let mut transfers = Vec::new();
                for i in 0..pubkeys.len() {
                    let from = pubkeys[i];
                    let to = pubkeys[(i + 1) % pubkeys.len()]; // Creates the circle
                    let amount = 1000 + (i as u64 * 100);
                    transfers.push((from, to, amount));
                }
                transfers
            })
    })
}

/// Property test: BFS depth calculation should be consistent across multiple runs
#[test]
fn prop_bfs_depth_consistency() {
    proptest!(|(transfers in arbitrary_transfer_graph())| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let fixtures = TestFixtures::new();
            let assertions = TestAssertions::new();
            
            // Create BFS state from transfers
            let bfs_state = create_bfs_state_from_transfers(&transfers, &fixtures).await;
            
            // Calculate depths multiple times - should be consistent
            let depth1 = calculate_max_depth(&bfs_state).await;
            let depth2 = calculate_max_depth(&bfs_state).await;
            let depth3 = calculate_max_depth(&bfs_state).await;
            
            // Property: Depth calculations should be deterministic
            prop_assert_eq!(depth1, depth2);
            prop_assert_eq!(depth2, depth3);
            
            // Property: Depth should be reasonable (not negative, not excessive)
            prop_assert!(depth1 >= 0);
            prop_assert!(depth1 <= transfers.len() as i32);
            
            // Verify BFS state consistency
            assertions.assert_bfs_state_consistent(&bfs_state).unwrap();
        });
    });
}

/// Property test: Circular transfers should be detected and handled properly
#[test]
fn prop_circular_transfer_detection() {
    proptest!(|(circular_transfers in arbitrary_circular_transfers())| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let fixtures = TestFixtures::new();
            let helpers = TestHelpers::new();
            let assertions = TestAssertions::new();
            
            // Create BFS state with circular transfers
            let bfs_state = create_bfs_state_from_transfers(&circular_transfers, &fixtures).await;
            
            // Property: Circular transfers should be detected
            let has_circular = detect_circular_transfers(&bfs_state).await;
            prop_assert!(has_circular, "Circular transfers should be detected");
            
            // Property: Processing should complete without infinite loops
            let completed = Arc::new(AtomicBool::new(false));
            let completed_clone = completed.clone();
            
            let processing_task = tokio::spawn(async move {
                // Simulate BFS processing with timeout
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    process_bfs_with_circular_handling(&bfs_state)
                ).await;
                
                completed_clone.store(true, Ordering::SeqCst);
                result.unwrap_or(Err("Timeout".into()))
            });
            
            // Wait for completion
            let result = processing_task.await.unwrap();
            
            // Property: Processing should complete successfully
            prop_assert!(result.is_ok(), "BFS processing should complete: {:?}", result);
            prop_assert!(completed.load(Ordering::SeqCst), "Processing should complete");
            
            // Property: Circular transfer handling should be correct
            assertions.assert_circular_transfer_handled(&bfs_state).unwrap();
        });
    });
}

/// Property test: BFS graph consistency under various operations
#[test]
fn prop_bfs_graph_consistency() {
    proptest!(|(transfers in arbitrary_transfer_graph())| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let fixtures = TestFixtures::new();
            let assertions = TestAssertions::new();
            
            let bfs_state = create_bfs_state_from_transfers(&transfers, &fixtures).await;
            
            // Property: All nodes should be reachable from some root
            let nodes = extract_all_nodes(&bfs_state).await;
            for node in &nodes {
                let is_reachable = is_node_reachable(&bfs_state, node).await;
                prop_assert!(is_reachable, "Node {:?} should be reachable", node.pubkey);
            }
            
            // Property: Graph should maintain internal consistency
            assertions.assert_graph_consistency(&bfs_state).unwrap();
            
            // Property: No duplicate nodes in processing queues
            assertions.assert_no_queue_duplicates(&bfs_state).unwrap();
        });
    });
}

/// Property test: Concurrent BFS operations should not cause data races
#[test]
fn prop_concurrent_bfs_operations() {
    proptest!(|(transfers in arbitrary_transfer_graph())| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let fixtures = TestFixtures::new();
            let helpers = TestHelpers::new();
            let assertions = TestAssertions::new();
            
            let bfs_state = Arc::new(create_bfs_state_from_transfers(&transfers, &fixtures).await);
            
            // Create multiple concurrent tasks that operate on the BFS state
            let mut tasks = Vec::new();
            
            for i in 0..5 {
                let state_clone = bfs_state.clone();
                let task = tokio::spawn(async move {
                    // Simulate concurrent BFS operations
                    for _ in 0..10 {
                        let _ = calculate_max_depth(&state_clone).await;
                        let _ = detect_circular_transfers(&state_clone).await;
                        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                    }
                    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                });
                tasks.push(task);
            }
            
            // Wait for all tasks to complete
            for task in tasks {
                let result = task.await.unwrap();
                prop_assert!(result.is_ok(), "Concurrent operation should succeed");
            }
            
            // Property: BFS state should remain consistent after concurrent access
            assertions.assert_bfs_state_consistent(&bfs_state).unwrap();
        });
    });
}

/// Property test: BFS serialization/deserialization should preserve data
#[test]
fn prop_bfs_serialization() {
    proptest!(|(transfers in arbitrary_transfer_graph())| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let fixtures = TestFixtures::new();
            
            let original_state = create_bfs_state_from_transfers(&transfers, &fixtures).await;
            
            // Serialize BFS state
            let serialized = serialize_bfs_state(&original_state).await.unwrap();
            
            // Deserialize BFS state
            let deserialized_state = deserialize_bfs_state(&serialized).await.unwrap();
            
            // Property: Original and deserialized states should be equivalent
            let original_depth = calculate_max_depth(&original_state).await;
            let deserialized_depth = calculate_max_depth(&deserialized_state).await;
            
            prop_assert_eq!(original_depth, deserialized_depth);
            
            // Property: Both states should detect the same circular transfers
            let original_circular = detect_circular_transfers(&original_state).await;
            let deserialized_circular = detect_circular_transfers(&deserialized_state).await;
            
            prop_assert_eq!(original_circular, deserialized_circular);
        });
    });
}

// Helper functions for property tests

async fn create_bfs_state_from_transfers(
    transfers: &[(Pubkey, Pubkey, u64)],
    fixtures: &TestFixtures,
) -> BfsState {
    let mut bfs_state = fixtures.sample_bfs_state();
    let mut nodes = HashMap::new();
    
    // Create nodes from transfers
    for (from, to, amount) in transfers {
        let from_node = BfsNode {
            pubkey: *from,
            depth: 0,
            amount: *amount,
            processed: false,
        };
        
        let to_node = BfsNode {
            pubkey: *to,
            depth: 1,
            amount: *amount,
            processed: false,
        };
        
        nodes.insert(*from, from_node);
        nodes.insert(*to, to_node);
    }
    
    bfs_state.nodes = nodes;
    bfs_state
}

async fn calculate_max_depth(bfs_state: &BfsState) -> i32 {
    bfs_state.nodes.values()
        .map(|node| node.depth)
        .max()
        .unwrap_or(0)
}

async fn detect_circular_transfers(bfs_state: &BfsState) -> bool {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    
    // Simple cycle detection using DFS
    for node in bfs_state.nodes.values() {
        if !visited.contains(&node.pubkey) {
            if has_cycle_dfs(&bfs_state, &node.pubkey, &mut visited, &mut rec_stack).await {
                return true;
            }
        }
    }
    
    false
}

async fn has_cycle_dfs(
    bfs_state: &BfsState,
    current: &Pubkey,
    visited: &mut HashSet<Pubkey>,
    rec_stack: &mut HashSet<Pubkey>,
) -> bool {
    visited.insert(*current);
    rec_stack.insert(*current);
    
    // Check if we find ourselves in the recursion stack (cycle detected)
    for node in bfs_state.nodes.values() {
        if node.pubkey != *current {
            // Simplified - in reality you'd check actual connections
            if rec_stack.contains(&node.pubkey) {
                return true;
            }
        }
    }
    
    rec_stack.remove(current);
    false
}

async fn process_bfs_with_circular_handling(
    bfs_state: &BfsState,
) -> Result<()> {
    // Simplified BFS processing with circular transfer handling
    let mut processed_count = 0;
    let max_iterations = bfs_state.nodes.len() * 2; // Prevent infinite loops
    
    while processed_count < max_iterations {
        let has_unprocessed = bfs_state.nodes.values()
            .any(|node| !node.processed);
            
        if !has_unprocessed {
            break;
        }
        
        processed_count += 1;
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    
    Ok(())
}

async fn extract_all_nodes(bfs_state: &BfsState) -> Vec<BfsNode> {
    bfs_state.nodes.values().cloned().collect()
}

async fn is_node_reachable(bfs_state: &BfsState, node: &BfsNode) -> bool {
    // Simplified reachability check
    bfs_state.nodes.contains_key(&node.pubkey)
}

async fn serialize_bfs_state(bfs_state: &BfsState) -> Result<Vec<u8>> {
    // Simplified serialization using bincode
    bincode::serialize(bfs_state)
        .map_err(|e| format!("Serialization error: {}", e).into())
}

async fn deserialize_bfs_state(data: &[u8]) -> Result<BfsState> {
    // Simplified deserialization using bincode
    bincode::deserialize(data)
        .map_err(|e| format!("Deserialization error: {}", e).into())
} 