use serde::Deserialize;
use serde::Serialize;

use crate::model::token::TokenMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTokenCache {
    pub mint: solana_pubkey::Pubkey,
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
    pub creator: solana_pubkey::Pubkey,
    pub cex_sources: Vec<solana_pubkey::Pubkey>,
    pub cex_updated_at: u64,
    pub connection_graph: String,
}

// Add a model for tracking accounts to analyze
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountToAnalyze {
    pub account: solana_pubkey::Pubkey,
    pub depth: usize,
    pub parent_mint: solana_pubkey::Pubkey,
    pub original_creator: solana_pubkey::Pubkey,
    pub latest_update: u64,
    pub created_at: u64,
    pub retry_count: usize,
    pub status: AccountStatus,
}

// Define a simple enum for account status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AccountStatus {
    Unprocessed, // Regular unprocessed account
    Failed,      // Failed account (no error message stored)
    BfsQueue,    // Account in BFS processing queue
}

impl AccountToAnalyze {
    // Helper to create a failed account
    pub fn mark_as_failed(&mut self) {
        self.retry_count += 1;
        self.status = AccountStatus::Failed;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
    }

    // Helper to create a BFS account
    pub fn mark_as_bfs(&mut self) {
        self.status = AccountStatus::BfsQueue;
        self.latest_update = chrono::Utc::now().timestamp() as u64;
    }
}

impl From<NewTokenCache> for AccountToAnalyze {
    fn from(token: NewTokenCache) -> Self {
        Self {
            account: token.creator,
            depth: 0, // Start at depth 0
            parent_mint: token.mint,
            original_creator: token.creator,
            latest_update: chrono::Utc::now().timestamp() as u64,
            created_at: token.created_at,
            retry_count: 0,
            status: AccountStatus::Unprocessed,
        }
    }
}
