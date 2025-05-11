pub mod metadata;
pub mod creator;

use crate::model::token::TokenMetadata;
use crate::model::cex::Cex;
use crate::storage::in_memory::creator::CreatorCexConnectionGraph;
use crate::model::creator::CreatorMetadata;
use creator::CreatorHandlerOperator;



pub enum TokenHandler {
    StoreToken {
        token_metadata: TokenMetadata
    },
    UpdateBondedToken {
        token_metadata: TokenMetadata
    },
}

pub enum CreatorHandler {
    CexConnection {
        cex: Cex,
        cex_connection: CreatorCexConnectionGraph
    },
    StoreCreator {
        creator_metadata: CreatorMetadata
    },
}