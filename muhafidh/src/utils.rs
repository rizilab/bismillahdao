use std::time::Duration;

use rand::Rng;

pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

/// Convert SOL to lamports
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

/// Calculate exponential backoff with jitter
/// Based on: https://www.helius.dev/docs/rpc/optimization-techniques
pub fn calculate_backoff_with_jitter(
    attempt: usize,
    base_delay_ms: u64,
    max_delay_ms: u64,
) -> Duration {
    // Exponential backoff: delay = base * 2^attempt
    let exponential_delay = base_delay_ms.saturating_mul(3u64.saturating_pow(attempt as u32));

    // Cap at max delay
    let capped_delay = exponential_delay.min(max_delay_ms);

    // Add jitter (±25% of the delay)
    let mut rng = rand::rng();
    let jitter_range = (capped_delay as f64 * 0.25) as u64;
    let jitter = rng.random_range(0..=jitter_range * 2);
    let final_delay = capped_delay.saturating_add(jitter).saturating_sub(jitter_range);

    Duration::from_millis(final_delay)
}

/// Check if an error message indicates a rate limit or timeout that should be retried
pub fn is_retryable_error(error_msg: &str) -> bool {
    error_msg.contains("429") // Rate limit
        || error_msg.contains("timed out")
        || error_msg.contains("operation timed out")
        || error_msg.contains("timeout")
        || error_msg.contains("connection reset")
        || error_msg.contains("connection refused")
        || error_msg.contains("Too Many Requests")
}
