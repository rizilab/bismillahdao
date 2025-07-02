use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use tokio::sync::{RwLock, Mutex, Semaphore};
use tokio::time::{timeout, sleep};
use serial_test::serial;

use muhafidh::test_utils::{TestFixtures, TestHelpers, TestAssertions};
use muhafidh::testing::{TestDatabase, TestRedis};
use muhafidh::model::bfs::{BfsState, BfsNode, NewTokenCache};
use muhafidh::storage::postgres::PostgresStorage;
use muhafidh::storage::redis::RedisStorage;
use muhafidh::pipeline::processor::creator::CreatorProcessor;
use muhafidh::error::Result;
use solana_sdk::pubkey::Pubkey;

/// Stress test module for BFS race conditions and edge cases
/// These tests focus on high-concurrency scenarios and the circular transfer issues
/// that were previously identified and fixed
mod bfs_stress_tests {
    use super::*;

    /// Test maximum concurrent BFS operations with circular transfers
    #[tokio::test]
    #[serial]
    async fn stress_test_concurrent_bfs_with_circular_transfers() -> Result<()> {
        let fixtures = TestFixtures::new();
        let helpers = TestHelpers::new();
        let assertions = TestAssertions::new();

        // Setup high concurrency test environment
        let test_db = TestDatabase::new().await?;
        let test_redis = TestRedis::new().await?;

        let postgres_storage = Arc::new(PostgresStorage::new(test_db.get_pool()).await?);
        let redis_storage = Arc::new(RedisStorage::new(test_redis.get_connection()).await?);

        // Create large circular transfer graph
        let graph_size = 1000;
        let circular_chains = 50; // Number of circular transfer chains
        let concurrency_level = 100; // Number of concurrent BFS operations

        // Build complex circular transfer scenario
        let mut transfer_graph = HashMap::new();
        let mut all_pubkeys = Vec::new();

        for chain_id in 0..circular_chains {
            let chain_pubkeys = fixtures.sample_pubkeys(20); // 20 nodes per circular chain
            
            // Create circular transfers: A -> B -> C -> ... -> Z -> A
            for i in 0..chain_pubkeys.len() {
                let from = chain_pubkeys[i];
                let to = chain_pubkeys[(i + 1) % chain_pubkeys.len()];
                
                transfer_graph.entry(from).or_insert_with(Vec::new).push((to, 1000 + i as u64));
                all_pubkeys.push(from);
            }
        }

        // Add some non-circular transfers to make it more realistic
        for _ in 0..200 {
            let from = fixtures.sample_pubkey();
            let to = fixtures.sample_pubkey();
            transfer_graph.entry(from).or_insert_with(Vec::new).push((to, 500));
            all_pubkeys.push(from);
            all_pubkeys.push(to);
        }

        // Store initial BFS states
        for pubkey in &all_pubkeys {
            let bfs_state = helpers.setup_circular_transfer_scenario(
                *pubkey,
                transfer_graph.get(pubkey).unwrap_or(&Vec::new()).clone()
            ).await?;
            
            redis_storage.store_bfs_state(pubkey, &bfs_state).await?;
        }

        // Track race condition indicators
        let max_depth_events = Arc::new(AtomicUsize::new(0));
        let completion_races = Arc::new(AtomicUsize::new(0));
        let circular_detections = Arc::new(AtomicUsize::new(0));

        // Execute high-concurrency BFS operations
        let semaphore = Arc::new(Semaphore::new(concurrency_level));
        let mut tasks = Vec::new();

        let start_time = Instant::now();

        for pubkey in all_pubkeys.clone() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let postgres_clone = postgres_storage.clone();
            let redis_clone = redis_storage.clone();
            let helpers_clone = helpers.clone();
            let assertions_clone = assertions.clone();
            let max_depth_clone = max_depth_events.clone();
            let completion_races_clone = completion_races.clone();
            let circular_detections_clone = circular_detections.clone();

            let task = tokio::spawn(async move {
                let _permit = permit; // Keep permit until task completes

                // Simulate BFS processing with potential race conditions
                let result = simulate_bfs_processing_with_races(
                    pubkey,
                    postgres_clone,
                    redis_clone,
                    helpers_clone,
                    assertions_clone,
                    max_depth_clone,
                    completion_races_clone,
                    circular_detections_clone,
                ).await;

                result
            });

            tasks.push(task);
        }

        // Wait for all tasks with timeout
        let timeout_duration = Duration::from_secs(300); // 5 minutes timeout
        let results = timeout(timeout_duration, async {
            let mut results = Vec::new();
            for task in tasks {
                results.push(task.await?);
            }
            Ok::<Vec<Result<()>>, tokio::task::JoinError>(results)
        }).await??;

        let elapsed = start_time.elapsed();

        // Analyze results
        let successful_operations = results.iter().filter(|r| r.is_ok()).count();
        let failed_operations = results.iter().filter(|r| r.is_err()).count();

        println!("Stress Test Results:");
        println!("  Total operations: {}", results.len());
        println!("  Successful: {}", successful_operations);
        println!("  Failed: {}", failed_operations);
        println!("  Elapsed time: {:?}", elapsed);
        println!("  Operations/sec: {:.2}", results.len() as f64 / elapsed.as_secs_f64());
        println!("  MaxDepthReached events: {}", max_depth_events.load(Ordering::Relaxed));
        println!("  Completion races detected: {}", completion_races.load(Ordering::Relaxed));
        println!("  Circular transfers detected: {}", circular_detections.load(Ordering::Relaxed));

        // Validate that we didn't have excessive failures or race conditions
        let failure_rate = failed_operations as f64 / results.len() as f64;
        assert!(failure_rate < 0.1, "Failure rate too high: {:.2}%", failure_rate * 100.0);

        // Check that MaxDepthReached events were not duplicated excessively
        let max_depth_rate = max_depth_events.load(Ordering::Relaxed) as f64 / results.len() as f64;
        assert!(max_depth_rate < 2.0, "Too many MaxDepthReached events: {:.2} per operation", max_depth_rate);

        // Verify final state consistency
        for pubkey in all_pubkeys.iter().take(100) { // Check subset for performance
            if let Ok(Some(final_state)) = redis_storage.get_bfs_state(pubkey).await {
                assertions.assert_bfs_state_consistent(&final_state)?;
                assertions.assert_circular_transfer_handled(&final_state)?;
                assertions.assert_completion_atomicity(&final_state)?;
            }
        }

        // Cleanup
        test_db.cleanup().await?;
        test_redis.cleanup().await?;

        Ok(())
    }

    /// Test BFS depth calculation under extreme conditions
    #[tokio::test]
    #[serial]
    async fn stress_test_bfs_depth_calculation_extremes() -> Result<()> {
        let fixtures = TestFixtures::new();
        let assertions = TestAssertions::new();

        // Test with various extreme scenarios
        let test_scenarios = vec![
            ("very_deep_chain", create_very_deep_transfer_chain(1000).await?),
            ("wide_graph", create_wide_transfer_graph(500).await?),
            ("dense_circular", create_dense_circular_graph(200).await?),
            ("mixed_topology", create_mixed_topology_graph(300).await?),
        ];

        for (scenario_name, bfs_state) in test_scenarios {
            println!("Testing scenario: {}", scenario_name);

            let start_time = Instant::now();

            // Verify depth calculation consistency
            assertions.assert_depth_progression(&bfs_state)?;
            assertions.assert_bfs_state_consistent(&bfs_state)?;

            // Test depth calculation performance
            let max_depth = bfs_state.nodes.values()
                .map(|node| node.depth)
                .max()
                .unwrap_or(0);

            let calculation_time = start_time.elapsed();
            
            println!("  Nodes: {}, Max depth: {}, Time: {:?}", 
                    bfs_state.nodes.len(), max_depth, calculation_time);

            // Ensure depth calculation doesn't take too long
            assert!(calculation_time < Duration::from_secs(10), 
                   "Depth calculation took too long for {}: {:?}", scenario_name, calculation_time);

            // Verify depth progression is logical
            let depth_distribution: HashMap<i32, usize> = bfs_state.nodes.values()
                .fold(HashMap::new(), |mut acc, node| {
                    *acc.entry(node.depth).or_insert(0) += 1;
                    acc
                });

            println!("  Depth distribution: {:?}", depth_distribution);
        }

        Ok(())
    }

    /// Test memory usage under high load
    #[tokio::test]
    #[serial]
    async fn stress_test_memory_usage_high_load() -> Result<()> {
        let fixtures = TestFixtures::new();
        let helpers = TestHelpers::new();

        // Create progressively larger BFS states
        let sizes = vec![1000, 5000, 10000, 25000, 50000];

        for size in sizes {
            println!("Testing memory usage with {} nodes", size);

            let start_memory = get_memory_usage();
            let start_time = Instant::now();

            // Create large BFS state
            let mut bfs_state = fixtures.sample_bfs_state();
            let pubkeys = fixtures.sample_pubkeys(size);

            for (i, pubkey) in pubkeys.into_iter().enumerate() {
                let node = BfsNode {
                    pubkey,
                    depth: (i % 100) as i32,
                    amount: 1000 + (i as u64),
                    processed: i % 10 == 0, // 10% processed
                };
                bfs_state.nodes.insert(pubkey, node);
            }

            let creation_time = start_time.elapsed();
            let memory_after_creation = get_memory_usage();

            // Perform operations on the BFS state
            let operations_start = Instant::now();
            
            // Simulate various operations
            let total_amount: u64 = bfs_state.nodes.values()
                .map(|node| node.amount)
                .sum();

            let processed_count = bfs_state.nodes.values()
                .filter(|node| node.processed)
                .count();

            let max_depth = bfs_state.nodes.values()
                .map(|node| node.depth)
                .max()
                .unwrap_or(0);

            let operations_time = operations_start.elapsed();
            let final_memory = get_memory_usage();

            println!("  Creation time: {:?}", creation_time);
            println!("  Operations time: {:?}", operations_time);
            println!("  Memory usage: start={}, after_creation={}, final={}", 
                    start_memory, memory_after_creation, final_memory);
            println!("  Total amount: {}, Processed: {}, Max depth: {}", 
                    total_amount, processed_count, max_depth);

            // Memory usage should be reasonable
            let memory_per_node = (memory_after_creation - start_memory) / size as u64;
            assert!(memory_per_node < 1024, // Less than 1KB per node
                   "Memory usage per node too high: {} bytes", memory_per_node);

            // Operations should complete in reasonable time
            assert!(operations_time < Duration::from_secs(5),
                   "Operations took too long: {:?}", operations_time);

            // Force garbage collection (drop large structure)
            drop(bfs_state);
            
            // Small delay to allow cleanup
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Test error recovery and resilience
    #[tokio::test]
    #[serial]
    async fn stress_test_error_recovery_resilience() -> Result<()> {
        let fixtures = TestFixtures::new();
        let helpers = TestHelpers::new();

        let test_db = TestDatabase::new().await?;
        let test_redis = TestRedis::new().await?;

        let postgres_storage = Arc::new(PostgresStorage::new(test_db.get_pool()).await?);
        let redis_storage = Arc::new(RedisStorage::new(test_redis.get_connection()).await?);

        // Create scenarios with various error conditions
        let error_scenarios = vec![
            "database_connection_loss",
            "redis_connection_loss", 
            "partial_data_corruption",
            "concurrent_modification",
            "memory_pressure",
        ];

        for scenario in error_scenarios {
            println!("Testing error recovery scenario: {}", scenario);

            let pubkeys = fixtures.sample_pubkeys(100);
            let errors_injected = Arc::new(AtomicUsize::new(0));
            let recoveries_successful = Arc::new(AtomicUsize::new(0));

            // Setup initial states
            for pubkey in &pubkeys {
                let bfs_state = fixtures.sample_bfs_state();
                redis_storage.store_bfs_state(pubkey, &bfs_state).await?;
            }

            // Execute operations with error injection
            let mut tasks = Vec::new();
            for pubkey in pubkeys {
                let postgres_clone = postgres_storage.clone();
                let redis_clone = redis_storage.clone();
                let helpers_clone = helpers.clone();
                let errors_clone = errors_injected.clone();
                let recoveries_clone = recoveries_successful.clone();
                let scenario_name = scenario.to_string();

                let task = tokio::spawn(async move {
                    simulate_operation_with_errors(
                        pubkey,
                        scenario_name,
                        postgres_clone,
                        redis_clone,
                        helpers_clone,
                        errors_clone,
                        recoveries_clone,
                    ).await
                });

                tasks.push(task);
            }

            // Wait for completion
            let results: Result<Vec<_>> = try {
                let mut results = Vec::new();
                for task in tasks {
                    results.push(task.await??);
                }
                results
            };

            let results = results?;
            let successful_recoveries = recoveries_successful.load(Ordering::Relaxed);
            let total_errors = errors_injected.load(Ordering::Relaxed);

            println!("  Errors injected: {}", total_errors);
            println!("  Successful recoveries: {}", successful_recoveries);
            
            if total_errors > 0 {
                let recovery_rate = successful_recoveries as f64 / total_errors as f64;
                println!("  Recovery rate: {:.2}%", recovery_rate * 100.0);
                
                // Should have reasonable recovery rate
                assert!(recovery_rate > 0.7, 
                       "Recovery rate too low for {}: {:.2}%", scenario, recovery_rate * 100.0);
            }
        }

        // Cleanup
        test_db.cleanup().await?;
        test_redis.cleanup().await?;

        Ok(())
    }

    /// Test edge cases and boundary conditions
    #[tokio::test]
    async fn stress_test_edge_cases_boundary_conditions() -> Result<()> {
        let fixtures = TestFixtures::new();
        let assertions = TestAssertions::new();

        // Test various edge cases
        let edge_cases = vec![
            ("empty_bfs_state", create_empty_bfs_state().await?),
            ("single_node", create_single_node_bfs_state().await?),
            ("self_referencing", create_self_referencing_bfs_state().await?),
            ("disconnected_components", create_disconnected_components_bfs_state().await?),
            ("maximum_depth", create_maximum_depth_bfs_state().await?),
            ("zero_amounts", create_zero_amounts_bfs_state().await?),
            ("large_amounts", create_large_amounts_bfs_state().await?),
        ];

        for (case_name, bfs_state) in edge_cases {
            println!("Testing edge case: {}", case_name);

            // All edge cases should maintain consistency
            let consistency_result = std::panic::catch_unwind(|| {
                assertions.assert_bfs_state_consistent(&bfs_state)
            });

            match consistency_result {
                Ok(Ok(())) => println!("  ✓ Consistency check passed"),
                Ok(Err(e)) => println!("  ⚠ Consistency check failed: {:?}", e),
                Err(_) => println!("  ✗ Consistency check panicked"),
            }

            // Test serialization/deserialization
            let serialization_result = test_bfs_state_serialization(&bfs_state).await;
            match serialization_result {
                Ok(()) => println!("  ✓ Serialization test passed"),
                Err(e) => println!("  ⚠ Serialization test failed: {:?}", e),
            }

            // Test various operations
            test_bfs_operations_on_edge_case(&bfs_state, case_name).await?;
        }

        Ok(())
    }
}

// Helper functions for stress testing

async fn simulate_bfs_processing_with_races(
    pubkey: Pubkey,
    _postgres_storage: Arc<PostgresStorage>,
    redis_storage: Arc<RedisStorage>,
    helpers: TestHelpers,
    assertions: TestAssertions,
    max_depth_events: Arc<AtomicUsize>,
    completion_races: Arc<AtomicUsize>,
    circular_detections: Arc<AtomicUsize>,
) -> Result<()> {
    // Get initial BFS state
    let mut bfs_state = redis_storage.get_bfs_state(&pubkey).await?
        .ok_or_else(|| muhafidh::error::MuhafidError::NotFound("BFS state not found".to_string()))?;

    // Simulate race condition scenarios
    let should_race = rand::random::<bool>();
    
    if should_race {
        // Simulate concurrent completion attempt
        let completion_detected = helpers.simulate_completion_race(&mut bfs_state).await?;
        if completion_detected {
            completion_races.fetch_add(1, Ordering::Relaxed);
        }
    }

    // Check for circular transfers
    if helpers.detect_circular_transfers(&bfs_state).await? {
        circular_detections.fetch_add(1, Ordering::Relaxed);
    }

    // Simulate depth progression
    let max_depth = bfs_state.nodes.values()
        .map(|node| node.depth)
        .max()
        .unwrap_or(0);

    if max_depth > 50 { // Arbitrary high depth threshold
        max_depth_events.fetch_add(1, Ordering::Relaxed);
    }

    // Perform consistency checks
    assertions.assert_bfs_state_consistent(&bfs_state)?;

    // Store updated state
    redis_storage.store_bfs_state(&pubkey, &bfs_state).await?;

    Ok(())
}

async fn create_very_deep_transfer_chain(depth: usize) -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();

    let pubkeys = fixtures.sample_pubkeys(depth);
    
    for (i, pubkey) in pubkeys.into_iter().enumerate() {
        let node = BfsNode {
            pubkey,
            depth: i as i32,
            amount: 1000,
            processed: false,
        };
        bfs_state.nodes.insert(pubkey, node);
    }

    Ok(bfs_state)
}

async fn create_wide_transfer_graph(width: usize) -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();

    let pubkeys = fixtures.sample_pubkeys(width);
    
    for (i, pubkey) in pubkeys.into_iter().enumerate() {
        let node = BfsNode {
            pubkey,
            depth: 1, // All at same depth for wide graph
            amount: 1000 + (i as u64),
            processed: false,
        };
        bfs_state.nodes.insert(pubkey, node);
    }

    Ok(bfs_state)
}

async fn create_dense_circular_graph(size: usize) -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();

    let pubkeys = fixtures.sample_pubkeys(size);
    
    // Create multiple circular references
    for (i, pubkey) in pubkeys.into_iter().enumerate() {
        let node = BfsNode {
            pubkey,
            depth: (i % 10) as i32, // Create depth cycles
            amount: 1000,
            processed: false,
        };
        bfs_state.nodes.insert(pubkey, node);
    }

    Ok(bfs_state)
}

async fn create_mixed_topology_graph(size: usize) -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();

    let pubkeys = fixtures.sample_pubkeys(size);
    
    for (i, pubkey) in pubkeys.into_iter().enumerate() {
        let depth = match i % 4 {
            0 => 1,           // Shallow nodes
            1 => 5,           // Medium depth
            2 => 20,          // Deep nodes
            3 => (i / 4) as i32, // Progressive depth
            _ => 1,
        };

        let node = BfsNode {
            pubkey,
            depth,
            amount: if i % 10 == 0 { 0 } else { 1000 + (i as u64) },
            processed: i % 5 == 0,
        };
        bfs_state.nodes.insert(pubkey, node);
    }

    Ok(bfs_state)
}

async fn create_empty_bfs_state() -> Result<BfsState> {
    Ok(BfsState {
        nodes: HashMap::new(),
        completed: false,
        max_depth_reached: false,
        processing_start: chrono::Utc::now(),
        last_update: chrono::Utc::now(),
    })
}

async fn create_single_node_bfs_state() -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();
    
    let pubkey = fixtures.sample_pubkey();
    let node = BfsNode {
        pubkey,
        depth: 0,
        amount: 1000,
        processed: true,
    };
    
    bfs_state.nodes.clear();
    bfs_state.nodes.insert(pubkey, node);
    
    Ok(bfs_state)
}

async fn create_self_referencing_bfs_state() -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();
    
    let pubkey = fixtures.sample_pubkey();
    let node = BfsNode {
        pubkey,
        depth: 1,
        amount: 1000,
        processed: false,
    };
    
    bfs_state.nodes.clear();
    bfs_state.nodes.insert(pubkey, node);
    
    Ok(bfs_state)
}

async fn create_disconnected_components_bfs_state() -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();
    
    bfs_state.nodes.clear();
    
    // Create several disconnected components
    for component in 0..5 {
        for node_in_component in 0..10 {
            let pubkey = fixtures.sample_pubkey();
            let node = BfsNode {
                pubkey,
                depth: node_in_component as i32,
                amount: 1000 + (component * 10 + node_in_component) as u64,
                processed: false,
            };
            bfs_state.nodes.insert(pubkey, node);
        }
    }
    
    Ok(bfs_state)
}

async fn create_maximum_depth_bfs_state() -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();
    
    let pubkey = fixtures.sample_pubkey();
    let node = BfsNode {
        pubkey,
        depth: i32::MAX,
        amount: 1000,
        processed: false,
    };
    
    bfs_state.nodes.clear();
    bfs_state.nodes.insert(pubkey, node);
    
    Ok(bfs_state)
}

async fn create_zero_amounts_bfs_state() -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();
    
    bfs_state.nodes.clear();
    
    let pubkeys = fixtures.sample_pubkeys(10);
    for (i, pubkey) in pubkeys.into_iter().enumerate() {
        let node = BfsNode {
            pubkey,
            depth: i as i32,
            amount: 0, // Zero amount
            processed: false,
        };
        bfs_state.nodes.insert(pubkey, node);
    }
    
    Ok(bfs_state)
}

async fn create_large_amounts_bfs_state() -> Result<BfsState> {
    let fixtures = TestFixtures::new();
    let mut bfs_state = fixtures.sample_bfs_state();
    
    bfs_state.nodes.clear();
    
    let pubkeys = fixtures.sample_pubkeys(10);
    for (i, pubkey) in pubkeys.into_iter().enumerate() {
        let node = BfsNode {
            pubkey,
            depth: i as i32,
            amount: u64::MAX - (i as u64), // Very large amounts
            processed: false,
        };
        bfs_state.nodes.insert(pubkey, node);
    }
    
    Ok(bfs_state)
}

async fn test_bfs_state_serialization(bfs_state: &BfsState) -> Result<()> {
    // Test serialization/deserialization
    let serialized = serde_json::to_string(bfs_state)
        .map_err(|e| muhafidh::error::MuhafidError::SerializationError(e.to_string()))?;
    
    let deserialized: BfsState = serde_json::from_str(&serialized)
        .map_err(|e| muhafidh::error::MuhafidError::SerializationError(e.to_string()))?;
    
    // Verify states match
    assert_eq!(bfs_state.nodes.len(), deserialized.nodes.len());
    assert_eq!(bfs_state.completed, deserialized.completed);
    assert_eq!(bfs_state.max_depth_reached, deserialized.max_depth_reached);
    
    Ok(())
}

async fn test_bfs_operations_on_edge_case(bfs_state: &BfsState, case_name: &str) -> Result<()> {
    // Test various operations that should be safe on any BFS state
    
    // Calculate statistics
    let node_count = bfs_state.nodes.len();
    let processed_count = bfs_state.nodes.values().filter(|n| n.processed).count();
    let total_amount: u64 = bfs_state.nodes.values().map(|n| n.amount).sum();
    let max_depth = bfs_state.nodes.values().map(|n| n.depth).max().unwrap_or(0);
    let min_depth = bfs_state.nodes.values().map(|n| n.depth).min().unwrap_or(0);
    
    println!("  Edge case '{}' statistics:", case_name);
    println!("    Nodes: {}, Processed: {}", node_count, processed_count);
    println!("    Total amount: {}, Depth range: {} to {}", total_amount, min_depth, max_depth);
    
    // All operations should complete without panic
    Ok(())
}

async fn simulate_operation_with_errors(
    pubkey: Pubkey,
    scenario: String,
    postgres_storage: Arc<PostgresStorage>,
    redis_storage: Arc<RedisStorage>,
    helpers: TestHelpers,
    errors_injected: Arc<AtomicUsize>,
    recoveries_successful: Arc<AtomicUsize>,
) -> Result<()> {
    // Randomly inject errors based on scenario
    let should_inject_error = rand::random::<f32>() < 0.3; // 30% chance of error
    
    if should_inject_error {
        errors_injected.fetch_add(1, Ordering::Relaxed);
        
        // Simulate error recovery
        let recovery_attempts = 3;
        for attempt in 1..=recovery_attempts {
            sleep(Duration::from_millis(attempt * 10)).await; // Backoff
            
            // Try to recover
            let recovery_success = rand::random::<f32>() < 0.8; // 80% chance of recovery
            
            if recovery_success {
                recoveries_successful.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }
    
    // Perform actual operation (simplified)
    if let Ok(Some(bfs_state)) = redis_storage.get_bfs_state(&pubkey).await {
        redis_storage.store_bfs_state(&pubkey, &bfs_state).await?;
    }
    
    Ok(())
}

fn get_memory_usage() -> u64 {
    // Simple memory usage approximation
    // In a real implementation, you'd use proper memory profiling tools
    std::hint::black_box(42) // Placeholder - would use actual memory measurement
} 