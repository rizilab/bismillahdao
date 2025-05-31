use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorAnalyzerConfig {
    pub max_depth: usize,
    pub max_concurrent_requests: usize,
    pub max_signatures_to_check: usize,
    pub min_transfer_amount: f64,
    pub base_retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
    pub max_retries: usize,
}

impl Default for CreatorAnalyzerConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_concurrent_requests: 20,
            max_signatures_to_check: 250,
            min_transfer_amount: 0.1,
            base_retry_delay_ms: 500,
            max_retry_delay_ms: 30_000,
            max_retries: 5,
        }
    }
}
