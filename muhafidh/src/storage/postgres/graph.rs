// this is where we use pgrouting as graph database
// pub async fn make_graph_client(engine_name: &str) -> Result<Arc<RedisClient>, RedisClientError> {
//     let graph_url = std::env::var("GRAPH_URL").unwrap_or_else(|_| "http://127.0.0.1:8000/".to_string());
//     let client = RedisClient::new(&graph_url).await?;
//     info!("{}::graph_client::connection_established: {}", engine_name, graph_url);
//     Ok(Arc::new(client))
// }

use std::sync::Arc;

use solana_pubkey::Pubkey;
use tracing::debug;
use tracing::error;
use tracing::info;

use crate::err_with_loc;
use crate::error::postgres::PostgresClientError;
use crate::error::Result;
use crate::storage::in_memory::creator::CreatorCexConnectionGraph;
use crate::storage::postgres::PostgresPool;
use crate::storage::postgres::PostgresStorage;

#[derive(Debug, Clone)]
pub struct GraphDb {
  pub pool: Arc<PostgresPool>,
}

#[async_trait::async_trait]
impl PostgresStorage for GraphDb {
  fn new(pool: Arc<PostgresPool>) -> Self { Self { pool } }

  async fn health_check(&self) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::PoolError(e))
    })?;

    conn.execute("SELECT 1", &[]).await.map_err(|e| {
      error!("failed_to_health_check: {}", e);
      err_with_loc!(PostgresClientError::QueryError(format!("failed_to_health_check: {}", e)))
    })?;
    Ok(())
  }

  async fn initialize(&self) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_get_client_pool_connection: {}", e)))
    })?;

    // First check if PostGIS extension is installed
    conn
      .execute(
        "CREATE EXTENSION IF NOT EXISTS postgis;
         CREATE EXTENSION IF NOT EXISTS pgrouting;",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_extensions: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_extensions: {}", e)))
      })?;

    // Create nodes table
    conn
      .execute(
        "CREATE TABLE IF NOT EXISTS wallet_nodes (
           id SERIAL PRIMARY KEY,
           pubkey TEXT UNIQUE NOT NULL,
           is_cex BOOLEAN NOT NULL,
           cex_name TEXT,
           total_received FLOAT DEFAULT 0.0,
           total_balance FLOAT DEFAULT 0.0,
           created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
         )",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_nodes_table: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_nodes_table: {}", e)))
      })?;

    // Create edges table with pgrouting requirements
    conn
      .execute(
        "CREATE TABLE IF NOT EXISTS wallet_edges (
           id SERIAL PRIMARY KEY,
           source_id INTEGER REFERENCES wallet_nodes(id),
           target_id INTEGER REFERENCES wallet_nodes(id),
           source_pubkey TEXT NOT NULL,
           target_pubkey TEXT NOT NULL,
           cost FLOAT DEFAULT 1.0, -- Required by pgrouting
           reverse_cost FLOAT DEFAULT -1.0, -- For directed graph
           amount FLOAT NOT NULL,
           timestamp BIGINT NOT NULL,
           mint TEXT NOT NULL,
           created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
           UNIQUE(source_pubkey, target_pubkey, mint, timestamp)
         )",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_edges_table: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_edges_table: {}", e)))
      })?;

    // Create indexes
    conn
      .execute(
        "CREATE INDEX IF NOT EXISTS idx_wallet_nodes_pubkey ON wallet_nodes(pubkey);
         CREATE INDEX IF NOT EXISTS idx_wallet_edges_source_target ON wallet_edges(source_id, target_id);
         CREATE INDEX IF NOT EXISTS idx_wallet_edges_pubkeys ON wallet_edges(source_pubkey, target_pubkey);
         CREATE INDEX IF NOT EXISTS idx_wallet_edges_mint ON wallet_edges(mint);",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_indexes: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_indexes: {}", e)))
      })?;

    info!("Graph database initialized");
    Ok(())
  }
}

impl GraphDb {
  // Store the connection graph in pgrouting
  pub async fn store_connection_graph(
    &self,
    mint: &Pubkey,
    connection_graph: &CreatorCexConnectionGraph,
  ) -> Result<()> {
    let mut conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::PoolError(e))
    })?;

    // Start a transaction for atomicity
    let tx = conn.transaction().await.map_err(|e| {
      error!("failed_to_start_transaction: {}", e);
      err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_start_transaction: {}", e)))
    })?;

    // First, extract all nodes from the graph and insert them
    let nodes = connection_graph.get_nodes();
    let node_count = nodes.len(); // Store length before iteration

    for node in &nodes {
      // Use reference to avoid moving the Vec
      tx.execute(
        "INSERT INTO wallet_nodes (pubkey, is_cex, cex_name) VALUES ($1, $2, $3)
         ON CONFLICT (pubkey) DO UPDATE SET is_cex = EXCLUDED.is_cex,
           cex_name = EXCLUDED.cex_name",
        &[
          &node.address.to_string(),
          &node.is_cex,
          &(if node.is_cex {
            match crate::model::cex::Cex::get_exchange_name(node.address) {
              Some(name) => Some(name.to_string()),
              None => None,
            }
          } else {
            None
          }),
        ],
      )
      .await
      .map_err(|e| {
        error!("failed_to_insert_node: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_insert_node: {}", e)))
      })?;
    }

    // Then extract all edges and insert them
    let edges = connection_graph.get_edges();
    let edge_count = edges.len(); // Store length before iteration

    for edge in &edges {
      // Use reference to avoid moving the Vec
      // First get the node IDs
      let source_id: i32 = tx
        .query_one("SELECT id FROM wallet_nodes WHERE pubkey = $1", &[&edge.from.to_string()])
        .await
        .map_err(|e| {
          error!("failed_to_get_source_node_id: {}", e);
          err_with_loc!(PostgresClientError::QueryError(format!("failed_to_get_source_node_id: {}", e)))
        })?
        .get(0);

      let target_id: i32 = tx
        .query_one("SELECT id FROM wallet_nodes WHERE pubkey = $1", &[&edge.to.to_string()])
        .await
        .map_err(|e| {
          error!("failed_to_get_target_node_id: {}", e);
          err_with_loc!(PostgresClientError::QueryError(format!("failed_to_get_target_node_id: {}", e)))
        })?
        .get(0);

      // Insert the edge
      tx.execute(
        "INSERT INTO wallet_edges (
           source_id, target_id, source_pubkey, target_pubkey,
           cost, amount, timestamp, mint
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (source_pubkey, target_pubkey, mint, timestamp) DO UPDATE SET
           amount = EXCLUDED.amount",
        &[
          &source_id,
          &target_id,
          &edge.from.to_string(),
          &edge.to.to_string(),
          &(1.0f64),                // Default cost - explicit f64 type annotation
          &(edge.amount as f64),    // Explicit conversion to f64
          &(edge.timestamp as i64), // Explicit conversion to i64
          &mint.to_string(),
        ],
      )
      .await
      .map_err(|e| {
        error!("failed_to_insert_edge: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_insert_edge: {}", e)))
      })?;
    }

    // Commit the transaction
    tx.commit().await.map_err(|e| {
      error!("failed_to_commit_transaction: {}", e);
      err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_commit_transaction: {}", e)))
    })?;

    debug!("Stored connection graph for mint {} with {} nodes and {} edges", mint, node_count, edge_count);
    Ok(())
  }
}
