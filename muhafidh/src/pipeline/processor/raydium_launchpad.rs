// use std::sync::Arc;

// use carbon_core::deserialize::ArrangeAccounts;
// use carbon_core::error::CarbonResult;
// use carbon_core::instruction::InstructionProcessorInputType;
// use carbon_core::metrics::MetricsCollection;
// use carbon_core::processor::Processor;
// use carbon_raydium_launchpad_decoder::instructions::RaydiumLaunchpadInstruction;
// use carbon_raydium_launchpad_decoder::instructions::initialize::Initialize;
// use crate::model::platform::Platform;
// use tracing::error;

// use crate::handler::token::metadata::TokenHandlerMetadataOperator;

// pub struct RaydiumLaunchpadInstructionProcessor {
//     token_handler: Arc<TokenHandlerMetadataOperator>,
// }

// impl RaydiumLaunchpadInstructionProcessor {
//     pub fn new(token_handler: Arc<TokenHandlerMetadataOperator>) -> Self {
//         Self {
//             token_handler,
//         }
//     }
// }

// #[async_trait::async_trait]
// impl Processor for RaydiumLaunchpadInstructionProcessor {
//     type InputType = InstructionProcessorInputType<RaydiumLaunchpadInstruction>;

//     async fn process(
//         &mut self,
//         data: Self::InputType,
//         _metrics: Arc<MetricsCollection>,
//     ) -> CarbonResult<()> {
//         let (meta, instruction, _nested_instructions, _solana_instruction) = data;
//         match &instruction.data {
//             RaydiumLaunchpadInstruction::Create(account_meta) => {
//                 // process_account_meta(account_meta);
//                 let accounts = Create::arrange_accounts(&instruction.accounts);
//                 if let Some(accounts) = accounts {
//                     // Get block time
//                     let block_time = meta.transaction_metadata.block_time.map(|t| t as u64).unwrap_or_else(|| {
//                         std::time::SystemTime::now()
//                             .duration_since(std::time::UNIX_EPOCH)
//                             .unwrap_or_default()
//                             .as_secs()
//                     });

//                     // Send to handler
//                     if let Err(e) = self.token_handler.store_token(&account_meta, &accounts, Platform::PumpFun,
// block_time).await {                         error!("store_token_failed::{}: {}", accounts.mint, e);
//                     }
//                 }
//             },
//             _ => {},
//         }
//         Ok(())
//     }
// }
