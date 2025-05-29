use std::sync::Arc;

use solana_pubkey::Pubkey;
use tracing::debug;
use tracing::error;

use super::PostgresPool;
use super::model::TokenMetadataDto;
use crate::err_with_loc;
use crate::error::Result;
use crate::error::postgres::PostgresClientError;
use crate::model::token::TokenMetadata;
use crate::storage::postgres::PostgresStorage;

#[derive(Debug, Clone)]
pub struct TokenMetadataDb {
    pub pool: Arc<PostgresPool>,
}

impl TokenMetadataDb {
    /// Sanitize UTF-8 string by removing null bytes and replacing invalid sequences
    /// Similar to Go's sanitizeUTF8 function
    fn sanitize_utf8(s: &str) -> String {
        // First, remove null bytes (0x00)
        let bytes: Vec<u8> = s.bytes().filter(|&b| b != 0).collect();

        // Convert back to string, replacing invalid UTF-8 with replacement character
        String::from_utf8_lossy(&bytes).to_string()
    }

    pub async fn insert_token_metadata(
        &self,
        token: &TokenMetadata,
    ) -> Result<()> {
        let dto = TokenMetadataDto::from(token.clone());
        let conn = self.pool.get().await?;
        conn.execute(
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
            error!("failed_to_insert_token_metadata::{}::{}::{}", dto.mint, dto.uri, e);
            err_with_loc!(PostgresClientError::TransactionError(format!(
                "failed_to_insert_token_metadata::{}::{}::{}",
                dto.mint, dto.uri, e
            )))
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

        conn.execute(
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

        // Sanitize the CEX name to prevent UTF-8 encoding errors
        let sanitized_cex_name = Self::sanitize_utf8(cex_name);

        // First, ensure the CEX exists in the table
        conn.execute(
            "INSERT INTO cex_metrics (
          name, address, total_tokens, last_token_at
      ) VALUES ($1, $2, 1, NOW())
      ON CONFLICT (address) DO UPDATE SET
          total_tokens = cex_metrics.total_tokens + 1,
          last_token_at = NOW()",
            &[&sanitized_cex_name, &cex_address.to_string()],
        )
        .await
        .map_err(|e| {
            error!("failed_to_update_cex_metrics: {}", e);
            err_with_loc!(PostgresClientError::QueryError(format!("failed_to_update_cex_metrics: {}", e)))
        })?;

        // Record the specific token-CEX relationship
        conn.execute(
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

        debug!("recorded_cex_activity::{}::{}", sanitized_cex_name, mint);
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
        conn.execute(
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
        conn.execute(
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
    fn new(pool: Arc<PostgresPool>) -> Self {
        Self {
            pool,
        }
    }

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

    // No need to initialize tables here as this is now handled by migrations
    async fn initialize(&self) -> Result<()> {
        // Just do a health check to ensure the database is available
        self.health_check().await
    }
}
