use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;

use carbon_core::deserialize::ArrangeAccounts;
use carbon_core::error::CarbonResult;
use carbon_core::instruction::InstructionProcessorInputType;
use carbon_core::metrics::MetricsCollection;
use carbon_core::processor::Processor;
use carbon_system_program_decoder::instructions::transfer_sol::TransferSol;
use carbon_system_program_decoder::instructions::SystemProgramInstruction;
use solana_pubkey::Pubkey;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;

use crate::handler::token::creator::CreatorHandlerOperator;
use crate::model::cex::Cex;
use crate::storage::in_memory::creator::SharedCreatorCexConnectionGraph;
use crate::utils::lamports_to_sol;
pub struct CreatorInstructionProcessor {
  mint_address:       Pubkey,
  analyzed_account:   Pubkey,
  creator_handler:    Arc<CreatorHandlerOperator>,
  cancellation_token: CancellationToken,
  // address state
  visited_addresses:  HashMap<Pubkey, (usize, Vec<Pubkey>)>,
  queue:              VecDeque<(Pubkey, usize, Vec<Pubkey>)>,
  max_depth:          usize,
  processed_cex:      HashSet<Pubkey>,
  connection_graph:   SharedCreatorCexConnectionGraph,
}

impl CreatorInstructionProcessor {
  pub fn new(
    mint_address: Pubkey,
    creator_handler: Arc<CreatorHandlerOperator>,
    cancellation_token: CancellationToken,
    max_depth: usize,
  ) -> Self {
    let visited_addresses = HashMap::new();
    let queue = VecDeque::new();

    let connection_graph = SharedCreatorCexConnectionGraph::new();
    let analyzed_account = Pubkey::default();

    Self {
      mint_address,
      analyzed_account,
      creator_handler,
      cancellation_token,
      visited_addresses,
      queue,
      max_depth,
      processed_cex: HashSet::new(),
      connection_graph,
    }
  }

  pub fn set_creator(
    &mut self,
    analyzed_account: Pubkey,
  ) {
    self.analyzed_account = analyzed_account;

    // Initialize BFS with creator
    self.visited_addresses.insert(analyzed_account, (0, vec![analyzed_account]));
    self.queue.push_back((analyzed_account, 0, vec![analyzed_account]));

    // Add creator node to graph
    self.connection_graph.add_node(analyzed_account, false);
  }

  async fn process_sender(
    &mut self,
    sender: Pubkey,
    receiver: Pubkey,
    amount: f64,
    timestamp: i64,
  ) -> Option<(Cex, Vec<Pubkey>)> {
    if self.processed_cex.contains(&sender) {
      return None;
    }

    // Check if sender is a CEX
    if let Some(cex_name) = Cex::get_exchange_name(sender) {
      // MY THOUGHT: since we want to track cex connection to the token, we should
      // save this to the database for later analysis
      let cex = Cex::new(cex_name, sender, 1);

      // Mark as processed
      self.processed_cex.insert(sender);

      // Get the path to the receiver
      let mut path = if let Some((_, receiver_path)) = self.visited_addresses.get(&receiver) {
        receiver_path.clone()
      } else {
        vec![receiver]
      };

      // Add sender to the path
      path.insert(0, sender);

      // Add to graph
      self.connection_graph.add_node(sender, true);
      self.connection_graph.add_edge(sender, receiver, amount, timestamp);

      return Some((cex, path));
    }

    // Regular sender - should we add to BFS queue?
    if let Some((receiver_depth, receiver_path)) = self.visited_addresses.get(&receiver).cloned() {
      // Only add if we're not at max depth
      if receiver_depth < self.max_depth - 1 {
        // Skip if already visited
        if !self.visited_addresses.contains_key(&sender) {
          // Create path for the new node
          let mut sender_path = vec![sender];
          sender_path.extend_from_slice(&receiver_path);

          // Add to BFS state
          let sender_depth = receiver_depth + 1;
          self.visited_addresses.insert(sender, (sender_depth, sender_path.clone()));
          self.queue.push_back((sender, sender_depth, sender_path));

          // Add to graph
          self.connection_graph.add_node(sender, false);
          self.connection_graph.add_edge(sender, receiver, amount, timestamp);
        }
      }
    }

    None
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
          if amount > 0.85 && accounts.source != self.analyzed_account {
            debug!("sol_transfer::from({})::to({})::for_amount({})", accounts.source, accounts.destination, amount);

            let timestamp = meta
              .transaction_metadata
              .block_time
              .unwrap_or(chrono::Utc::now().timestamp_millis());

            // Process this sender
            if let Some((cex, path)) = self
              .process_sender(accounts.source, accounts.destination, amount, timestamp)
              .await
            {
              info!("cex_connection::{}:{} (path length: {})", self.mint_address, cex.name, path.len());

              // Notify handler about CEX connection
              if let Err(e) = self
                .creator_handler
                .record_cex_connection(
                  cex,
                  self.connection_graph.clone_graph(),
                  self.mint_address,
                  self.analyzed_account,
                )
                .await
              {
                error!("Failed to record CEX connection: {}", e);
              }

              // Stop processing
              self.cancellation_token.cancel();
            }
          }
        }
      },
      _ => {},
    }

    // Process next BFS level if needed
    if let Some((address, depth, _)) = self.queue.pop_front() {
      // If we're processing a different address than the initial creator,
      // we need to trigger a new pipeline to process it
      if address != self.analyzed_account {
        // This would trigger a new pipeline for the next BFS level
        if let Err(e) = self
          .creator_handler
          .process_next_bfs_level(address, depth, self.mint_address, self.connection_graph.clone_graph())
          .await
        {
          error!("Failed to process next BFS level: {}", e);
        }
      }
    }

    Ok(())
  }
}
