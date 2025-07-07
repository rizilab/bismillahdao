use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use serde::Deserialize;
use serde::Serialize;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey;
use tokio::sync::RwLock;
use tracing::error;

use crate::Result;
use crate::config::RpcConfig;
use crate::config::RpcProviderRole;
use crate::err_with_loc;
use crate::error::HandlerError;
use crate::utils::lamports_to_sol;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressDetails {
    pub address: Pubkey,
    pub solscan_url: String,
    pub sol_balance: f64,
    pub last_updated: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressNode {
    pub detail: AddressDetails,
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
                self.node_indices.insert(node.detail.address, node_index);
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
        sol_balance: f64,
        is_cex: bool,
    ) -> NodeIndex {
        self.ensure_indices();

        if let Some(&idx) = self.node_indices.get(&address) {
            return idx;
        }

        let solscan_url = format!("https://solscan.io/account/{}", address);
        let last_updated = Utc::now().timestamp_millis();
        let detail = AddressDetails {
            address,
            solscan_url,
            sol_balance,
            last_updated,
        };
        let node = AddressNode {
            detail,
            is_cex,
        };

        let idx = self.graph.add_node(node);
        self.node_indices.insert(address, idx);

        idx
    }

    pub fn add_edge(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        amount: f64,
        timestamp: i64,
    ) {
        let sender = self.graph.node_weight(from).unwrap();
        let receiver = self.graph.node_weight(to).unwrap();
        let edge = TransactionEdge {
            from: sender.detail.address,
            to: receiver.detail.address,
            amount,
            timestamp,
        };

        self.graph.add_edge(from, to, edge);
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

    pub fn get_node_by_address(
        &self,
        address: Pubkey,
    ) -> Option<AddressNode> {
        self.node_indices
            .get(&address)
            .and_then(|idx| self.graph.node_weight(*idx).cloned())
    }

    pub fn get_edge_by_addresses(
        &self,
        from: Pubkey,
        to: Pubkey,
    ) -> Option<TransactionEdge> {
        self.graph
            .find_edge(self.node_indices[&from], self.node_indices[&to])
            .and_then(|edge_idx| self.graph.edge_weight(edge_idx).cloned())
    }

    pub async fn update_node_balance(
        &mut self,
        rpc_config: Arc<RpcConfig>,
    ) -> Result<()> {
        let rpc_config = rpc_config.clone();
        let pubkeys = self
            .graph
            .node_weights()
            .map(|node| node.detail.address)
            .collect::<Vec<Pubkey>>();
        let commitment_config = CommitmentConfig::processed();

        if let Some((client, _)) = rpc_config
            .get_next_client_for_role(&RpcProviderRole::TransactionFetcher, commitment_config)
            .await
        {
            let config = RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::JsonParsed),
                commitment: Some(commitment_config),
                ..RpcAccountInfoConfig::default()
            };

            match client.get_multiple_accounts_with_config(&pubkeys, config).await {
                Ok(result) => {
                    let accounts = result.value;
                    for (i, account) in accounts.iter().enumerate() {
                        if let Some(acc) = account {
                            let balance = lamports_to_sol(acc.lamports);
                            if let Some(idx) =
                                self.graph.node_weights().position(|node| node.detail.address == pubkeys[i])
                            {
                                let node_index = NodeIndex::new(idx);
                                self.graph.node_weight_mut(node_index).unwrap().detail.sol_balance = balance;
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("failed_to_get_multiple_accounts_with_config::error::{}", e);
                    return Err(err_with_loc!(HandlerError::GraphError(format!(
                        "failed_to_get_multiple_accounts_with_config: {}",
                        e
                    ))));
                },
            }
        }
        Ok(())
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
        self.inner.write().await.add_node(address, 0.0, is_cex)
    }

    pub async fn add_edge(
        &self,
        from: NodeIndex,
        to: NodeIndex,
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
