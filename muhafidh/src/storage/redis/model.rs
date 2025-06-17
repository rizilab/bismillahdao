use serde::Deserialize;
use serde::Serialize;

use crate::model::token::TokenMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTokenCache {
    pub mint: solana_pubkey::Pubkey,
    pub bonding_curve: Option<solana_pubkey::Pubkey>,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creator: solana_pubkey::Pubkey,
    pub created_at: u64,
}

impl From<TokenMetadata> for NewTokenCache {
    fn from(token: TokenMetadata) -> Self {
        NewTokenCache {
            mint: token.mint,
            bonding_curve: token.bonding_curve,
            name: token.name,
            symbol: token.symbol,
            uri: token.uri,
            creator: token.creator,
            created_at: token.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRelationCache {
    pub mint: solana_pubkey::Pubkey,
    pub bonding_curve: Option<solana_pubkey::Pubkey>,
    pub creator: solana_pubkey::Pubkey,
    pub cex_sources: Vec<solana_pubkey::Pubkey>,
    pub cex_updated_at: u64,
    pub connection_graph: String,
}
