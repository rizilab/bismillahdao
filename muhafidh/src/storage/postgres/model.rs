use crate::model::token::TokenMetadata;

#[derive(Debug, Clone)]
pub struct TokenMetadataDto {
    pub mint: solana_pubkey::Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creator: solana_pubkey::Pubkey,
    pub created_at: u64,
    pub cex_sources: Option<Vec<solana_pubkey::Pubkey>>,
    pub cex_updated_at: Option<u64>,
    pub associated_bonding_curve: Option<solana_pubkey::Pubkey>,
    pub is_bonded: bool,
    pub bonded_at: Option<u64>,
    pub all_time_high_price: u64,
    pub all_time_high_price_at: u64,
}

impl From<TokenMetadata> for TokenMetadataDto {
    fn from(token: TokenMetadata) -> Self {
        TokenMetadataDto {
            mint: token.mint,
            name: token.name,
            symbol: token.symbol,
            uri: token.uri,
            creator: token.creator,
            created_at: token.created_at,
            cex_sources: token.cex_sources,
            cex_updated_at: token.cex_updated_at,
            associated_bonding_curve: token.associated_bonding_curve,
            is_bonded: token.is_bonded,
            bonded_at: token.bonded_at,
            all_time_high_price: token.all_time_high_price,
            all_time_high_price_at: token.all_time_high_price_at,
        }
    }
}