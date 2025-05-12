use std::sync::Arc;
use petgraph::Graph;
use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;
use std::collections::HashMap;
use petgraph::prelude::*;
use crate::model::cex::Cex;
use std::sync::RwLock;

use super::{AddressNode, TransactionEdge};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreatorCexConnectionGraph {
    graph: Graph<AddressNode, TransactionEdge>,
    #[serde(skip)]
    node_indices: HashMap<Pubkey, NodeIndex>,
}

impl CreatorCexConnectionGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            node_indices: HashMap::new(),
        }
    }
    
    pub fn add_node(&mut self, address: Pubkey, is_cex: bool) -> NodeIndex {
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
    
    pub fn add_edge(&mut self, from: Pubkey, to: Pubkey, amount: f64, timestamp: i64) {
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

    // For serialization/deserialization
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.graph).unwrap_or_default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let graph: Graph<AddressNode, TransactionEdge> = serde_json::from_slice(bytes).ok()?;
        let mut node_indices = HashMap::new();
        
        // Rebuild node indices
        for node_idx in graph.node_indices() {
            if let Some(node) = graph.node_weight(node_idx) {
                node_indices.insert(node.address, node_idx);
            }
        }
        
        Some(Self {
            graph,
            node_indices,
        })
    }
}

// Thread-safe wrapper for the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedCreatorCexConnectionGraph {
    #[serde(skip)]
    inner: Arc<RwLock<CreatorCexConnectionGraph>>,
}

impl SharedCreatorCexConnectionGraph {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CreatorCexConnectionGraph::new())),
        }
    }

    pub fn add_node(&self, address: Pubkey, is_cex: bool) -> NodeIndex {
        self.inner.write().unwrap().add_node(address, is_cex)
    }

    pub fn add_edge(&self, from: Pubkey, to: Pubkey, amount: f64, timestamp: i64) {
        self.inner.write().unwrap().add_edge(from, to, amount, timestamp);
    }

    pub fn get_node_count(&self) -> usize {
        self.inner.read().unwrap().get_node_count()
    }

    pub fn get_edge_count(&self) -> usize {
        self.inner.read().unwrap().get_edge_count()
    }

    pub fn clone_graph(&self) -> CreatorCexConnectionGraph {
        self.inner.read().unwrap().clone()
    }
}

impl From<CreatorCexConnectionGraph> for SharedCreatorCexConnectionGraph {
    fn from(graph: CreatorCexConnectionGraph) -> Self {
        Self {
            inner: Arc::new(RwLock::new(graph)),
        }
    }
}
