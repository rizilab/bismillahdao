use std::sync::Arc;

use carbon_core::pipeline::Pipeline;
use carbon_core::pipeline::ShutdownStrategy;
use carbon_log_metrics::LogMetrics;
use carbon_system_program_decoder::SystemProgramDecoder;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::warn;

use crate::Result;
use crate::config::RpcConfig;
use crate::handler::token::CreatorHandler;
use crate::pipeline::datasource::rpc_creator_analyzer::Filters;
use crate::pipeline::datasource::rpc_creator_analyzer::RpcTransactionAnalyzer;
use crate::pipeline::processor::creator::CreatorInstructionProcessor;

pub async fn make_creator_crawler_pipeline(
    mut processor: CreatorInstructionProcessor,
    child_token: CancellationToken,
    max_depth: usize,
    rpc_config: Arc<RpcConfig>,
    sender: mpsc::Sender<CreatorHandler>,
) -> Result<Option<(Pipeline, solana_sdk::pubkey::Pubkey)>> {
    let filters = Filters::new(None, None, None);
    let creator_metadata = processor.get_creator();
    let (analyzed_account, depth, _) = match creator_metadata.pop_from_queue().await {
        Some(item) => item,
        None => {
            warn!("no_items_in_queue::mint::{}", creator_metadata.mint);
            
            // Atomically check if BFS is complete and claim the completion
            // This prevents race conditions where multiple threads try to send MaxDepthReached
            if creator_metadata.try_claim_completion().await {
                debug!("bfs_truly_complete_sending_max_depth_reached::mint::{}", creator_metadata.mint);
                if let Err(e) = sender.try_send(CreatorHandler::MaxDepthReached {
                    creator_metadata: creator_metadata.clone(),
                    child_token: child_token.clone(),
                }) {
                    error!("failed_to_send_max_depth_reached_message::mint::{}::error::{}", creator_metadata.mint, e);
                }
            } else {
                debug!("bfs_completion_already_claimed_or_not_complete::mint::{}", creator_metadata.mint);
            }
            
            return Ok(None);
        }
    };

    // Mark this address as currently being processed
    creator_metadata.mark_processing(analyzed_account).await;

    let creator_analyzer_config = processor.get_creator_analyzer_config();

    // This shouldn't happen since we only add to queue if depth < max_depth
    // But keep as a safety check
    if depth >= max_depth {
        error!("unexpected_depth_in_queue::mint::{}::depth::{}::max_depth::{}", 
            creator_metadata.mint, depth, max_depth);
        return Ok(None);
    }

    let rpc_crawler = RpcTransactionAnalyzer::new(
        rpc_config,
        analyzed_account,
        filters,
        Some(CommitmentConfig::confirmed()),
        creator_analyzer_config,
    );

    creator_metadata.add_to_history(analyzed_account).await;
    processor.set_creator(creator_metadata.clone());

    let pipeline = Pipeline::builder()
        .datasource(rpc_crawler)
        .datasource_cancellation_token(child_token.clone())
        .metrics(Arc::new(LogMetrics::new()))
        .shutdown_strategy(ShutdownStrategy::Immediate)
        .instruction(SystemProgramDecoder, processor)
        .build()?;
    debug!("pipeline_built_successfully::mint::{}", creator_metadata.mint);

    Ok(Some((pipeline, analyzed_account)))
}
