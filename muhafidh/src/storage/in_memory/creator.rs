use std::sync::Arc;
use petgraph::Graph;
use serde::{Deserialize, Serialize};

use super::{AddressNode, TransactionEdge};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorCexConnectionGraph {
    graph: Arc<Graph<AddressNode, TransactionEdge>>,
}

impl CreatorCexConnectionGraph {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(Graph::new()),
        }
    }
}
