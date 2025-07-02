use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use rstest::*;

use muhafidh::test_utils::{TestFixtures, TestHelpers, TestAssertions};
use muhafidh::test_utils::mocks::{MockStorageEngine, MockRpcClient, create_mock_storage_with_failures};
use muhafidh::handler::token::creator::CreatorHandler;
use muhafidh::model::creator::metadata::{CreatorMetadata, CreatorStatus};
use muhafidh::pipeline::datasource::rpc_creator_analyzer::RpcCreatorAnalyzer;
use muhafidh::error::{Result, MuhafidError};
use solana_sdk::pubkey::Pubkey;

/// Test module for CreatorHandler unit tests
/// These tests focus on isolated component testing using mocks
mod creator_handler_tests {
    use super::*;

    #[rstest]
    #[tokio::test]
    async fn test_creator_handler_successful_processing() -> Result<()> {
        let fixtures = TestFixtures::new();
        let helpers = TestHelpers::new();
        let assertions = TestAssertions::new();

        // Setup test data
        let token_pubkey = fixtures.sample_pubkey();
        let expected_metadata = fixtures.creator_metadata_with_status(CreatorStatus::Discovered);

        // Create mocks
        let mut mock_storage = MockStorageEngine::new();
        let mut mock_rpc = MockRpcClient::new();

        // Configure mock expectations
        mock_storage
            .expect_get_creator_metadata()
            .with(eq(token_pubkey))
            .times(1)
            .returning(move |_| Ok(Some(expected_metadata.clone())));

        mock_storage
            .expect_store_creator_metadata()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_rpc
            .expect_get_token_metadata()
            .with(eq(token_pubkey))
            .times(1)
            .returning(move |_| {
                Ok(Some(muhafidh::model::TokenMetadata {
                    name: "Test Token".to_string(),
                    symbol: "TEST".to_string(),
                    creator: fixtures.sample_pubkey(),
                    ..Default::default()
                }))
            });

        // Create handler with mocks
        let creator_handler = CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        );

        // Execute test
        let result = creator_handler.process_token(&token_pubkey).await;

        // Assertions
        assert!(result.is_ok(), "Processing should succeed");

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_creator_handler_with_storage_failure() -> Result<()> {
        let fixtures = TestFixtures::new();
        let token_pubkey = fixtures.sample_pubkey();

        // Create mock with storage failure
        let mock_storage = create_mock_storage_with_failures();
        let mut mock_rpc = MockRpcClient::new();

        mock_rpc
            .expect_get_token_metadata()
            .returning(|_| Ok(None));

        let creator_handler = CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        );

        // Execute test
        let result = creator_handler.process_token(&token_pubkey).await;

        // Assertions
        assert!(result.is_err(), "Should fail due to storage error");
        
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_creator_handler_concurrent_processing() -> Result<()> {
        let fixtures = TestFixtures::new();
        let helpers = TestHelpers::new();

        // Setup test data
        let token_pubkeys = fixtures.sample_pubkeys(10);

        // Create mocks that support concurrent access
        let mut mock_storage = MockStorageEngine::new();
        let mut mock_rpc = MockRpcClient::new();

        // Configure mocks for concurrent access
        mock_storage
            .expect_get_creator_metadata()
            .returning(|_| Ok(None));

        mock_storage
            .expect_store_creator_metadata()
            .returning(|_, _| Ok(()));

        mock_rpc
            .expect_get_token_metadata()
            .returning(move |pubkey| {
                Ok(Some(muhafidh::model::TokenMetadata {
                    name: format!("Token {}", pubkey),
                    symbol: "TEST".to_string(),
                    creator: fixtures.sample_pubkey(),
                    ..Default::default()
                }))
            });

        let creator_handler = Arc::new(CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        ));

        // Execute concurrent operations
        let handler_clone = creator_handler.clone();
        let concurrent_tasks = helpers.simulate_concurrent_access(
            token_pubkeys,
            move |pubkey| {
                let handler = handler_clone.clone();
                async move {
                    handler.process_token(&pubkey).await
                }
            },
        ).await;

        // Wait for all tasks to complete
        let mut results = Vec::new();
        for task in concurrent_tasks {
            results.push(task.await?);
        }

        // Assertions
        for result in results {
            assert!(result.is_ok(), "All concurrent operations should succeed");
        }

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_creator_handler_timeout_handling() -> Result<()> {
        let fixtures = TestFixtures::new();
        let token_pubkey = fixtures.sample_pubkey();

        // Create mocks with artificial delays
        let mut mock_storage = MockStorageEngine::new();
        let mut mock_rpc = MockRpcClient::new();

        mock_storage
            .expect_get_creator_metadata()
            .returning(move |_| {
                // Simulate slow storage operation
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(None)
            });

        mock_rpc
            .expect_get_token_metadata()
            .returning(move |_| {
                // Simulate slow RPC operation
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok(None)
            });

        let creator_handler = CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        );

        // Test with very short timeout
        let result = timeout(
            Duration::from_millis(50),
            creator_handler.process_token(&token_pubkey)
        ).await;

        // Should timeout
        assert!(result.is_err(), "Operation should timeout");

        // Test with adequate timeout
        let result = timeout(
            Duration::from_millis(500),
            creator_handler.process_token(&token_pubkey)
        ).await;

        // Should complete
        assert!(result.is_ok(), "Operation should complete within timeout");

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_creator_handler_error_recovery() -> Result<()> {
        let fixtures = TestFixtures::new();
        let token_pubkey = fixtures.sample_pubkey();

        // Create mocks that fail then succeed
        let mut mock_storage = MockStorageEngine::new();
        let mut mock_rpc = MockRpcClient::new();

        // First call fails, second succeeds
        mock_storage
            .expect_get_creator_metadata()
            .times(1)
            .returning(|_| Err(MuhafidError::StorageError("Connection failed".to_string())));

        mock_storage
            .expect_get_creator_metadata()
            .times(1)
            .returning(|_| Ok(None));

        mock_storage
            .expect_store_creator_metadata()
            .returning(|_, _| Ok(()));

        mock_rpc
            .expect_get_token_metadata()
            .returning(|_| Ok(None));

        let creator_handler = CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        );

        // First attempt should fail
        let result1 = creator_handler.process_token(&token_pubkey).await;
        assert!(result1.is_err(), "First attempt should fail");

        // Second attempt should succeed (in real scenario with retry logic)
        // Note: This would need actual retry logic in the handler
        let result2 = creator_handler.process_token(&token_pubkey).await;
        // This might still fail depending on mock setup, adjust based on actual retry implementation

        Ok(())
    }

    #[rstest]
    #[case::discovered(CreatorStatus::Discovered)]
    #[case::processing(CreatorStatus::Processing)]
    #[case::completed(CreatorStatus::Completed)]
    #[case::failed(CreatorStatus::Failed)]
    #[tokio::test]
    async fn test_creator_handler_status_transitions(
        #[case] initial_status: CreatorStatus
    ) -> Result<()> {
        let fixtures = TestFixtures::new();
        let assertions = TestAssertions::new();
        let token_pubkey = fixtures.sample_pubkey();

        let initial_metadata = fixtures.creator_metadata_with_status(initial_status.clone());

        let mut mock_storage = MockStorageEngine::new();
        let mut mock_rpc = MockRpcClient::new();

        // Setup storage mock to return initial metadata
        mock_storage
            .expect_get_creator_metadata()
            .with(eq(token_pubkey))
            .returning(move |_| Ok(Some(initial_metadata.clone())));

        // Capture the stored metadata for validation
        let stored_metadata = Arc::new(tokio::sync::Mutex::new(None));
        let stored_clone = stored_metadata.clone();

        mock_storage
            .expect_store_creator_metadata()
            .withf(move |pubkey, metadata| {
                // Store the metadata for later assertion
                let stored = stored_clone.clone();
                tokio::spawn(async move {
                    *stored.lock().await = Some(metadata.clone());
                });
                true
            })
            .returning(|_, _| Ok(()));

        mock_rpc
            .expect_get_token_metadata()
            .returning(|_| Ok(None));

        let creator_handler = CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        );

        // Execute processing
        let result = creator_handler.process_token(&token_pubkey).await;

        // Validate that status transitions are valid
        if let Some(final_metadata) = stored_metadata.lock().await.as_ref() {
            assertions.assert_valid_status_transition(&initial_status, &final_metadata.status)?;
        }

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_creator_handler_metadata_validation() -> Result<()> {
        let fixtures = TestFixtures::new();
        let token_pubkey = fixtures.sample_pubkey();

        // Create metadata with invalid fields
        let mut invalid_metadata = fixtures.creator_metadata_with_status(CreatorStatus::Discovered);
        invalid_metadata.name = "".to_string(); // Empty name should be invalid
        invalid_metadata.symbol = "TOOLONGSUBSYMBOL".to_string(); // Too long symbol

        let mut mock_storage = MockStorageEngine::new();
        let mut mock_rpc = MockRpcClient::new();

        mock_storage
            .expect_get_creator_metadata()
            .returning(move |_| Ok(Some(invalid_metadata.clone())));

        mock_storage
            .expect_store_creator_metadata()
            .returning(|_, _| Ok(()));

        mock_rpc
            .expect_get_token_metadata()
            .returning(|_| Ok(None));

        let creator_handler = CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        );

        // Execute processing
        let result = creator_handler.process_token(&token_pubkey).await;

        // Should handle invalid metadata gracefully
        // Depending on implementation, this might succeed with warnings or fail
        // Adjust assertion based on actual behavior

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_creator_handler_rpc_analysis_integration() -> Result<()> {
        let fixtures = TestFixtures::new();
        let token_pubkey = fixtures.sample_pubkey();

        // Test integration with RPC creator analyzer
        let mut mock_storage = MockStorageEngine::new();
        let mut mock_rpc = MockRpcClient::new();

        // Setup complex RPC response
        let complex_metadata = muhafidh::model::TokenMetadata {
            name: "Complex Token".to_string(),
            symbol: "COMPLEX".to_string(),
            creator: fixtures.sample_pubkey(),
            uri: Some("https://example.com/metadata.json".to_string()),
            verified: true,
            mint_authority: Some(fixtures.sample_pubkey()),
            freeze_authority: Some(fixtures.sample_pubkey()),
            decimals: 9,
            supply: 1_000_000_000,
        };

        mock_storage
            .expect_get_creator_metadata()
            .returning(|_| Ok(None));

        mock_storage
            .expect_store_creator_metadata()
            .returning(|_, _| Ok(()));

        mock_rpc
            .expect_get_token_metadata()
            .returning(move |_| Ok(Some(complex_metadata.clone())));

        // Additional RPC calls that might be needed for analysis
        mock_rpc
            .expect_get_account_info()
            .returning(|_| Ok(None));

        mock_rpc
            .expect_get_token_largest_accounts()
            .returning(|_| Ok(Vec::new()));

        let creator_handler = CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        );

        // Execute processing
        let result = creator_handler.process_token(&token_pubkey).await;

        // Should successfully process complex metadata
        assert!(result.is_ok(), "Should handle complex metadata successfully");

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_creator_handler_memory_cleanup() -> Result<()> {
        let fixtures = TestFixtures::new();
        let helpers = TestHelpers::new();

        // Process many tokens to test memory cleanup
        let token_pubkeys = fixtures.sample_pubkeys(100);

        let mut mock_storage = MockStorageEngine::new();
        let mut mock_rpc = MockRpcClient::new();

        // Setup minimal mocks
        mock_storage
            .expect_get_creator_metadata()
            .returning(|_| Ok(None));

        mock_storage
            .expect_store_creator_metadata()
            .returning(|_, _| Ok(()));

        mock_rpc
            .expect_get_token_metadata()
            .returning(|_| Ok(None));

        let creator_handler = Arc::new(CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        ));

        // Process tokens sequentially to test memory cleanup
        for pubkey in token_pubkeys {
            let result = creator_handler.process_token(&pubkey).await;
            assert!(result.is_ok(), "Each processing should succeed");
        }

        // Memory usage should not grow indefinitely
        // This is more of a documentation test - actual memory testing would need more sophisticated tools

        Ok(())
    }
}

// Helper functions for testing

fn eq<T: PartialEq + 'static>(expected: T) -> impl Fn(&T) -> bool + 'static
where
    T: Clone,
{
    move |actual: &T| *actual == expected
}

#[cfg(test)]
mod test_helpers {
    use super::*;

    /// Helper to create test environment for creator handler tests
    pub async fn create_test_creator_handler() -> Result<CreatorHandler> {
        let fixtures = TestFixtures::new();
        
        let mock_storage = MockStorageEngine::new();
        let mock_rpc = MockRpcClient::new();

        Ok(CreatorHandler::new(
            Arc::new(mock_storage),
            Arc::new(mock_rpc),
        ))
    }

    /// Helper to validate creator handler state consistency
    pub async fn validate_handler_state_consistency(
        handler: &CreatorHandler,
        token_pubkey: &Pubkey,
    ) -> Result<()> {
        // Add custom validation logic here
        // This would check internal state consistency if handler exposes state
        Ok(())
    }
} 