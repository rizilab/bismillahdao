use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use async_trait::async_trait;
use carbon_core::datasource::Datasource;
use carbon_core::datasource::TransactionUpdate;
use carbon_core::datasource::Update;
use carbon_core::datasource::UpdateType;
use carbon_core::error::CarbonResult;
use carbon_core::metrics::MetricsCollection;
use carbon_core::transformers::transaction_metadata_from_original_meta;
use futures::StreamExt;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_signature::Signature;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use solana_transaction_status::UiLoadedAddresses;
use solana_transaction_status::UiTransactionEncoding;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::warn;

use crate::config::CreatorAnalyzerConfig;
use crate::rpc::config::RpcConfig;
use crate::rpc::config::RpcProviderRole;
use crate::utils::calculate_backoff_with_jitter;
use crate::utils::is_retryable_error;

#[derive(Debug, Clone)]
pub struct Filters {
    pub accounts: Option<Vec<Pubkey>>,
    pub before_signature: Option<Signature>,
    pub until_signature: Option<Signature>,
}

impl Filters {
    pub const fn new(
        accounts: Option<Vec<Pubkey>>,
        before_signature: Option<Signature>,
        until_signature: Option<Signature>,
    ) -> Self {
        Filters {
            accounts,
            before_signature,
            until_signature,
        }
    }
}

pub struct RpcTransactionAnalyzer {
    pub rpc_config: Arc<RpcConfig>,
    pub account: Pubkey,
    pub filters: Filters,
    pub commitment: Option<CommitmentConfig>,
    pub config: Arc<CreatorAnalyzerConfig>,
}

impl RpcTransactionAnalyzer {
    pub fn new(
        rpc_config: Arc<RpcConfig>,
        account: Pubkey,
        filters: Filters,
        commitment: Option<CommitmentConfig>,
        config: Arc<CreatorAnalyzerConfig>,
    ) -> Self {
        Self {
            rpc_config,
            account,
            filters,
            commitment,
            config,
        }
    }
}

#[async_trait]
impl Datasource for RpcTransactionAnalyzer {
    async fn consume(
        &self,
        sender: &Sender<Update>,
        cancellation_token: CancellationToken,
        metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let rpc_config = self.rpc_config.clone();
        let account = self.account;
        let filters = self.filters.clone();
        let sender = sender.clone();
        let commitment = self.commitment;
        let max_concurrent_requests = self.config.max_concurrent_requests;
        let config = self.config.clone();

        let (signature_sender, signature_receiver) = mpsc::channel(1000);
        let (transaction_sender, transaction_receiver) = mpsc::channel(1000);

        let signature_fetcher = signature_fetcher(
            rpc_config.clone(),
            account,
            signature_sender,
            filters.clone(),
            commitment,
            cancellation_token.clone(),
            metrics.clone(),
            config.clone(),
        );

        let transaction_fetcher = transaction_fetcher(
            rpc_config,
            signature_receiver,
            transaction_sender,
            commitment,
            max_concurrent_requests,
            cancellation_token.clone(),
            metrics.clone(),
            config.clone(),
        );

        let task_processor =
            task_processor(transaction_receiver, sender, filters, cancellation_token.clone(), metrics.clone());

        tokio::select! {
        _ = signature_fetcher => {},
        _ = transaction_fetcher => {},
        _ = task_processor => {},
        };

        Ok(())
    }

    fn update_types(&self) -> Vec<UpdateType> {
        vec![UpdateType::Transaction]
    }
}

fn signature_fetcher(
    rpc_config: Arc<RpcConfig>,
    account: Pubkey,
    signature_sender: Sender<Signature>,
    filters: Filters,
    commitment: Option<CommitmentConfig>,
    cancellation_token: CancellationToken,
    metrics: Arc<MetricsCollection>,
    config: Arc<CreatorAnalyzerConfig>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let before_signature = filters.before_signature;
        let until_signature = filters.until_signature;

        let mut retry_count = 0;
        let max_retries = config.max_retries;

        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    debug!("cancellation_detected_in_signature_fetcher");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_millis(0)) => {
                    // Get next client using the new API
                    let commitment_config = commitment.unwrap_or(CommitmentConfig::confirmed());
                    if let Some((client, provider_name)) = rpc_config.get_next_client_for_role(
                        &RpcProviderRole::SignatureFetcher,
                        commitment_config
                    ).await {
                        debug!("fetching_signatures::provider::{}::account::{}", provider_name, account);

                        match client
                            .get_signatures_for_address_with_config(&account, GetConfirmedSignaturesForAddress2Config {
                                before:     before_signature,
                                until:      until_signature,
                                limit:      None,
                                commitment: Some(commitment_config),
                            })
                            .await
                        {
                            Ok(signatures) => {
                                if signatures.is_empty() {
                                    debug!("no_signatures_found_for_account::{}", account);
                                    return;
                                }

                                debug!("fetched_signatures::count::{}::provider::{}", signatures.len(), provider_name);

                                let max_signatures_to_fetch = config.max_signatures_to_check;

                                for sig_info in signatures.iter().take(max_signatures_to_fetch) {
                                    // Check if we're cancelled before processing each signature
                                    if cancellation_token.is_cancelled() {
                                        debug!("cancellation_detected_during_signature_processing");
                                        return;
                                    }

                                    let signature = match Signature::from_str(&sig_info.signature) {
                                        Ok(sig) => sig,
                                        Err(e) => {
                                            error!("invalid_signature_format::{}::error::{:?}", sig_info.signature, e);
                                            continue;
                                        },
                                    };

                                    if let Err(e) = signature_sender.try_send(signature) {
                                        debug!("signature_channel_closed::likely_cancelled::error::{:?}", e);
                                        return;
                                    }
                                }

                                let start = Instant::now();
                                let time_taken = start.elapsed().as_millis();

                                metrics.record_histogram("transaction_crawler_signatures_fetch_times_milliseconds", time_taken as f64)
                                        .await.unwrap_or_else(|value| error!("Error recording metric: {}", value));

                                metrics.increment_counter("transaction_crawler_signatures_fetched", signatures.len() as u64).await.unwrap_or_else(|value| error!("Error recording metric: {}", value));

                                return;
                            }
                            Err(e) => {
                                error!("error_fetching_signatures::provider::{}::account::{}::error::{}", provider_name, account, e);

                                retry_count += 1;
                                if retry_count >= max_retries {
                                    error!("max_retries_reached_for_signatures::account::{}", account);
                                    return;
                                }

                                // Calculate backoff with jitter
                                let backoff_delay = calculate_backoff_with_jitter(
                                    retry_count - 1,
                                    config.base_retry_delay_ms,
                                    config.max_retry_delay_ms,
                                );

                                debug!(
                                    "retrying_signature_fetch_after_backoff::attempt::{}::delay_ms::{}::account::{}",
                                    retry_count,
                                    backoff_delay.as_millis(),
                                    account
                                );

                                tokio::time::sleep(backoff_delay).await;
                            }
                        }
                    } else {
                        error!("no_signature_fetcher_providers_available::account::{}", account);
                        
                        retry_count += 1;
                        if retry_count >= max_retries {
                            error!("max_retries_reached_no_providers::account::{}", account);
                            // Close the channel to signal failure
                            drop(signature_sender);
                            return;
                        }
                        
                        // Wait and retry with exponential backoff
                        let backoff_delay = calculate_backoff_with_jitter(
                            retry_count - 1,
                            config.base_retry_delay_ms,
                            config.max_retry_delay_ms,
                        );
                        
                        warn!(
                            "no_providers_available::retrying_after_backoff::attempt::{}::delay_ms::{}::account::{}", 
                            retry_count, 
                            backoff_delay.as_millis(),
                            account
                        );
                        
                        tokio::time::sleep(backoff_delay).await;
                    }
                }
            }
        }
    })
}

fn transaction_fetcher(
    rpc_config: Arc<RpcConfig>,
    signature_receiver: Receiver<Signature>,
    transaction_sender: Sender<(Signature, EncodedConfirmedTransactionWithStatusMeta)>,
    commitment: Option<CommitmentConfig>,
    max_concurrent_requests: usize,
    cancellation_token: CancellationToken,
    metrics: Arc<MetricsCollection>,
    config: Arc<CreatorAnalyzerConfig>,
) -> JoinHandle<()> {
    let mut receiver = signature_receiver;

    tokio::spawn(async move {
        let fetch_stream_task = async {
            let fetch_stream = async_stream::stream! {
                while let Some(signature) = receiver.recv().await {
                    yield signature;
                    }
            };

            fetch_stream
                .map(|signature| {
                    let rpc_config = rpc_config.clone();
                    let metrics = metrics.clone();
                    let config = config.clone();
                    let commitment = commitment;

                    async move {
                        let start = Instant::now();
                        debug!("fetching_transaction::signature::{}", signature);

                        let max_retries = config.max_retries;

                        // Try with retries
                        for attempt in 0..max_retries {
                            // Get next client using the new API
                            let commitment_config = commitment.unwrap_or(CommitmentConfig::confirmed());
                            if let Some((client, provider_name)) = rpc_config
                                .get_next_client_for_role(&RpcProviderRole::TransactionFetcher, commitment_config)
                                .await
                            {
                                debug!("fetching_transaction::provider::{}::signature::{}", provider_name, signature);

                                match client
                                    .get_transaction_with_config(&signature, RpcTransactionConfig {
                                        encoding: Some(UiTransactionEncoding::Base64),
                                        commitment: Some(commitment_config),
                                        max_supported_transaction_version: Some(0),
                                    })
                                    .await
                                {
                                    Ok(tx) => {
                                        let time_taken = start.elapsed().as_millis();

                                        if let Err(e) = metrics
                                            .record_histogram("transaction_fetch_time_milliseconds", time_taken as f64)
                                            .await
                                        {
                                            error!("failed_to_record_fetch_time_metric::error::{}", e);
                                        }

                                        return Some((signature, tx));
                                    },
                                    Err(e) => {
                                        let error_string = e.to_string();

                                        // Check if this is a "transaction not found" error that we should skip
                                        if error_string.contains("invalid type: null")
                                            || error_string.contains("Transaction version (0) is not supported")
                                            || error_string.contains("not found")
                                        {
                                            warn!(
                                                "transaction_not_available::signature::{}::provider::{}::skipping",
                                                signature, provider_name
                                            );
                                            return None;
                                        }

                                        error!(
                                            "error_fetching_transaction::provider::{}::signature::{}::error::{}",
                                            provider_name, signature, error_string
                                        );

                                        // Check if it's a retryable error
                                        if is_retryable_error(&error_string) && attempt < max_retries - 1 {
                                            // Calculate backoff with jitter
                                            let backoff_delay = calculate_backoff_with_jitter(
                                                attempt,
                                                config.base_retry_delay_ms,
                                                config.max_retry_delay_ms,
                                            );

                                            debug!(
                                                "retrying_after_backoff::attempt::{}::delay_ms::{}::signature::{}",
                                                attempt + 1,
                                                backoff_delay.as_millis(),
                                                signature
                                            );

                                            tokio::time::sleep(backoff_delay).await;
                                        } else if attempt < max_retries - 1 {
                                            // Non-retryable error, still do basic retry with fixed delay
                                            tokio::time::sleep(Duration::from_secs(1)).await;
                                        }
                                    },
                                }
                            } else {
                                error!("no_transaction_fetcher_providers_available::signature::{}", signature);
                                return None;
                            }
                        }

                        error!("all_retries_failed_for_transaction::signature::{}", signature);
                        None
                    }
                })
                .buffer_unordered(max_concurrent_requests)
                .for_each(|result| {
                    async {
                        if let Some((signature, fetched_transaction)) = result {
                            // Record metrics
                            if let Err(e) = metrics.increment_counter("transactions_fetched", 1).await {
                                error!("failed_to_record_transactions_metric::error::{}", e);
                            }

                            // Send transaction
                            if let Err(e) = transaction_sender.send((signature, fetched_transaction)).await {
                                error!("failed_to_send_transaction::error::{:?}", e);
                            }
                        }
                    }
                })
                .await;
        };

        tokio::select! {
            _ = cancellation_token.cancelled() => {
                debug!("cancellation_detected_in_transaction_fetcher");
            }
            _ = fetch_stream_task => {}
        }
    })
}

fn task_processor(
    transaction_receiver: Receiver<(Signature, EncodedConfirmedTransactionWithStatusMeta)>,
    sender: Sender<Update>,
    filters: Filters,
    cancellation_token: CancellationToken,
    metrics: Arc<MetricsCollection>,
) -> JoinHandle<()> {
    let mut transaction_receiver = transaction_receiver;
    let sender = sender.clone();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    break;
                }
                Some((signature, fetched_transaction)) = transaction_receiver.recv() => {
                    let start = Instant::now();
                    let transaction = fetched_transaction.transaction;

                          // Check meta
                    let meta_original = if let Some(meta) = transaction.clone().meta {
                        meta
                    } else {
                        warn!("meta_malformed::transaction::{:?}", signature);
                        continue;
                    };

                          // Skip failed transactions
                    if meta_original.status.is_err() {
                        continue;
                    }

                          // Decode transaction
                    let Some(decoded_transaction) = transaction.transaction.decode() else {
                              error!("failed_to_decode_transaction::signature::{}", signature);
                        continue;
                    };

                          // Filter by accounts if needed
                    if let Some(accounts) = &filters.accounts {
                        let account_set: HashSet<Pubkey> = accounts.iter().cloned().collect();

                        let static_accounts = decoded_transaction.message.static_account_keys();

                        let loaded_addresses = meta_original
                            .loaded_addresses
                            .clone()
                            .unwrap_or_else(|| UiLoadedAddresses {
                                writable: vec![],
                                readonly: vec![],
                            });

                        let all_accounts: HashSet<Pubkey> = static_accounts
                            .iter()
                            .cloned()
                            .chain(
                                loaded_addresses
                                    .writable
                                    .iter()
                                    .filter_map(|s| Pubkey::from_str(s).ok()),
                            )
                            .chain(
                                loaded_addresses
                                    .readonly
                                    .iter()
                                    .filter_map(|s| Pubkey::from_str(s).ok()),
                            )
                            .collect();

                        if !all_accounts
                            .iter()
                            .any(|account| account_set.contains(account))
                        {
                            continue;
                        }
                    }

                          // Get metadata
                    let Ok(meta_needed) = transaction_metadata_from_original_meta(meta_original) else {
                              error!("error_getting_metadata_from_transaction_original_meta::signature::{}", signature);
                        continue;
                    };

                          // Create update
                    let update = Update::Transaction(Box::new(TransactionUpdate {
                        signature,
                        transaction: decoded_transaction.clone(),
                        meta: meta_needed,
                        is_vote: false,
                        slot: fetched_transaction.slot,
                        block_time: fetched_transaction.block_time,
                    }));

                    let elapsed = start.elapsed();
                    if let Err(e) = metrics.record_histogram(
                        "transaction_process_time_milliseconds",
                        elapsed.as_millis() as f64
                    ).await {
                        error!("failed_to_record_process_time_metric::error::{}", e);
                    }

                    if let Err(e) = sender.try_send(update) {
                        error!("failed_to_send_update::signature::{}::error::{:?}", signature, e);
                    continue;
                    }
                }
            }
        }
    })
}
