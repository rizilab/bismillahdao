use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use petgraph::prelude::*;
use petgraph::Graph;
use serde::Deserialize;
use serde::Serialize;
use solana_pubkey::Pubkey;

use super::AddressNode;
use super::TransactionEdge;
use crate::model::cex::Cex;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreatorCexConnectionGraph {
  graph:        Graph<AddressNode, TransactionEdge>,
  #[serde(skip)]
  node_indices: HashMap<Pubkey, NodeIndex>,
}

impl CreatorCexConnectionGraph {
  pub fn new() -> Self {
    Self {
      graph:        Graph::new(),
      node_indices: HashMap::new(),
    }
  }

  pub fn add_node(
    &mut self,
    address: Pubkey,
    is_cex: bool,
  ) -> NodeIndex {
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

    let edge = TransactionEdge { from, to, amount, timestamp };

    self.graph.add_edge(from_idx, to_idx, edge);
  }

  pub fn get_node_count(&self) -> usize { self.graph.node_count() }

  pub fn get_edge_count(&self) -> usize { self.graph.edge_count() }

  // Get all nodes in the graph
  pub fn get_nodes(&self) -> Vec<AddressNode> { self.graph.node_weights().map(|node| node.clone()).collect() }

  // Get all edges in the graph
  pub fn get_edges(&self) -> Vec<TransactionEdge> { self.graph.edge_weights().map(|edge| edge.clone()).collect() }
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

  pub fn add_node(
    &self,
    address: Pubkey,
    is_cex: bool,
  ) -> NodeIndex {
    self.inner.write().unwrap().add_node(address, is_cex)
  }

  pub fn add_edge(
    &self,
    from: Pubkey,
    to: Pubkey,
    amount: f64,
    timestamp: i64,
  ) {
    self.inner.write().unwrap().add_edge(from, to, amount, timestamp);
  }

  pub fn get_node_count(&self) -> usize { self.inner.read().unwrap().get_node_count() }

  pub fn get_edge_count(&self) -> usize { self.inner.read().unwrap().get_edge_count() }

  pub fn clone_graph(&self) -> CreatorCexConnectionGraph { self.inner.read().unwrap().clone() }
}

impl From<CreatorCexConnectionGraph> for SharedCreatorCexConnectionGraph {
  fn from(graph: CreatorCexConnectionGraph) -> Self { Self { inner: Arc::new(RwLock::new(graph)) } }
}
