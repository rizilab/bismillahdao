pub mod metadata;
pub mod cex_relation;

use crate::model::token::TokenMetadata;
pub use metadata::TokenHandlerMetadataOperator;


pub enum TokenHandler {
    StoreToken {
        token_metadata: TokenMetadata
    },
    UpdateBondedToken {
        token_metadata: TokenMetadata
    },
    // UpdateCexSources {
    //     mint: solana_pubkey::Pubkey,
    //     cex_sources: Vec<solana_pubkey::Pubkey>,
    //     cex_updated_at: u64,
    // },
}