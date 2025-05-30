use std::sync::Arc;

use carbon_core::pipeline::Pipeline;
use carbon_core::pipeline::ShutdownStrategy;
use carbon_log_metrics::LogMetrics;
use carbon_system_program_decoder::SystemProgramDecoder;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

use crate::Result;
use crate::pipeline::datasource::rpc_creator_analyzer::Filters;
use crate::pipeline::datasource::rpc_creator_analyzer::RpcTransactionAnalyzer;
use crate::pipeline::processor::creator::CreatorInstructionProcessor;
use crate::rpc::config::RpcConfig;

pub async fn make_creator_crawler_pipeline(
    mut processor: CreatorInstructionProcessor,
    child_token: CancellationToken,
    max_depth: usize,
    rpc_config: Arc<RpcConfig>,
) -> Result<Option<Pipeline>> {
    let filters = Filters::new(None, None, None);
    let creator_metadata = processor.get_creator();
    let (analyzed_account, depth, _) = match creator_metadata.pop_from_queue().await {
        Some(item) => item,
        None => {
            warn!("no_items_in_queue::mint::{}", creator_metadata.mint);
            return Ok(None);
        },
    };
    let creator_analyzer_config = processor.get_creator_analyzer_config();

    if depth > max_depth {
        child_token.cancel();
        debug!("max_depth_reached::mint::{}::depth::{}::cancellation_token_cancelled", creator_metadata.mint, depth);
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

    Ok(Some(pipeline))
}
