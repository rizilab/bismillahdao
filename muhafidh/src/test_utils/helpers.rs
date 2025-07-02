use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use solana_pubkey::Pubkey;
use crate::model::creator::metadata::CreatorMetadata;
use crate::test_utils::fixtures::TestFixtures;

/// Test helpers for common operations
pub struct TestHelpers;

impl TestHelpers {
    /// Create a cancellation token that auto-cancels after a timeout
    pub fn timeout_token(duration: Duration) -> CancellationToken {
        let token = CancellationToken::new();
        let token_clone = token.clone();
        
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            token_clone.cancel();
        });
        
        token
    }

    /// Wait for a condition to be true with timeout
    pub async fn wait_for_condition<F, Fut>(
        mut condition: F,
        timeout: Duration,
        check_interval: Duration,
    ) -> bool
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = tokio::time::Instant::now();
        
        while start.elapsed() < timeout {
            if condition().await {
                return true;
            }
            tokio::time::sleep(check_interval).await;
        }
        
        false
    }

    /// Create a test environment with channels and cancellation
    pub fn create_test_environment() -> (
        mpsc::Sender<String>,
        mpsc::Receiver<String>,
        CancellationToken,
    ) {
        let (sender, receiver) = mpsc::channel(100);
        let token = CancellationToken::new();
        (sender, receiver, token)
    }

    /// Simulate concurrent access to shared resources
    pub async fn simulate_concurrent_access<F, Fut>(
        shared_resource: Arc<CreatorMetadata>,
        num_tasks: usize,
        operation: F,
    ) -> Vec<tokio::task::JoinHandle<()>>
    where
        F: Fn(Arc<CreatorMetadata>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let operation = Arc::new(operation);
        let mut handles = Vec::new();
        
        for _ in 0..num_tasks {
            let resource = Arc::clone(&shared_resource);
            let op = Arc::clone(&operation);
            
            let handle = tokio::spawn(async move {
                op(resource).await;
            });
            
            handles.push(handle);
        }
        
        handles
    }

    /// Create a sequence of addresses for testing BFS operations
    pub fn create_address_sequence(count: usize) -> Vec<Pubkey> {
        TestFixtures::sample_pubkeys(count)
    }

    /// Setup logging for tests
    pub fn setup_test_logging() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
    }

    /// Create a test scenario where multiple threads try to claim completion
    pub async fn simulate_completion_race(metadata: Arc<CreatorMetadata>, num_threads: usize) -> Vec<bool> {
        // First ensure BFS is complete
        metadata.bfs_state.queue.write().await.clear();
        metadata.bfs_state.processing_addresses.write().await.clear();
        
        let mut handles = Vec::new();
        
        for _ in 0..num_threads {
            let metadata_clone = Arc::clone(&metadata);
            let handle = tokio::spawn(async move {
                metadata_clone.try_claim_completion().await
            });
            handles.push(handle);
        }
        
        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }
        
        results
    }

    /// Create a test scenario with circular transfers
    pub async fn setup_circular_transfer_scenario(metadata: Arc<CreatorMetadata>) -> (Pubkey, Pubkey) {
        let addr_a = TestFixtures::sample_pubkey();
        let addr_b = TestFixtures::sample_pubkey();
        
        // Simulate A -> B transfer
        metadata.mark_visited(addr_a, 1, vec![addr_a]).await;
        metadata.mark_visited(addr_b, 2, vec![addr_a, addr_b]).await;
        
        (addr_a, addr_b)
    }

    /// Verify BFS state consistency
    pub async fn verify_bfs_consistency(metadata: &CreatorMetadata) -> Result<(), String> {
        let visited = metadata.bfs_state.visited_addresses.read().await;
        let queue = metadata.bfs_state.queue.read().await;
        let processing = metadata.bfs_state.processing_addresses.read().await;
        
        // Check that all queue items are not visited yet or being processed
        for (addr, depth, _) in queue.iter() {
            if let Some((visited_depth, _)) = visited.get(addr) {
                if *depth < *visited_depth {
                    return Err(format!("Queue item {} has depth {} but visited with depth {}", addr, depth, visited_depth));
                }
            }
        }
        
        // Check that processing addresses are not completed
        for addr in processing.iter() {
            if !visited.contains_key(addr) && !queue.iter().any(|(q_addr, _, _)| q_addr == addr) {
                return Err(format!("Processing address {} not found in visited or queue", addr));
            }
        }
        
        Ok(())
    }
} 