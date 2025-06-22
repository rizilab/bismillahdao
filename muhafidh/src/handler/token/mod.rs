pub mod creator;
pub mod metadata;

use std::sync::Arc;

use solana_pubkey::Pubkey;
use tokio_util::sync::CancellationToken;

use crate::config::CreatorAnalyzerConfig;
use crate::model::cex::Cex;
use crate::model::creator::graph::SharedCreatorConnectionGraph;
use crate::model::creator::metadata::CreatorMetadata;
use crate::model::token::TokenMetadata;

pub enum TokenHandler {
    StoreToken {
        token_metadata: TokenMetadata,
    },
    UpdateBondedToken {
        token_metadata: TokenMetadata,
    },
}

pub enum CreatorHandler {
    ProcessBfsLevel {
        creator_metadata: Arc<CreatorMetadata>,
        sender: Pubkey,
        child_token: CancellationToken,
        creator_analyzer_config: Arc<CreatorAnalyzerConfig>,
    },
    CexConnection {
        cex: Cex,
        cex_connection: SharedCreatorConnectionGraph,
        mint: Pubkey,
        name: String,
        uri: String,
        dev: Pubkey,
        created_at: u64,
        bonding_curve: Pubkey,
    },
    ProcessRecoveredAccount {
        creator_metadata: Arc<CreatorMetadata>,
        child_token: CancellationToken,
        creator_analyzer_config: Arc<CreatorAnalyzerConfig>,
    },
    MaxDepthReached {
        creator_metadata: Arc<CreatorMetadata>,
        child_token: CancellationToken,
    },
}
