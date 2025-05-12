use std::sync::Arc;

use solana_pubkey::Pubkey;
use tracing::debug;
use tracing::error;

use super::model::TokenMetadataDto;
use super::PostgresPool;
use crate::err_with_loc;
use crate::error::postgres::PostgresClientError;
use crate::error::Result;
use crate::model::token::TokenMetadata;
use crate::storage::postgres::PostgresStorage;

#[derive(Debug, Clone)]
pub struct TokenMetadataDb {
  pub pool: Arc<PostgresPool>,
}

impl TokenMetadataDb {
  pub async fn insert_token_metadata(
    &self,
    token: &TokenMetadata,
  ) -> Result<()> {
    let dto = TokenMetadataDto::from(token.clone());
    let conn = self.pool.get().await?;
    conn
      .execute(
        "INSERT INTO tokens (
                mint, name, symbol, uri, creator, created_at, 
                associated_bonding_curve, is_bonded, all_time_high_price, all_time_high_price_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (mint) DO UPDATE SET
                name = EXCLUDED.name,
                symbol = EXCLUDED.symbol,
                uri = EXCLUDED.uri,
                associated_bonding_curve = EXCLUDED.associated_bonding_curve,
                all_time_high_price = 
                    CASE WHEN tokens.all_time_high_price < EXCLUDED.all_time_high_price 
                    THEN EXCLUDED.all_time_high_price 
                    ELSE tokens.all_time_high_price END,
                all_time_high_price_at = 
                    CASE WHEN tokens.all_time_high_price < EXCLUDED.all_time_high_price 
                    THEN EXCLUDED.all_time_high_price_at 
                    ELSE tokens.all_time_high_price_at END",
        &[
          &dto.mint.to_string(),
          &dto.name,
          &dto.symbol,
          &dto.uri,
          &dto.creator.to_string(),
          &(dto.created_at as i64),
          &dto.associated_bonding_curve.map(|p| p.to_string()),
          &dto.is_bonded,
          &(dto.all_time_high_price as i64),
          &(dto.all_time_high_price_at as i64),
        ],
      )
      .await
      .map_err(|e| {
        error!("failed_to_insert_token_metadata::{}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_insert_token_metadata::{}", e)))
      })?;

    debug!("insert_token_metadata::{}", token.mint);
    Ok(())
  }

  pub async fn update_token_cex_sources(
    &self,
    mint: &Pubkey,
    cex_sources: &[Pubkey],
    cex_updated_at: u64,
  ) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::PoolError(e))
    })?;

    // Convert Pubkey array to string array for PostgreSQL
    let cex_sources_str: Vec<String> = cex_sources.iter().map(|pubkey| pubkey.to_string()).collect();

    conn
      .execute(
        "UPDATE tokens 
             SET cex_sources = $1::text[], 
                 cex_updated_at = $2 
             WHERE mint = $3",
        &[&cex_sources_str, &(cex_updated_at as i64), &mint.to_string()],
      )
      .await
      .map_err(|e| {
        error!("failed_to_update_token_cex_sources: {}", e);
        err_with_loc!(PostgresClientError::QueryError(format!("failed_to_update_token_cex_sources: {}", e)))
      })?;

    debug!("update_token_cex_sources::{}: {} sources", mint, cex_sources.len());
    Ok(())
  }
}

#[async_trait::async_trait]
impl PostgresStorage for TokenMetadataDb {
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

    // Create tokens table
    conn
      .execute(
        "CREATE TABLE IF NOT EXISTS tokens (
                mint TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                symbol TEXT NOT NULL,
                uri TEXT NOT NULL,
                creator TEXT NOT NULL,
                created_at BIGINT NOT NULL,
                cex_sources TEXT[] DEFAULT NULL,
                cex_updated_at BIGINT DEFAULT NULL,
                associated_bonding_curve TEXT DEFAULT NULL,
                is_bonded BOOLEAN NOT NULL DEFAULT FALSE,
                bonded_at BIGINT DEFAULT NULL,
                all_time_high_price BIGINT NOT NULL DEFAULT 0,
                all_time_high_price_at BIGINT NOT NULL
            )",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_tokens_table: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_tokens_table: {}", e)))
      })?;

    // Add indexes
    conn
      .execute("CREATE INDEX IF NOT EXISTS idx_tokens_creator ON tokens(creator)", &[])
      .await
      .map_err(|e| {
        error!("failed_to_create_idx_tokens_creator: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_idx_tokens_creator: {}", e)))
      })?;

    conn
      .execute("CREATE INDEX IF NOT EXISTS idx_tokens_mint ON tokens(mint)", &[])
      .await
      .map_err(|e| {
        error!("failed_to_create_idx_tokens_mint: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_idx_tokens_mint: {}", e)))
      })?;

    Ok(())
  }
}
