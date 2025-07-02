use mockall::predicate::*;
use mockall::mock;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use solana_pubkey::Pubkey;
use crate::Result;
use crate::model::creator::metadata::CreatorMetadata;
use crate::model::creator::graph::CreatorConnectionGraph;

// Mock for storage operations
mock! {
    pub StorageEngine {
        // Redis operations
        pub async fn add_failed_account(&self, account: &CreatorMetadata) -> Result<()>;
        pub async fn add_unprocessed_account(&self, account: &CreatorMetadata) -> Result<()>;
        pub async fn get_next_failed_account(&self) -> Result<Option<CreatorMetadata>>;
        pub async fn get_next_unprocessed_account(&self) -> Result<Option<CreatorMetadata>>;
        pub async fn get_pending_account_counts(&self) -> Result<(usize, usize)>;
        
        // PostgreSQL operations
        pub async fn update_token_cex_sources(&self, mint: &Pubkey, sources: &[Pubkey], updated_at: u64) -> Result<()>;
        pub async fn record_cex_activity(&self, cex_name: &str, cex_address: &Pubkey, mint: &Pubkey) -> Result<()>;
        pub async fn store_connection_graph(&self, mint: &Pubkey, graph: &CreatorConnectionGraph) -> Result<()>;
    }
}

// Mock for RPC client
mock! {
    pub RpcClient {
        pub async fn get_signatures_for_address(&self, address: &Pubkey) -> Result<Vec<String>>;
        pub async fn get_transaction(&self, signature: &str) -> Result<Option<String>>;
    }
}

// Mock for message handlers
mock! {
    pub CreatorHandler {
        pub async fn process_sender(
            &self,
            creator_metadata: Arc<CreatorMetadata>,
            sender: Pubkey,
            receiver: Pubkey,
            amount: f64,
            timestamp: i64,
        ) -> Result<()>;
        
        pub async fn handle_max_depth_reached(&self, metadata: Arc<CreatorMetadata>) -> Result<()>;
    }
}

// Mock for pipeline components
mock! {
    pub Pipeline {
        pub async fn run(&mut self) -> Result<()>;
    }
}

/// Helper to create a mock storage engine with common expectations
pub fn create_mock_storage() -> MockStorageEngine {
    let mut mock = MockStorageEngine::new();
    
    // Default successful responses
    mock.expect_add_failed_account()
        .returning(|_| Ok(()));
    
    mock.expect_add_unprocessed_account()
        .returning(|_| Ok(()));
    
    mock.expect_get_pending_account_counts()
        .returning(|| Ok((0, 0)));
    
    mock
}

/// Helper to create a mock RPC client with common expectations
pub fn create_mock_rpc_client() -> MockRpcClient {
    let mut mock = MockRpcClient::new();
    
    // Default empty responses
    mock.expect_get_signatures_for_address()
        .returning(|_| Ok(vec![]));
    
    mock.expect_get_transaction()
        .returning(|_| Ok(None));
    
    mock
}

/// Create a mock pipeline that succeeds by default
pub fn create_mock_pipeline() -> MockPipeline {
    let mut mock = MockPipeline::new();
    
    mock.expect_run()
        .returning(|| Ok(()));
    
    mock
}

/// Create a mock pipeline that fails
pub fn create_failing_mock_pipeline() -> MockPipeline {
    let mut mock = MockPipeline::new();
    
    mock.expect_run()
        .returning(|| Err(crate::error::PipelineError::ProcessingError("Mock pipeline failure".to_string()).into()));
    
    mock
} 