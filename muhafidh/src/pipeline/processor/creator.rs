use std::sync::Arc;

use carbon_core::deserialize::ArrangeAccounts;
use carbon_core::error::CarbonResult;
use carbon_core::instruction::InstructionProcessorInputType;
use carbon_core::metrics::MetricsCollection;
use carbon_core::processor::Processor;
use carbon_system_program_decoder::instructions::SystemProgramInstruction;
use carbon_system_program_decoder::instructions::transfer_sol::TransferSol;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::debug;

use crate::config::CreatorAnalyzerConfig;
use crate::handler::token::creator::CreatorHandlerOperator;
use crate::model::creator::metadata::CreatorMetadata;
use crate::utils::lamports_to_sol;

#[derive(Debug, Clone)]
pub struct CreatorInstructionProcessor {
    creator_metadata: Arc<CreatorMetadata>,
    creator_handler: Arc<CreatorHandlerOperator>,
    cancellation_token: CancellationToken,
    creator_analyzer_config: Arc<CreatorAnalyzerConfig>,
}

impl CreatorInstructionProcessor {
    pub fn new(
        creator_handler: Arc<CreatorHandlerOperator>,
        creator_metadata: Arc<CreatorMetadata>,
        cancellation_token: CancellationToken,
        creator_analyzer_config: Arc<CreatorAnalyzerConfig>,
    ) -> Self {
        Self {
            creator_metadata,
            creator_handler,
            cancellation_token,
            creator_analyzer_config,
        }
    }

    pub fn get_creator(&self) -> Arc<CreatorMetadata> {
        self.creator_metadata.clone()
    }

    pub fn set_creator(&mut self, creator_metadata: Arc<CreatorMetadata>) {
        self.creator_metadata = creator_metadata;
    }

    pub fn get_creator_analyzer_config(&self) -> Arc<CreatorAnalyzerConfig> {
        self.creator_analyzer_config.clone()
    }

    // Method to handle pipeline failures
    pub async fn handle_pipeline_failure(&self) {
        error!(
            "pipeline_failure::mint::{}::account::{}::marking_as_failed",
            self.creator_metadata.mint, self.creator_metadata.address
        );

        let mut failed_metadata = (*self.creator_metadata).clone();
        failed_metadata.mark_as_failed();
        
        debug!(
            "adding_to_failed_queue::mint::{}::account::{}::retry_count::{}::status::{:?}",
            failed_metadata.mint, failed_metadata.address, failed_metadata.retry_count, failed_metadata.status
        );

        // Add to failed queue for retry
        if let Err(e) = self.creator_handler.add_failed_account(&failed_metadata).await {
            error!(
                "failed_to_add_to_failed_queue_after_pipeline_failure::account::{}::error::{}",
                failed_metadata.address, e
            );
        } else {
            debug!(
                "successfully_added_to_failed_queue::mint::{}::account::{}",
                failed_metadata.mint, failed_metadata.address
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
        let (meta, instruction, _nested_instructions) = data;
        match &instruction.data {
            SystemProgramInstruction::TransferSol(transfer_sol) => {
                let accounts = TransferSol::arrange_accounts(&instruction.accounts);
                let amount = lamports_to_sol(transfer_sol.amount);
                let cancellation_token = self.cancellation_token.clone();
                let analyzed_account = match self.creator_metadata.get_history_front().await {
                    Some(addr) => addr,
                    None => {
                        error!("no_history_available_for_processing");
                        return Ok(());
                    },
                };
                let creator_metadata = self.creator_metadata.clone();
                let creator_analyzer_config = self.creator_analyzer_config.clone();
                let min_transfer_amount = self.creator_analyzer_config.min_transfer_amount;

                if let Some(accounts) = accounts {
                    if amount > min_transfer_amount && accounts.source != analyzed_account && accounts.destination == analyzed_account
                    {
                        let timestamp = meta
                            .transaction_metadata
                            .block_time
                            .unwrap_or(chrono::Utc::now().timestamp_millis());

                        if let Err(e) = self
                            .creator_handler
                            .process_sender(
                                creator_metadata,
                                accounts.source,
                                accounts.destination,
                                amount,
                                timestamp,
                                cancellation_token,
                                creator_analyzer_config,
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
