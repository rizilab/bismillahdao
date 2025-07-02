use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use tokio::runtime::Runtime;

use muhafidh::testing::{TestDatabase, TestRedis};
use muhafidh::test_utils::{TestFixtures, TestHelpers};
use muhafidh::model::creator::metadata::CreatorStatus;
use muhafidh::pipeline::processor::creator::CreatorProcessor;
use muhafidh::storage::postgres::PostgresStorage;
use muhafidh::storage::redis::RedisStorage;
use muhafidh::model::bfs::BfsState;

/// Benchmark creator metadata processing throughput
fn bench_creator_processing_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("creator_processing_throughput");
    group.measurement_time(Duration::from_secs(30));
    
    // Test different batch sizes
    for batch_size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("batch_processing", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    bench_process_creator_batch(batch_size).await
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark concurrent creator processing performance
fn bench_concurrent_creator_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_creator_processing");
    group.measurement_time(Duration::from_secs(30));
    
    // Test different concurrency levels
    for concurrency in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_tasks", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    bench_concurrent_processing(concurrency).await
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark BFS operations performance
fn bench_bfs_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("bfs_operations");
    group.measurement_time(Duration::from_secs(20));
    
    // Test different graph sizes
    for graph_size in [100, 500, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*graph_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("bfs_depth_calculation", graph_size),
            graph_size,
            |b, &graph_size| {
                b.to_async(&rt).iter(|| async {
                    bench_bfs_depth_calculation(graph_size).await
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("circular_detection", graph_size),
            graph_size,
            |b, &graph_size| {
                b.to_async(&rt).iter(|| async {
                    bench_circular_transfer_detection(graph_size).await
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark database operations performance
fn bench_database_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("database_operations");
    group.measurement_time(Duration::from_secs(20));
    
    // Test different operation types
    group.bench_function("postgres_write", |b| {
        b.to_async(&rt).iter(|| async {
            bench_postgres_write_operations().await
        });
    });
    
    group.bench_function("postgres_read", |b| {
        b.to_async(&rt).iter(|| async {
            bench_postgres_read_operations().await
        });
    });
    
    group.bench_function("redis_cache", |b| {
        b.to_async(&rt).iter(|| async {
            bench_redis_cache_operations().await
        });
    });
    
    group.finish();
}

/// Benchmark memory usage and allocation patterns
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_usage");
    group.measurement_time(Duration::from_secs(15));
    
    // Test different data structure sizes
    for size in [1000, 5000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("bfs_state_creation", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    bench_bfs_state_memory_allocation(size).await
                });
            },
        );
    }
    
    group.finish();
}

// Benchmark implementation functions

async fn bench_process_creator_batch(batch_size: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_db = TestDatabase::new().await?;
    let test_redis = TestRedis::new().await?;
    let fixtures = TestFixtures::new();
    
    let postgres_storage = PostgresStorage::new(test_db.get_pool()).await?;
    let redis_storage = RedisStorage::new(test_redis.get_connection()).await?;
    
    let creator_processor = CreatorProcessor::new(
        postgres_storage.clone(),
        redis_storage.clone(),
    );
    
    // Create batch of creators
    let token_pubkeys = fixtures.sample_pubkeys(batch_size);
    
    // Store initial metadata
    for pubkey in &token_pubkeys {
        let metadata = fixtures.creator_metadata_with_status(CreatorStatus::Discovered);
        postgres_storage.store_creator_metadata(pubkey, &metadata).await?;
    }
    
    // Benchmark processing
    let start = std::time::Instant::now();
    
    for pubkey in &token_pubkeys {
        creator_processor.process_creator(pubkey).await?;
    }
    
    let _duration = start.elapsed();
    
    // Cleanup
    test_db.cleanup().await?;
    test_redis.cleanup().await?;
    
    Ok(())
}

async fn bench_concurrent_processing(concurrency: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_db = TestDatabase::new().await?;
    let test_redis = TestRedis::new().await?;
    let fixtures = TestFixtures::new();
    let helpers = TestHelpers::new();
    
    let postgres_storage = PostgresStorage::new(test_db.get_pool()).await?;
    let redis_storage = RedisStorage::new(test_redis.get_connection()).await?;
    
    let creator_processor = CreatorProcessor::new(
        postgres_storage.clone(),
        redis_storage.clone(),
    );
    
    // Create tokens for concurrent processing
    let token_pubkeys = fixtures.sample_pubkeys(concurrency * 5);
    
    // Store initial metadata
    for pubkey in &token_pubkeys {
        let metadata = fixtures.creator_metadata_with_status(CreatorStatus::Discovered);
        postgres_storage.store_creator_metadata(pubkey, &metadata).await?;
    }
    
    // Benchmark concurrent processing
    let processor_clone = creator_processor.clone();
    let concurrent_tasks = helpers.simulate_concurrent_access(
        token_pubkeys,
        move |pubkey| {
            let processor = processor_clone.clone();
            async move {
                processor.process_creator(&pubkey).await
            }
        },
    ).await;
    
    // Wait for completion
    for task in concurrent_tasks {
        task.await??;
    }
    
    // Cleanup
    test_db.cleanup().await?;
    test_redis.cleanup().await?;
    
    Ok(())
}

async fn bench_bfs_depth_calculation(graph_size: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fixtures = TestFixtures::new();
    
    // Create large BFS state
    let mut bfs_state = fixtures.sample_bfs_state();
    
    // Add nodes to reach target graph size
    let pubkeys = fixtures.sample_pubkeys(graph_size);
    for (i, pubkey) in pubkeys.into_iter().enumerate() {
        let node = muhafidh::model::bfs::BfsNode {
            pubkey,
            depth: (i % 10) as i32,
            amount: 1000 + (i as u64),
            processed: false,
        };
        bfs_state.nodes.insert(pubkey, node);
    }
    
    // Benchmark depth calculation
    let _max_depth = bfs_state.nodes.values()
        .map(|node| node.depth)
        .max()
        .unwrap_or(0);
    
    Ok(())
}

async fn bench_circular_transfer_detection(graph_size: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fixtures = TestFixtures::new();
    
    // Create BFS state with potential circular transfers
    let bfs_state = fixtures.bfs_state_with_circular_transfers(graph_size);
    
    // Benchmark circular transfer detection
    let _has_circular = detect_circular_transfers_benchmark(&bfs_state).await;
    
    Ok(())
}

async fn bench_postgres_write_operations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_db = TestDatabase::new().await?;
    let fixtures = TestFixtures::new();
    
    let postgres_storage = PostgresStorage::new(test_db.get_pool()).await?;
    
    // Benchmark write operations
    let token_pubkey = fixtures.sample_pubkey();
    let metadata = fixtures.creator_metadata_with_status(CreatorStatus::Discovered);
    
    postgres_storage.store_creator_metadata(&token_pubkey, &metadata).await?;
    
    // Cleanup
    test_db.cleanup().await?;
    
    Ok(())
}

async fn bench_postgres_read_operations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_db = TestDatabase::new().await?;
    let fixtures = TestFixtures::new();
    
    let postgres_storage = PostgresStorage::new(test_db.get_pool()).await?;
    
    // Setup data
    let token_pubkey = fixtures.sample_pubkey();
    let metadata = fixtures.creator_metadata_with_status(CreatorStatus::Discovered);
    postgres_storage.store_creator_metadata(&token_pubkey, &metadata).await?;
    
    // Benchmark read operations
    let _retrieved = postgres_storage.get_creator_metadata(&token_pubkey).await?;
    
    // Cleanup
    test_db.cleanup().await?;
    
    Ok(())
}

async fn bench_redis_cache_operations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_redis = TestRedis::new().await?;
    let fixtures = TestFixtures::new();
    
    let redis_storage = RedisStorage::new(test_redis.get_connection()).await?;
    
    // Benchmark cache operations
    let cache_key = "test_cache_key";
    let cache_data = fixtures.sample_new_token_cache();
    
    redis_storage.set_cache(cache_key, &cache_data).await?;
    let _retrieved = redis_storage.get_cache(cache_key).await?;
    
    // Cleanup
    test_redis.cleanup().await?;
    
    Ok(())
}

async fn bench_bfs_state_memory_allocation(size: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fixtures = TestFixtures::new();
    
    // Benchmark memory allocation for large BFS state
    let mut bfs_state = fixtures.sample_bfs_state();
    
    let pubkeys = fixtures.sample_pubkeys(size);
    for (i, pubkey) in pubkeys.into_iter().enumerate() {
        let node = muhafidh::model::bfs::BfsNode {
            pubkey,
            depth: (i % 100) as i32,
            amount: 1000 + (i as u64),
            processed: false,
        };
        bfs_state.nodes.insert(pubkey, node);
    }
    
    // Force some operations to ensure memory is actually used
    let _node_count = bfs_state.nodes.len();
    let _total_amount: u64 = bfs_state.nodes.values()
        .map(|node| node.amount)
        .sum();
    
    Ok(())
}

// Helper function for circular detection benchmark
async fn detect_circular_transfers_benchmark(bfs_state: &BfsState) -> bool {
    use std::collections::HashSet;
    
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    
    // Simplified circular detection for benchmarking
    for node in bfs_state.nodes.values() {
        if !visited.contains(&node.pubkey) {
            visited.insert(node.pubkey);
            rec_stack.insert(node.pubkey);
            
            // Simulate some processing
            for other_node in bfs_state.nodes.values() {
                if other_node.pubkey != node.pubkey && rec_stack.contains(&other_node.pubkey) {
                    return true;
                }
            }
            
            rec_stack.remove(&node.pubkey);
        }
    }
    
    false
}

criterion_group!(
    benches,
    bench_creator_processing_throughput,
    bench_concurrent_creator_processing,
    bench_bfs_operations,
    bench_database_operations,
    bench_memory_usage
);

criterion_main!(benches); 