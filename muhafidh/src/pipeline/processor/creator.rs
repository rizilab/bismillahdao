use std::sync::Arc;

use carbon_core::deserialize::ArrangeAccounts;
use carbon_core::error::CarbonResult;
use carbon_core::instruction::InstructionProcessorInputType;
use carbon_core::metrics::MetricsCollection;
use carbon_core::processor::Processor;
use carbon_system_program_decoder::instructions::SystemProgramInstruction;
use carbon_system_program_decoder::instructions::transfer_sol::TransferSol;
use crate::handler::token::creator::CreatorHandlerOperator;
use tokio_util::sync::CancellationToken;
use crate::model::cex::Cex;
use tracing::info;

use crate::utils::lamports_to_sol;
pub struct CreatorInstructionProcessor {
    token_address: solana_pubkey::Pubkey,
    creator_handler: Arc<CreatorHandlerOperator>,
    cancellation_token: CancellationToken,
}

impl CreatorInstructionProcessor {
    pub fn new(token_address: solana_pubkey::Pubkey, creator_handler: Arc<CreatorHandlerOperator>, cancellation_token: CancellationToken) -> Self {
        Self { token_address, creator_handler, cancellation_token }
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
        if let Some(accounts) = accounts {
            if amount > 0.85 {
                info!("sol_transfer::from({})::to({})::for_amount({})", accounts.source, accounts.destination, amount);        
            }
            
            if let Some(cex) = Cex::get_exchange_name(accounts.source) {
                info!("cex_connection::{}:{}", self.token_address, cex);
                self.cancellation_token.cancel();
            }
            
            // source is not cex address, so what to do?
        }
      },
      _ => {},
    }
    Ok(())
  }
}
