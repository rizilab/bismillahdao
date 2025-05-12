use carbon_system_program_decoder::SystemProgramDecoder;
use carbon_core::pipeline::Pipeline;
use carbon_core::pipeline::ShutdownStrategy;
use carbon_log_metrics::LogMetrics;
use crate::pipeline::datasource::rpc_creator_analyzer::RpcTransactionAnalyzer;
use crate::pipeline::datasource::rpc_creator_analyzer::Filters;
use crate::Result;
use std::time::Duration;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use crate::storage::redis::model::NewTokenCache;
use tracing::debug;

use crate::pipeline::processor::creator::CreatorInstructionProcessor;
use crate::handler::token::creator::CreatorHandlerOperator;

pub fn make_creator_crawler_pipeline(rpc_url: String, creator_handler: Arc<CreatorHandlerOperator>, token: NewTokenCache, cancellation_token: CancellationToken) -> Result<Pipeline> {
    debug!("rpc_url: {}", rpc_url);
    
    let filters = Filters::new(
        None,
        None,
        None,
    );

    let rpc_crawler = RpcTransactionAnalyzer::new(
        rpc_url,
        token.creator,
        500,
        Duration::from_secs(1),
        filters,
        None,
        10,
    );
    
    let mut processor = CreatorInstructionProcessor::new(
        token.mint, 
        creator_handler.clone(), 
        cancellation_token.clone()
    );
    
    processor.set_creator(token.creator);

    let pipeline = Pipeline::builder()
        .datasource(rpc_crawler)
        .datasource_cancellation_token(cancellation_token.clone())
        .metrics(Arc::new(LogMetrics::new()))
        .shutdown_strategy(ShutdownStrategy::Immediate)
        .instruction(SystemProgramDecoder, processor)
        .build()?;

    Ok(pipeline)
}
