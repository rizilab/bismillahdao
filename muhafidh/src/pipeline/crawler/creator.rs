use std::sync::Arc;

use carbon_core::pipeline::Pipeline;
use carbon_core::pipeline::ShutdownStrategy;
use carbon_log_metrics::LogMetrics;
use carbon_system_program_decoder::SystemProgramDecoder;
use solana_commitment_config::CommitmentConfig;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::warn;

use crate::Result;
use crate::handler::token::CreatorHandler;
use crate::pipeline::datasource::rpc_creator_analyzer::Filters;
use crate::pipeline::datasource::rpc_creator_analyzer::RpcTransactionAnalyzer;
use crate::pipeline::processor::creator::CreatorInstructionProcessor;
use solana_pubkey::Pubkey;

pub async fn make_creator_crawler_pipeline(
    mut processor: CreatorInstructionProcessor,
    child_token: CancellationToken,
    max_depth: usize,
    sender: mpsc::Sender<CreatorHandler>,
) -> Result<Option<(Pipeline, Pubkey)>> {
    let filters = Filters::new(None, None, None);
    let creator_metadata = processor.get_creator_metadata();
    
    let current_depth = processor.get_current_depth().await;

    if let Some((analyzed_account, depth, parent_address)) = creator_metadata.pop_from_queue().await {
        let creator_analyzer_config = processor.get_creator_analyzer_config();
        let rpc_config = processor.get_rpc_config();
        
        let rpc_crawler = RpcTransactionAnalyzer::new(
            rpc_config,
            analyzed_account,
            filters,
            Some(CommitmentConfig::confirmed()),
            creator_analyzer_config,
        );

        creator_metadata.add_to_history(analyzed_account).await;
        creator_metadata.set_analyzed_account(analyzed_account).await;
        processor.set_creator_metadata(creator_metadata.clone());
    
        let pipeline = Pipeline::builder()
            .datasource(rpc_crawler)
            .datasource_cancellation_token(child_token.clone())
            .metrics(Arc::new(LogMetrics::new()))
            .shutdown_strategy(ShutdownStrategy::Immediate)
            .instruction(SystemProgramDecoder, processor)
            .build()?;
        // debug!("pipeline_built_successfully::mint::{}", creator_metadata.mint);
    
        return Ok(Some((pipeline, analyzed_account)));
    }
    
    debug!("no_items_in_queue::mint::{}", creator_metadata.mint);
    child_token.cancel();
    return Ok(None);
}
