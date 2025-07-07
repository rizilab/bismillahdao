use std::sync::Arc;

use carbon_core::deserialize::ArrangeAccounts;
use carbon_core::error::CarbonResult;
use carbon_core::instruction::InstructionProcessorInputType;
use carbon_core::metrics::MetricsCollection;
use carbon_core::processor::Processor;
use carbon_system_program_decoder::instructions::SystemProgramInstruction;
use carbon_system_program_decoder::instructions::transfer_sol::TransferSol;
use carbon_system_program_decoder::instructions::transfer_sol::TransferSolInstructionAccounts;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;

use crate::config::CreatorAnalyzerConfig;
use crate::config::RpcConfig;
use crate::handler::token::creator::CreatorHandlerOperator;
use crate::model::creator::metadata::CreatorMetadata;
use crate::utils::lamports_to_sol;

#[derive(Debug, Clone)]
pub struct CreatorInstructionProcessor {
    creator_metadata: Arc<CreatorMetadata>,
    creator_handler: Arc<CreatorHandlerOperator>,
    cancellation_token: CancellationToken,
    creator_analyzer_config: Arc<CreatorAnalyzerConfig>,
    rpc_config: Arc<RpcConfig>,
    current_depth: Arc<RwLock<usize>>,
}

impl CreatorInstructionProcessor {
    pub fn new(
        creator_handler: Arc<CreatorHandlerOperator>,
        creator_metadata: Arc<CreatorMetadata>,
        cancellation_token: CancellationToken,
        creator_analyzer_config: Arc<CreatorAnalyzerConfig>,
        rpc_config: Arc<RpcConfig>,
        current_depth: Arc<RwLock<usize>>,
    ) -> Self {
        Self {
            creator_metadata,
            creator_handler,
            cancellation_token,
            creator_analyzer_config,
            rpc_config,
            current_depth,
        }
    }

    pub fn get_creator_metadata(&self) -> Arc<CreatorMetadata> {
        self.creator_metadata.clone()
    }

    pub async fn get_current_depth(&self) -> usize {
        self.current_depth.read().await.clone()
    }

    pub async fn set_current_depth(
        &mut self,
        depth: usize,
    ) {
        *self.current_depth.write().await = depth;
    }

    pub fn set_creator_metadata(
        &mut self,
        creator_metadata: Arc<CreatorMetadata>,
    ) {
        self.creator_metadata = creator_metadata;
    }

    pub fn get_creator_analyzer_config(&self) -> Arc<CreatorAnalyzerConfig> {
        self.creator_analyzer_config.clone()
    }

    pub fn get_rpc_config(&self) -> Arc<RpcConfig> {
        self.rpc_config.clone()
    }

    pub async fn handle_pipeline_failure(&self) {
        // error!(
        //     "pipeline_failure::mint::{}::account::{}::marking_as_failed",
        //     self.creator_metadata.mint, self.creator_metadata.address
        // );

        let mut failed_metadata = (*self.creator_metadata).clone();
        failed_metadata.mark_as_failed().await;

        debug!(
            "adding_to_failed_queue::mint::{}::account::{}::retry_count::{}::status::{:?}",
            failed_metadata.mint,
            failed_metadata.get_analyzed_account().await,
            failed_metadata.retry_count,
            failed_metadata.status
        );

        if let Err(e) = self.creator_handler.add_failed_account(&failed_metadata).await {
            error!(
                "failed_to_add_to_failed_queue_after_pipeline_failure::account::{}::error::{}",
                failed_metadata.get_analyzed_account().await,
                e
            );
        }
    }
}

#[async_trait::async_trait]
impl Processor for CreatorInstructionProcessor {
    type InputType = InstructionProcessorInputType<SystemProgramInstruction>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, instruction, _nested_instructions, _solana_instruction) = data;
        match &instruction.data {
            SystemProgramInstruction::TransferSol(transfer_sol) => {
                let accounts = TransferSol::arrange_accounts(&instruction.accounts);
                let amount = lamports_to_sol(transfer_sol.amount);
                let cancellation_token = self.cancellation_token.clone();
                let analyzed_account = self.creator_metadata.get_analyzed_account().await;
                let creator_metadata = self.creator_metadata.clone();
                let creator_analyzer_config = self.creator_analyzer_config.clone();
                let min_transfer_amount = self.creator_analyzer_config.min_transfer_amount;

                if let Some(TransferSolInstructionAccounts {
                    source,
                    destination,
                }) = accounts
                {
                    if amount > min_transfer_amount && source != analyzed_account && destination == analyzed_account {
                        let source_idx = self.creator_metadata.wallet_connection.add_node(source, false).await;
                        let destination_idx =
                            self.creator_metadata.wallet_connection.add_node(destination, false).await;

                        self.creator_metadata
                            .wallet_connection
                            .add_edge(source_idx, destination_idx, amount, chrono::Utc::now().timestamp_millis())
                            .await;
                        let depth = self.get_current_depth().await;
                        creator_metadata.push_to_queue((source, depth + 1, analyzed_account)).await;

                        let timestamp = meta
                            .transaction_metadata
                            .block_time
                            .unwrap_or(chrono::Utc::now().timestamp_millis());

                        if let Err(e) = self
                            .creator_handler
                            .process_sender(
                                creator_metadata,
                                source,
                                destination,
                                amount,
                                timestamp,
                                cancellation_token,
                                creator_analyzer_config,
                                depth,
                            )
                            .await
                        {
                            error!("failed_to_process_sender::error::{}", e);
                        }
                    }
                }
            },
            _ => {},
        }

        Ok(())
    }
}
