# Muhafidh TDD Testing Strategy

## Overview

This document outlines the comprehensive Test-Driven Development (TDD) approach for the Muhafidh Solana blockchain analyzer project. Our testing strategy covers unit tests, integration tests, property-based tests, performance tests, and edge case testing.

## Testing Pyramid

```
    ┌─────────────────────────┐
    │    E2E Tests (Few)      │  ← Integration tests with real services
    ├─────────────────────────┤
    │  Integration (Some)     │  ← Component integration tests
    ├─────────────────────────┤
    │   Unit Tests (Many)     │  ← Fast, isolated unit tests
    └─────────────────────────┘
```

## Test Categories

### 1. Unit Tests (`tests/unit/`)
- **BFS Logic Tests**: Test breadth-first search state management
- **Race Condition Tests**: Test atomic operations and completion claiming
- **Circular Transfer Tests**: Test handling of circular token transfers
- **Pipeline Component Tests**: Test individual pipeline stages
- **Graph Operations Tests**: Test connection graph operations
- **Serialization Tests**: Test data persistence and recovery

### 2. Integration Tests (`tests/integration/`)
- **Database Integration**: Test with real PostgreSQL (testcontainers)
- **Redis Integration**: Test with real Redis (testcontainers)
- **RPC Mock Integration**: Test with mock Solana RPC
- **End-to-End Pipeline**: Test complete token analysis workflow

### 3. Property-Based Tests (`tests/property/`)
- **BFS Invariants**: Properties that must always hold during BFS
- **State Consistency**: Properties about shared state consistency
- **Concurrency Properties**: Properties under concurrent access

### 4. Performance Tests (`benches/`)
- **BFS Performance**: Benchmark BFS operations at scale
- **Pipeline Throughput**: Measure processing throughput
- **Concurrent Access**: Test performance under high concurrency

### 5. Edge Case Tests (`tests/edge_cases/`)
- **Network Failures**: Test behavior during RPC failures
- **Memory Limits**: Test behavior under memory pressure
- **Extreme Values**: Test with maximum depth, large graphs
- **Error Recovery**: Test recovery from various failure modes

## Test Structure

### Unit Test Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    
    #[tokio::test]
    async fn test_bfs_completion_atomicity() {
        // Arrange
        let metadata = TestFixtures::sample_creator_metadata().await;
        
        // Act & Assert
        TestAssertions::assert_completion_atomicity(&metadata).await;
    }
}
```

### Integration Test Structure
```rust
#[tokio::test]
async fn test_full_pipeline_with_real_services() {
    let test_env = TestEnvironment::new().await.unwrap();
    
    // Test with real database and Redis
    let result = test_env.run_full_pipeline().await;
    
    assert!(result.is_ok());
    test_env.verify_final_state().await.unwrap();
}
```

## Test Data Strategy

### 1. Fixtures
- Deterministic test data for consistent results
- Realistic Solana addresses and transaction patterns
- Various graph topologies (linear, branching, circular)

### 2. Property-Based Testing
- Generate random but valid Solana addresses
- Create random graph structures
- Test with random concurrent access patterns

### 3. Mock Data
- Mock RPC responses for different scenarios
- Simulated network conditions
- Controlled failure scenarios

## Concurrency Testing Strategy

### 1. Race Condition Tests
```rust
#[tokio::test]
async fn test_completion_claiming_race() {
    let metadata = Arc::new(TestFixtures::sample_creator_metadata().await);
    
    // Simulate race between multiple threads
    let results = TestHelpers::simulate_completion_race(metadata, 10).await;
    
    // Only one thread should successfully claim completion
    assert_eq!(results.iter().filter(|&r| *r).count(), 1);
}
```

### 2. Stress Testing
- Test with hundreds of concurrent operations
- Verify no deadlocks or data corruption
- Measure performance degradation

### 3. Interleaving Testing
- Test different orderings of operations
- Verify state consistency across all interleavings

## Edge Cases Coverage

### 1. BFS Edge Cases
- **Empty Queue**: What happens when queue becomes empty
- **Max Depth Reached**: Behavior at depth boundaries
- **Circular References**: A→B→A transfer patterns
- **Self-Transfers**: Address transferring to itself
- **Duplicate Addresses**: Same address appearing multiple times

### 2. System Edge Cases
- **Memory Exhaustion**: Large graphs exceeding memory
- **Network Timeouts**: RPC calls timing out
- **Database Failures**: Connection losses during operations
- **Redis Failures**: Queue operations failing
- **Serialization Failures**: Corrupt data scenarios

### 3. Data Edge Cases
- **Invalid Addresses**: Malformed public keys
- **Zero Amounts**: Transfers with zero value
- **Negative Timestamps**: Invalid time values
- **Extremely Large Numbers**: Overflow scenarios

## Test Execution Strategy

### 1. Local Development
```bash
# Run all tests
cargo test

# Run specific test category
cargo test unit::bfs
cargo test integration::pipeline

# Run with logging
RUST_LOG=debug cargo test test_name -- --nocapture

# Run property tests
cargo test property --release
```

### 2. Continuous Integration
- **Fast Tests**: Unit tests run on every commit
- **Integration Tests**: Run on PR creation/update
- **Property Tests**: Run nightly with extended iterations
- **Performance Tests**: Run weekly with historical comparison

### 3. Test Isolation
- Each test gets fresh database/Redis instances
- Tests run in parallel where possible
- Cleanup after each test to prevent interference

## Property-Based Testing Approach

### BFS Invariants
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn bfs_depth_monotonic(addresses in prop::collection::vec(any::<Pubkey>(), 1..100)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let metadata = create_test_metadata().await;
            
            // Add addresses to BFS
            for addr in &addresses {
                metadata.add_to_queue(*addr, calculate_depth(*addr), vec![]).await;
            }
            
            // Verify depth progression is monotonic
            TestAssertions::assert_depth_progression(&metadata, &addresses).await;
        });
    }
}
```

### Concurrency Properties
```rust
proptest! {
    #[test]
    fn concurrent_operations_maintain_consistency(
        operations in prop::collection::vec(operation_strategy(), 1..50)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let metadata = Arc::new(create_test_metadata().await);
            
            // Execute operations concurrently
            let handles = execute_concurrent_operations(metadata.clone(), operations).await;
            
            // Wait for completion
            for handle in handles {
                handle.await.unwrap();
            }
            
            // Verify final state is consistent
            TestHelpers::verify_bfs_consistency(&metadata).await.unwrap();
        });
    }
}
```

## Performance Testing

### Benchmarks Structure
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_bfs_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("bfs_add_1000_addresses", |b| {
        b.iter(|| {
            rt.block_on(async {
                let metadata = create_test_metadata().await;
                for i in 0..1000 {
                    let addr = generate_test_address(i);
                    metadata.add_to_queue(addr, i % 10, vec![]).await;
                }
            })
        })
    });
}

criterion_group!(benches, bench_bfs_operations);
criterion_main!(benches);
```

## Error Testing Strategy

### 1. Failure Injection
- Randomly fail RPC calls
- Simulate network partitions
- Inject database errors
- Cause memory allocation failures

### 2. Recovery Testing
- Test recovery from partial failures
- Verify data consistency after failures
- Test retry mechanisms
- Validate cleanup after errors

### 3. Chaos Testing
- Random combinations of failures
- Test system behavior under extreme conditions
- Verify graceful degradation

## Test Organization

```
tests/
├── unit/
│   ├── bfs/
│   │   ├── completion_tests.rs
│   │   ├── circular_transfer_tests.rs
│   │   └── race_condition_tests.rs
│   ├── pipeline/
│   │   ├── crawler_tests.rs
│   │   ├── processor_tests.rs
│   │   └── datasource_tests.rs
│   └── model/
│       ├── metadata_tests.rs
│       └── graph_tests.rs
├── integration/
│   ├── database_tests.rs
│   ├── redis_tests.rs
│   ├── pipeline_integration_tests.rs
│   └── end_to_end_tests.rs
├── property/
│   ├── bfs_properties.rs
│   ├── concurrency_properties.rs
│   └── consistency_properties.rs
├── edge_cases/
│   ├── network_failure_tests.rs
│   ├── memory_limit_tests.rs
│   └── extreme_value_tests.rs
└── common/
    ├── test_environment.rs
    └── test_data.rs
```

## Code Coverage Goals

- **Unit Tests**: 90%+ line coverage
- **Integration Tests**: 80%+ feature coverage
- **Edge Cases**: 100% error path coverage
- **Property Tests**: Verify all critical invariants

## Testing Tools

- **Unit Testing**: `tokio-test`, `rstest`
- **Property Testing**: `proptest`
- **Performance**: `criterion`
- **Integration**: `testcontainers`
- **Mocking**: `mockall`
- **Assertions**: `pretty_assertions`
- **Coverage**: `cargo-tarpaulin`

This comprehensive testing strategy ensures that the Muhafidh project maintains high quality, reliability, and performance while enabling confident refactoring and feature development. 