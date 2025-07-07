use crate::model::token::TokenMetadata;

#[derive(Debug, Clone)]
pub struct TokenMetadataDto {
    pub mint: solana_pubkey::Pubkey,
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

impl TokenMetadataDto {
    /// Sanitize UTF-8 string by removing null bytes and replacing invalid sequences
    /// Similar to Go's sanitizeUTF8 function
    fn sanitize_utf8(s: &str) -> String {
        // First, remove null bytes (0x00)
        let bytes: Vec<u8> = s.bytes().filter(|&b| b != 0).collect();

        // Convert back to string, replacing invalid UTF-8 with replacement character
        String::from_utf8_lossy(&bytes).to_string()
    }
}

impl From<TokenMetadata> for TokenMetadataDto {
    fn from(token: TokenMetadata) -> Self {
        let name = Self::sanitize_utf8(&token.name);
        let symbol = Self::sanitize_utf8(&token.symbol);
        let uri = Self::sanitize_utf8(&token.uri);

        TokenMetadataDto {
            mint: token.mint,
            name,
            symbol,
            uri,
            creator: token.creator,
            platform: token.platform,
            created_at: token.created_at,
            cex_sources: token.cex_sources,
            cex_updated_at: token.cex_updated_at,
            updated_at: token.updated_at,
            associated_bonding_curve: token.associated_bonding_curve,
            is_bonded: token.is_bonded,
            bonded_at: token.bonded_at,
            all_time_high_price: token.all_time_high_price,
            all_time_high_price_at: token.all_time_high_price_at,
        }
    }
}
