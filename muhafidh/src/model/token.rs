use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub mint: solana_pubkey::Pubkey,
    pub bonding_curve: Option<solana_pubkey::Pubkey>,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creator: solana_pubkey::Pubkey,
    pub platform: String,
    pub created_at: u64,
    pub cex_sources: Option<Vec<solana_pubkey::Pubkey>>,
    pub cex_updated_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub associated_bonding_curve: Option<solana_pubkey::Pubkey>,
    pub is_bonded: bool,
    pub bonded_at: Option<u64>,
    pub all_time_high_price: u64,
    pub all_time_high_price_at: u64,
}

impl TokenMetadata {
    pub fn new(
        mint: solana_pubkey::Pubkey,
        bonding_curve: Option<solana_pubkey::Pubkey>,
        name: String,
        symbol: String,
        uri: String,
        creator: solana_pubkey::Pubkey,
        platform: String,
        created_at: u64,
        associated_bonding_curve: Option<solana_pubkey::Pubkey>,
        is_bonded: bool,
        all_time_high_price: u64,
        all_time_high_price_at: u64,
    ) -> Self {
        Self {
            mint,
            bonding_curve,
            name,
            symbol,
            uri,
            creator,
            platform,
            created_at,
            cex_sources: None,
            cex_updated_at: None,
            updated_at: None,
            associated_bonding_curve,
            is_bonded,
            bonded_at: None,
            all_time_high_price,
            all_time_high_price_at,
        }
    }
}
