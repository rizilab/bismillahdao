use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use serde::Deserialize;
use serde::Serialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use tokio::sync::RwLock;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcProviderRole {
    SignatureFetcher,   // Used for fetching signatures via HTTP
    TransactionFetcher, // Used for fetching transactions via HTTP
    WebSocketProvider,  // Used specifically for WebSocket connections
    Both,               // Can be used for both signature and transaction fetching
    All,                // Can handle any role
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcProviderConfig {
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
    pub rate_limit: usize, // requests per second
    pub role: RpcProviderRole,
}

impl RpcProviderConfig {
    pub fn get_http_url(&self) -> String {
        if let Some(api_key) = &self.api_key {
            // Special handling for Helius format
            if self.name == "helius" {
                format!("https://{}/?api-key={}", self.url, api_key)
            } else {
                format!("https://{}/{}", self.url, api_key)
            }
        } else {
            format!("https://{}", self.url)
        }
    }

    pub fn get_ws_url(&self) -> String {
        if let Some(api_key) = &self.api_key {
            // Special handling for Helius format
            if self.name == "helius" {
                format!("wss://{}/?api-key={}", self.url, api_key)
            } else {
                format!("wss://{}/{}", self.url, api_key)
            }
        } else {
            format!("wss://{}", self.url)
        }
    }
}

// Rate limiter state for each provider
#[derive(Debug)]
pub struct RateLimiterState {
    last_reset: Instant,
    request_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcConfig {
    pub providers: Vec<RpcProviderConfig>,
    pub fallback_timeout_ms: u64,
    #[serde(skip)]
    pub signature_fetcher_index: Arc<AtomicUsize>,
    #[serde(skip)]
    pub transaction_fetcher_index: Arc<AtomicUsize>,
    #[serde(skip)]
    pub rate_limiters: Arc<RwLock<HashMap<String, RateLimiterState>>>,
}

impl Clone for RpcConfig {
    fn clone(&self) -> Self {
        Self {
            providers: self.providers.clone(),
            fallback_timeout_ms: self.fallback_timeout_ms,
            signature_fetcher_index: self.signature_fetcher_index.clone(),
            transaction_fetcher_index: self.transaction_fetcher_index.clone(),
            rate_limiters: self.rate_limiters.clone(),
        }
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            fallback_timeout_ms: 5000,
            signature_fetcher_index: Arc::new(AtomicUsize::new(0)),
            transaction_fetcher_index: Arc::new(AtomicUsize::new(0)),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl RpcConfig {
    // Initialize runtime state after deserialization
    pub async fn init_runtime_state(&mut self) {
        let mut rate_limiters = self.rate_limiters.write().await;
        rate_limiters.clear(); // Clear any existing state

        // Initialize rate limiters for all providers
        for provider in &self.providers {
            let state = RateLimiterState {
                last_reset: Instant::now(),
                request_count: 0,
            };
            rate_limiters.insert(provider.name.clone(), state);
        }
    }

    pub fn get_all_providers_for_role(
        &self,
        role: &RpcProviderRole,
    ) -> Vec<&RpcProviderConfig> {
        self.providers
            .iter()
            .filter(|p| match (&p.role, role) {
                (RpcProviderRole::All, _) => true,
                (RpcProviderRole::Both, RpcProviderRole::SignatureFetcher) => true,
                (RpcProviderRole::Both, RpcProviderRole::TransactionFetcher) => true,
                (RpcProviderRole::SignatureFetcher, RpcProviderRole::SignatureFetcher) => true,
                (RpcProviderRole::TransactionFetcher, RpcProviderRole::TransactionFetcher) => true,
                (RpcProviderRole::WebSocketProvider, RpcProviderRole::WebSocketProvider) => true,
                _ => false,
            })
            .collect()
    }

    pub async fn get_next_client_for_role(
        &self,
        role: &RpcProviderRole,
        commitment: CommitmentConfig,
    ) -> Option<(RpcClient, String)> {
        let providers: Vec<&RpcProviderConfig> = self.get_all_providers_for_role(role);

        if providers.is_empty() {
            warn!("No providers configured for role: {:?}", role);
            return None;
        }

        let current_index_arc = match role {
            RpcProviderRole::SignatureFetcher | RpcProviderRole::Both => &self.signature_fetcher_index,
            RpcProviderRole::TransactionFetcher => &self.transaction_fetcher_index,
            RpcProviderRole::All => &self.signature_fetcher_index,
            RpcProviderRole::WebSocketProvider => {
                warn!(
                    "get_next_client_for_role called with WebSocketProvider role, which is not typical for \
                     round-robin HTTP clients."
                );
                return None;
            },
        };

        let providers_count = providers.len();
        let mut attempts = 0;

        loop {
            let index = current_index_arc.fetch_add(1, Ordering::Relaxed) % providers_count;
            let provider = providers[index];

            let mut can_use_provider = false;

            {
                // Scope for the RwLockWriteGuard
                let mut rate_limiters_guard = self.rate_limiters.write().await;
                let state = rate_limiters_guard
                    .entry(provider.name.clone())
                    .or_insert_with(|| RateLimiterState {
                        last_reset: Instant::now(),
                        request_count: 0,
                    });

                let now = Instant::now();
                if now.duration_since(state.last_reset) >= Duration::from_secs(1) {
                    state.last_reset = now;
                    state.request_count = 0;
                }

                if state.request_count < provider.rate_limit {
                    state.request_count += 1;
                    can_use_provider = true;
                }
            }

            if can_use_provider {
                let client = RpcClient::new_with_commitment(provider.get_http_url(), commitment);
                return Some((client, provider.name.clone()));
            }

            // Rate limited, try next provider or wait
            attempts += 1;
            if attempts >= providers_count {
                #[cfg(feature = "deep-trace")]
                debug!("all_providers_rate_limited_for_role::{:?}::waiting_3_second", role);
                tokio::time::sleep(Duration::from_secs(3)).await;
                attempts = 0;
                continue;
            }
        }
    }

    pub fn get_ws_url(&self) -> String {
        // First check for dedicated WebSocket providers
        let ws_provider = self
            .providers
            .iter()
            .find(|p| matches!(p.role, RpcProviderRole::WebSocketProvider | RpcProviderRole::All));

        // If dedicated WebSocket provider found, use it
        if let Some(provider) = ws_provider {
            return provider.get_ws_url();
        }

        // Fallback to first provider that could serve as WS (e.g. 'All' or 'Both' if applicable, though 'Both' is
        // HTTP-focused) This fallback logic might need refinement based on how 'All' and 'Both' are intended
        // for WebSockets. For now, we only explicitly check WebSocketProvider and All.
        self.providers
            .iter()
            .find(|p| matches!(p.role, RpcProviderRole::All)) // Fallback to 'All' if no dedicated WS
            .map_or_else(
                || {
                    warn!(
                        "No suitable WebSocket provider found, attempting to use the first provider's URL as WS. This \
                         may fail."
                    );
                    self.providers.first().map_or_else(
                        || panic!("No RPC providers configured"),
                        |p| p.get_ws_url(), // This assumes the first provider can be a WS, which might not be true.
                    )
                },
                |p| p.get_ws_url(),
            )
    }
}
