use crate::model::token::TokenMetadata;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTokenCache {
    pub mint: solana_pubkey::Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creator: solana_pubkey::Pubkey,
}

impl From<TokenMetadata> for NewTokenCache {
    fn from(token: TokenMetadata) -> Self {
        NewTokenCache { mint: token.mint, name: token.name, symbol: token.symbol, uri: token.uri, creator: token.creator }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRelationCache {
    pub mint: solana_pubkey::Pubkey,
    pub creator: solana_pubkey::Pubkey,
    pub cex_sources: Vec<solana_pubkey::Pubkey>,
    pub cex_updated_at: u64,
    pub connection_graph: String,
}
