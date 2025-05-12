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
                all_time_high_price = CASE
                    WHEN tokens.all_time_high_price < EXCLUDED.all_time_high_price
                    THEN EXCLUDED.all_time_high_price
                    ELSE tokens.all_time_high_price
                END,
                all_time_high_price_at = CASE
                    WHEN tokens.all_time_high_price < EXCLUDED.all_time_high_price
                    THEN EXCLUDED.all_time_high_price_at
                    ELSE tokens.all_time_high_price_at
                END",
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
         SET cex_sources = $1::text[], cex_updated_at = $2
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

  pub async fn record_cex_activity(
    &self,
    cex_name: &str,
    cex_address: &Pubkey,
    mint: &Pubkey,
  ) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::PoolError(e))
    })?;

    // First, ensure the CEX exists in the table
    conn
      .execute(
        "INSERT INTO cex_metrics (
          name, address, total_tokens, last_token_at
      ) VALUES ($1, $2, 1, NOW())
      ON CONFLICT (address) DO UPDATE SET
          total_tokens = cex_metrics.total_tokens + 1,
          last_token_at = NOW()",
        &[&cex_name, &cex_address.to_string()],
      )
      .await
      .map_err(|e| {
        error!("failed_to_update_cex_metrics: {}", e);
        err_with_loc!(PostgresClientError::QueryError(format!("failed_to_update_cex_metrics: {}", e)))
      })?;

    // Record the specific token-CEX relationship
    conn
      .execute(
        "INSERT INTO cex_token_relations (
          cex_address, token_mint, created_at
      ) VALUES ($1, $2, NOW())
      ON CONFLICT (cex_address, token_mint) DO NOTHING",
        &[&cex_address.to_string(), &mint.to_string()],
      )
      .await
      .map_err(|e| {
        error!("failed_to_record_cex_token_relation: {}", e);
        err_with_loc!(PostgresClientError::QueryError(format!("failed_to_record_cex_token_relation: {}", e)))
      })?;

    debug!("recorded_cex_activity::{}::{}", cex_name, mint);
    Ok(())
  }

  pub async fn update_cex_token_ath(
    &self,
    cex_address: &Pubkey,
    mint: &Pubkey,
    price: u64,
  ) -> Result<()> {
    let conn = self.pool.get().await.map_err(|e| {
      error!("failed_to_get_client_pool_connection: {}", e);
      err_with_loc!(PostgresClientError::PoolError(e))
    })?;

    // Update the CEX metrics to track ATH tokens from this CEX
    conn
      .execute(
        "UPDATE cex_metrics SET ath_tokens = cex_metrics.ath_tokens + 1
         WHERE address = $1 AND NOT EXISTS (
            SELECT 1 FROM cex_token_ath WHERE cex_address = $1 AND token_mint = $2
         )",
        &[&cex_address.to_string(), &mint.to_string()],
      )
      .await
      .map_err(|e| {
        error!("failed_to_update_cex_ath_metrics: {}", e);
        err_with_loc!(PostgresClientError::QueryError(format!("failed_to_update_cex_ath_metrics: {}", e)))
      })?;

    // Record the specific token ATH
    conn
      .execute(
        "INSERT INTO cex_token_ath (
          cex_address, token_mint, ath_price, ath_at
      ) VALUES ($1, $2, $3, NOW())
      ON CONFLICT (cex_address, token_mint) DO UPDATE SET
          ath_price = GREATEST(cex_token_ath.ath_price, EXCLUDED.ath_price),
          ath_at = CASE WHEN cex_token_ath.ath_price < EXCLUDED.ath_price THEN NOW() ELSE cex_token_ath.ath_at END",
        &[&cex_address.to_string(), &mint.to_string(), &(price as i64)],
      )
      .await
      .map_err(|e| {
        error!("failed_to_record_cex_token_ath: {}", e);
        err_with_loc!(PostgresClientError::QueryError(format!("failed_to_record_cex_token_ath: {}", e)))
      })?;

    debug!("updated_cex_token_ath::{}::{}", cex_address, mint);
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

    // Create CEX metrics table
    conn
      .execute(
        "CREATE TABLE IF NOT EXISTS cex_metrics (
          id SERIAL PRIMARY KEY,
          name TEXT NOT NULL,
          address TEXT UNIQUE NOT NULL,
          total_tokens BIGINT NOT NULL DEFAULT 0,
          ath_tokens BIGINT NOT NULL DEFAULT 0,
          first_seen_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
          last_token_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
      )",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_cex_metrics_table: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_cex_metrics_table: {}", e)))
      })?;

    // Create CEX-token relations table
    conn
      .execute(
        "CREATE TABLE IF NOT EXISTS cex_token_relations (
          id SERIAL PRIMARY KEY,
          cex_address TEXT NOT NULL,
          token_mint TEXT NOT NULL,
          created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
          UNIQUE(cex_address, token_mint)
      )",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_cex_token_relations_table: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!(
          "failed_to_create_cex_token_relations_table: {}",
          e
        )))
      })?;

    // Create CEX token ATH table
    conn
      .execute(
        "CREATE TABLE IF NOT EXISTS cex_token_ath (
          id SERIAL PRIMARY KEY,
          cex_address TEXT NOT NULL,
          token_mint TEXT NOT NULL,
          ath_price BIGINT NOT NULL,
          ath_at TIMESTAMP WITH TIME ZONE NOT NULL,
          UNIQUE(cex_address, token_mint)
      )",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_cex_token_ath_table: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_cex_token_ath_table: {}", e)))
      })?;

    // Add indexes
    conn
      .execute(
        "CREATE INDEX IF NOT EXISTS idx_tokens_creator ON tokens(creator);
       CREATE INDEX IF NOT EXISTS idx_tokens_mint ON tokens(mint);
       CREATE INDEX IF NOT EXISTS idx_cex_metrics_address ON cex_metrics(address);
       CREATE INDEX IF NOT EXISTS idx_cex_token_relations_cex ON cex_token_relations(cex_address);
       CREATE INDEX IF NOT EXISTS idx_cex_token_relations_token ON cex_token_relations(token_mint);
       CREATE INDEX IF NOT EXISTS idx_cex_token_ath_cex ON cex_token_ath(cex_address);
       CREATE INDEX IF NOT EXISTS idx_cex_token_ath_token ON cex_token_ath(token_mint);",
        &[],
      )
      .await
      .map_err(|e| {
        error!("failed_to_create_indexes: {}", e);
        err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_indexes: {}", e)))
      })?;

    Ok(())
  }
}
