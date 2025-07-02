use std::time::Duration;
use tokio::time::timeout;
use serial_test::serial;

use muhafidh::testing::{TestDatabase, TestRedis};
use muhafidh::test_utils::{TestFixtures, TestHelpers, TestAssertions};
use muhafidh::model::creator::metadata::{CreatorMetadata, CreatorStatus};
use muhafidh::handler::token::creator::CreatorHandler;
use muhafidh::pipeline::processor::creator::CreatorProcessor;
use muhafidh::error::Result;
use muhafidh::storage::postgres::PostgresStorage;
use muhafidh::storage::redis::RedisStorage;

/// Integration test for creator metadata processing pipeline
/// This test demonstrates the complete flow from token discovery to metadata processing
#[tokio::test]
#[serial]
async fn test_creator_metadata_integration_flow() -> Result<()> {
    // Setup test infrastructure
    let test_db = TestDatabase::new().await?;
    let test_redis = TestRedis::new().await?;
    let fixtures = TestFixtures::new();
    let helpers = TestHelpers::new();
    let assertions = TestAssertions::new();

    // Create storage instances with test databases
    let postgres_storage = PostgresStorage::new(test_db.get_pool()).await?;
    let redis_storage = RedisStorage::new(test_redis.get_connection()).await?;

    // Create test creator metadata
    let token_pubkey = fixtures.sample_pubkey();
    let creator_metadata = fixtures.creator_metadata_with_status(CreatorStatus::Discovered);

    // Setup creator handler and processor
    let creator_handler = CreatorHandler::new(
        postgres_storage.clone(),
        redis_storage.clone(),
    );

    let creator_processor = CreatorProcessor::new(
        postgres_storage.clone(),
        redis_storage.clone(),
    );

    // Test 1: Store initial creator metadata
    creator_handler.store_metadata(&token_pubkey, &creator_metadata).await?;

    // Verify metadata was stored correctly
    let stored_metadata = postgres_storage.get_creator_metadata(&token_pubkey).await?;
    assert_eq!(stored_metadata.unwrap().status, CreatorStatus::Discovered);

    // Test 2: Process creator metadata (transition to Processing)
    creator_processor.process_creator(&token_pubkey).await?;

    // Wait for processing to complete
    helpers.wait_for_condition(
        || async {
            let metadata = postgres_storage.get_creator_metadata(&token_pubkey).await.ok();
            metadata.map_or(false, |m| m.map_or(false, |meta| meta.status == CreatorStatus::Processing))
        },
        Duration::from_secs(10),
    ).await?;

    // Test 3: Verify processing state transition
    let processing_metadata = postgres_storage.get_creator_metadata(&token_pubkey).await?;
    assertions.assert_valid_status_transition(
        &CreatorStatus::Discovered,
        &processing_metadata.unwrap().status,
    )?;

    // Test 4: Complete processing (transition to Completed)
    creator_processor.complete_processing(&token_pubkey).await?;

    // Verify final state
    let final_metadata = postgres_storage.get_creator_metadata(&token_pubkey).await?;
    assert_eq!(final_metadata.unwrap().status, CreatorStatus::Completed);

    // Test 5: Verify timestamps are reasonable
    let metadata = postgres_storage.get_creator_metadata(&token_pubkey).await?.unwrap();
    assertions.assert_reasonable_timestamps(&metadata)?;

    // Cleanup
    test_db.cleanup().await?;
    test_redis.cleanup().await?;

    println!("✅ Creator metadata integration test completed successfully");
    Ok(())
}

/// Test concurrent creator metadata processing to detect race conditions
#[tokio::test]
#[serial]
async fn test_concurrent_creator_processing() -> Result<()> {
    let test_db = TestDatabase::new().await?;
    let test_redis = TestRedis::new().await?;
    let fixtures = TestFixtures::new();
    let helpers = TestHelpers::new();

    let postgres_storage = PostgresStorage::new(test_db.get_pool()).await?;
    let redis_storage = RedisStorage::new(test_redis.get_connection()).await?;

    // Create multiple token pubkeys for concurrent processing
    let token_pubkeys = fixtures.sample_pubkeys(10);
    
    // Create creator processor
    let creator_processor = CreatorProcessor::new(
        postgres_storage.clone(),
        redis_storage.clone(),
    );

    // Store initial metadata for all tokens
    for pubkey in &token_pubkeys {
        let metadata = fixtures.creator_metadata_with_status(CreatorStatus::Discovered);
        postgres_storage.store_creator_metadata(pubkey, &metadata).await?;
    }

    // Test concurrent processing
    let processor_clone = creator_processor.clone();
    let concurrent_tasks = helpers.simulate_concurrent_access(
        token_pubkeys.clone(),
        move |pubkey| {
            let processor = processor_clone.clone();
            async move {
                processor.process_creator(&pubkey).await
            }
        },
    ).await;

    // Wait for all tasks to complete with timeout
    timeout(Duration::from_secs(30), async {
        for task in concurrent_tasks {
            task.await??;
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }).await??;

    // Verify all creators were processed correctly
    for pubkey in &token_pubkeys {
        let metadata = postgres_storage.get_creator_metadata(pubkey).await?;
        assert!(metadata.is_some());
        
        let status = &metadata.unwrap().status;
        assert!(
            matches!(status, CreatorStatus::Processing | CreatorStatus::Completed),
            "Creator {:?} has invalid status: {:?}", pubkey, status
        );
    }

    // Cleanup
    test_db.cleanup().await?;
    test_redis.cleanup().await?;

    println!("✅ Concurrent creator processing test completed successfully");
    Ok(())
}

/// Test error handling in creator metadata processing
#[tokio::test]
#[serial]
async fn test_creator_metadata_error_handling() -> Result<()> {
    let test_db = TestDatabase::new().await?;
    let test_redis = TestRedis::new().await?;
    let fixtures = TestFixtures::new();

    let postgres_storage = PostgresStorage::new(test_db.get_pool()).await?;
    let redis_storage = RedisStorage::new(test_redis.get_connection()).await?;

    // Test 1: Processing non-existent creator
    let non_existent_pubkey = fixtures.sample_pubkey();
    let creator_processor = CreatorProcessor::new(
        postgres_storage.clone(),
        redis_storage.clone(),
    );

    let result = creator_processor.process_creator(&non_existent_pubkey).await;
    assert!(result.is_err(), "Processing non-existent creator should fail");

    // Test 2: Invalid status transition
    let token_pubkey = fixtures.sample_pubkey();
    let completed_metadata = fixtures.creator_metadata_with_status(CreatorStatus::Completed);
    
    postgres_storage.store_creator_metadata(&token_pubkey, &completed_metadata).await?;
    
    // Try to process already completed creator
    let result = creator_processor.process_creator(&token_pubkey).await;
    assert!(result.is_err(), "Processing completed creator should fail");

    // Test 3: Database connection failure simulation
    // (This would require more complex infrastructure to simulate actual DB failures)

    // Cleanup
    test_db.cleanup().await?;
    test_redis.cleanup().await?;

    println!("✅ Creator metadata error handling test completed successfully");
    Ok(())
}

/// Test retry logic for failed creator processing
#[tokio::test]
#[serial]
async fn test_creator_processing_retry_logic() -> Result<()> {
    let test_db = TestDatabase::new().await?;
    let test_redis = TestRedis::new().await?;
    let fixtures = TestFixtures::new();
    let assertions = TestAssertions::new();

    let postgres_storage = PostgresStorage::new(test_db.get_pool()).await?;
    let redis_storage = RedisStorage::new(test_redis.get_connection()).await?;

    let token_pubkey = fixtures.sample_pubkey();
    let mut failed_metadata = fixtures.creator_metadata_with_status(CreatorStatus::Failed);
    failed_metadata.retry_count = 2; // Simulate previous failures

    // Store failed metadata
    postgres_storage.store_creator_metadata(&token_pubkey, &failed_metadata).await?;

    // Create processor with retry capability
    let creator_processor = CreatorProcessor::new(
        postgres_storage.clone(),
        redis_storage.clone(),
    );

    // Test retry processing
    creator_processor.retry_failed_creator(&token_pubkey).await?;

    // Verify retry count was incremented
    let updated_metadata = postgres_storage.get_creator_metadata(&token_pubkey).await?.unwrap();
    assertions.assert_retry_count_valid(&updated_metadata)?;

    // Cleanup
    test_db.cleanup().await?;
    test_redis.cleanup().await?;

    println!("✅ Creator processing retry logic test completed successfully");
    Ok(())
} 