use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressNode {
  pub address:        solana_pubkey::Pubkey,
  pub total_received: f64,
  pub total_balance:  f64,
  pub is_cex:         bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEdge {
  pub from:      solana_pubkey::Pubkey,
  pub to:        solana_pubkey::Pubkey,
  pub amount:    f64,
  pub timestamp: i64,
}
