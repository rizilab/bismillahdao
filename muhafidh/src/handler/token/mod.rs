pub mod creator;
pub mod metadata;

use solana_pubkey::Pubkey;

use crate::model::cex::Cex;
use crate::model::creator::CreatorMetadata;
use crate::model::token::TokenMetadata;
use crate::storage::in_memory::creator::CreatorCexConnectionGraph;

pub enum TokenHandler {
  StoreToken {
    token_metadata: TokenMetadata,
  },
  UpdateBondedToken {
    token_metadata: TokenMetadata,
  },
}

pub enum CreatorHandler {
  CexConnection {
    cex:            Cex,
    cex_connection: CreatorCexConnectionGraph,
    mint:           Pubkey,
    creator:        Pubkey,
  },
  StoreCreator {
    creator_metadata: CreatorMetadata,
  },
  ProcessBfsLevel {
    address:          Pubkey,
    depth:            usize,
    mint:             Pubkey,
    connection_graph: CreatorCexConnectionGraph,
  },
}
