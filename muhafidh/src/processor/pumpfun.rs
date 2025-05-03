use std::sync::Arc;

use carbon_core::deserialize::ArrangeAccounts;
use carbon_core::error::CarbonResult;
use carbon_core::instruction::InstructionProcessorInputType;
use carbon_core::metrics::MetricsCollection;
use carbon_core::processor::Processor;
use carbon_pumpfun_decoder::instructions::create::Create;
use carbon_pumpfun_decoder::instructions::PumpfunInstruction;
use tracing::error;
use tracing::info;

pub struct PfProgramInstructionProcessor;
// {
//     creation_handler: Arc<TokenCreationHandler>,
// }

#[async_trait::async_trait]
impl Processor for PfProgramInstructionProcessor {
  type InputType = InstructionProcessorInputType<PumpfunInstruction>;

  async fn process(
    &mut self,
    data: Self::InputType,
    _metrics: Arc<MetricsCollection>,
  ) -> CarbonResult<()> {
    let (meta, instruction, _nested_instructions) = data;
    match &instruction.data {
      PumpfunInstruction::Create(account_meta) => {
        // process_account_meta(account_meta);
        let accounts = Create::arrange_accounts(&instruction.accounts);
        if let Some(accounts) = accounts {
          info!("Created: {:?}", accounts);
        }
      },
      _ => {},
    }
    Ok(())
  }
}
