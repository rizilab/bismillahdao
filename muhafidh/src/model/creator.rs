use serde::{Deserialize, Serialize};
use crate::storage::in_memory::creator::CreatorCexConnectionGraph;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorMetadata {
    pub address: solana_pubkey::Pubkey,
    pub cex_sources: Vec<solana_pubkey::Pubkey>,
    pub cex_updated_at: u64,
    pub balance: u64,
    pub wallet_connection: CreatorCexConnectionGraph,
}