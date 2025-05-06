use std::sync::Arc;

use anyhow::Result;
use carbon_core::pipeline::Pipeline;
use carbon_core::pipeline::ShutdownStrategy;
use carbon_log_metrics::LogMetrics;
use carbon_pumpfun_decoder::PumpfunDecoder;
use carbon_rpc_block_subscribe_datasource::Filters;
use carbon_rpc_block_subscribe_datasource::RpcBlockSubscribe;
use solana_client::rpc_config::RpcBlockSubscribeConfig;
use solana_client::rpc_config::RpcBlockSubscribeFilter;
use solana_sdk::commitment_config::CommitmentConfig;
use tracing::debug;

use crate::constants::PUMP_FUN_PROGRAM_ID;
use crate::engine::raqib::Raqib;
use crate::processor::pumpfun::PfProgramInstructionProcessor;

pub fn make_pumpfun_subscriber_pipeline(raqib: Raqib) -> Result<Pipeline> {
  let ws_url =
    dotenvy::var("PUMPFUN_MONITOR_RPC_NEW_TOKEN_WSS").expect("PUMPFUN_MONITOR_RPC_NEW_TOKEN_WSS not found in the .env");

  debug!("raqib::pumpfun::subscriber::ws_url: {}", ws_url);

  let filters = Filters::new(
    RpcBlockSubscribeFilter::MentionsAccountOrProgram(PUMP_FUN_PROGRAM_ID.to_string()),
    Some(RpcBlockSubscribeConfig {
      max_supported_transaction_version: Some(0),
      commitment: Some(CommitmentConfig::confirmed()),
      ..RpcBlockSubscribeConfig::default()
    }),
  );
  debug!("raqib::pumpfun::subscriber::filters: {:?}", filters);

  let rpc_program_subscribe = RpcBlockSubscribe::new(ws_url, filters);
  let pipeline = Pipeline::builder()
    .datasource(rpc_program_subscribe)
    .metrics(Arc::new(LogMetrics::new()))
    .metrics_flush_interval(3)
    .instruction(PumpfunDecoder, PfProgramInstructionProcessor::new(raqib.token_handler.clone()))
    .shutdown_strategy(ShutdownStrategy::Immediate)
    .build()?;

  Ok(pipeline)
}
