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
