use std::collections::HashMap;
use std::sync::Arc;

use petgraph::Graph;
use petgraph::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use solana_pubkey::Pubkey;
use tokio::sync::RwLock;

use crate::model::cex::Cex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressNode {
    pub address: solana_pubkey::Pubkey,
    pub total_received: f64,
    pub total_balance: f64,
    pub is_cex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEdge {
    pub from: solana_pubkey::Pubkey,
    pub to: solana_pubkey::Pubkey,
    pub amount: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreatorConnectionGraph {
    graph: Graph<AddressNode, TransactionEdge>,
    #[serde(skip)]
    node_indices: HashMap<Pubkey, NodeIndex>,
}

impl CreatorConnectionGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            node_indices: HashMap::new(),
        }
    }

    // Rebuild the node_indices HashMap from the graph (useful after deserialization)
    pub fn rebuild_indices(&mut self) {
        self.node_indices.clear();
        for node_index in self.graph.node_indices() {
            if let Some(node) = self.graph.node_weight(node_index) {
                self.node_indices.insert(node.address, node_index);
            }
        }
    }

    // Ensure indices are available (rebuild if empty and graph has nodes)
    fn ensure_indices(&mut self) {
        if self.node_indices.is_empty() && self.graph.node_count() > 0 {
            self.rebuild_indices();
        }
    }

    pub fn add_node(
        &mut self,
        address: Pubkey,
        is_cex: bool,
    ) -> NodeIndex {
        self.ensure_indices();

        if let Some(&idx) = self.node_indices.get(&address) {
            return idx;
        }

        let node = AddressNode {
            address,
            total_received: 0.0,
            total_balance: 0.0,
            is_cex,
        };

        let idx = self.graph.add_node(node);
        self.node_indices.insert(address, idx);

        idx
    }

    pub fn add_edge(
        &mut self,
        from: Pubkey,
        to: Pubkey,
        amount: f64,
        timestamp: i64,
    ) {
        let from_idx = self.add_node(from, Cex::get_exchange_name(from).is_some());
        let to_idx = self.add_node(to, Cex::get_exchange_name(to).is_some());

        let edge = TransactionEdge {
            from,
            to,
            amount,
            timestamp,
        };

        self.graph.add_edge(from_idx, to_idx, edge);
    }

    pub fn get_node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn get_edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    // Get all nodes in the graph
    pub fn get_nodes(&self) -> Vec<AddressNode> {
        self.graph.node_weights().map(|node| node.clone()).collect()
    }

    // Get all edges in the graph
    pub fn get_edges(&self) -> Vec<TransactionEdge> {
        self.graph.edge_weights().map(|edge| edge.clone()).collect()
    }
}

// Thread-safe wrapper for the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedCreatorConnectionGraph {
    #[serde(skip)]
    inner: Arc<RwLock<CreatorConnectionGraph>>,
}

impl SharedCreatorConnectionGraph {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CreatorConnectionGraph::new())),
        }
    }

    pub async fn add_node(
        &self,
        address: Pubkey,
        is_cex: bool,
    ) -> NodeIndex {
        self.inner.write().await.add_node(address, is_cex)
    }

    pub async fn add_edge(
        &self,
        from: Pubkey,
        to: Pubkey,
        amount: f64,
        timestamp: i64,
    ) {
        self.inner.write().await.add_edge(from, to, amount, timestamp);
    }

    pub async fn get_node_count(&self) -> usize {
        self.inner.read().await.get_node_count()
    }

    pub async fn get_edge_count(&self) -> usize {
        self.inner.read().await.get_edge_count()
    }

    pub async fn clone_graph(&self) -> CreatorConnectionGraph {
        let mut graph = self.inner.read().await.clone();
        // Ensure indices are rebuilt after cloning (since they're skipped in serialization)
        graph.rebuild_indices();
        graph
    }

    // Method to ensure indices are available (useful after deserialization)
    pub async fn ensure_indices(&self) {
        self.inner.write().await.ensure_indices();
    }
}

impl From<CreatorConnectionGraph> for SharedCreatorConnectionGraph {
    fn from(graph: CreatorConnectionGraph) -> Self {
        Self {
            inner: Arc::new(RwLock::new(graph)),
        }
    }
}
