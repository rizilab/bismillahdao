use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::err_with_loc;
use crate::error::postgres::PostgresClientError;
use crate::storage::postgres::PostgresPool;

/// Current schema version - increment this when adding new migrations
pub const CURRENT_SCHEMA_VERSION: i64 = 18;

/// A migration that can be applied to the database
pub struct Migration {
    /// A unique identifier for this migration
    pub version: i64,
    /// A descriptive name for this migration
    pub name: String,
    /// The SQL to run for this migration - may contain multiple statements separated by semicolons
    pub sql: Vec<&'static str>,
}

/// The Migrator manages database migrations
pub struct Migrator {
    pool: Arc<PostgresPool>,
}

impl Migrator {
    /// Create a new migrator with the given database pool
    pub fn new(pool: Arc<PostgresPool>) -> Self {
        Self {
            pool,
        }
    }

    /// Run all pending migrations
    pub async fn run_migrations(&self) -> Result<()> {
        // Create migrations table if it doesn't exist
        self.create_migrations_table().await?;

        // Get all migrations that have been applied
        let applied = self.get_applied_migrations().await?;

        // Apply any migrations that haven't been applied yet
        for migration in self.get_migrations() {
            if !applied.contains(&migration.version) {
                info!("Applying migration {}_{}", migration.version, migration.name);
                self.apply_migration(&migration).await?;
            }
        }

        Ok(())
    }

    /// Check if the database schema is at the expected version without applying migrations
    pub async fn check_schema_version(&self) -> Result<bool> {
        self.create_migrations_table().await?;
        let applied = self.get_applied_migrations().await?;

        // Get the highest applied migration version
        let current_version = applied.iter().max().copied().unwrap_or(0);

        if current_version < CURRENT_SCHEMA_VERSION {
            warn!(
                "Database schema version mismatch. Expected {}, found {}. Please run migrations.",
                CURRENT_SCHEMA_VERSION, current_version
            );
            return Ok(false);
        }

        info!("Database schema version check passed. Current version: {}", current_version);
        Ok(true)
    }

    /// Create the migrations table if it doesn't exist
    async fn create_migrations_table(&self) -> Result<()> {
        let conn = self.pool.get().await.map_err(|e| {
            error!("failed_to_get_client_pool_connection: {}", e);
            err_with_loc!(PostgresClientError::PoolError(e))
        })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS migrations (
                version BIGINT PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TIMESTAMP WITH TIME ZONE NOT NULL
            )",
            &[],
        )
        .await
        .map_err(|e| {
            error!("failed_to_create_migrations_table: {}", e);
            err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_create_migrations_table: {}", e)))
        })?;

        Ok(())
    }

    /// Get all migrations that have been applied to the database
    async fn get_applied_migrations(&self) -> Result<Vec<i64>> {
        let conn = self.pool.get().await.map_err(|e| {
            error!("failed_to_get_client_pool_connection: {}", e);
            err_with_loc!(PostgresClientError::PoolError(e))
        })?;

        let rows = conn
            .query("SELECT version FROM migrations ORDER BY version ASC", &[])
            .await
            .map_err(|e| {
                error!("failed_to_get_applied_migrations: {}", e);
                err_with_loc!(PostgresClientError::QueryError(format!("failed_to_get_applied_migrations: {}", e)))
            })?;

        let versions = rows.iter().map(|row| row.get::<_, i64>(0)).collect();
        Ok(versions)
    }

    /// Apply a migration to the database
    async fn apply_migration(
        &self,
        migration: &Migration,
    ) -> Result<()> {
        let mut conn = self.pool.get().await.map_err(|e| {
            error!("failed_to_get_client_pool_connection: {}", e);
            err_with_loc!(PostgresClientError::PoolError(e))
        })?;

        // Start a transaction
        let tx = conn.transaction().await.map_err(|e| {
            error!("failed_to_start_transaction: {}", e);
            err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_start_transaction: {}", e)))
        })?;

        // Execute each SQL statement in the migration
        for (i, sql) in migration.sql.iter().enumerate() {
            tx.execute(*sql, &[]).await.map_err(|e| {
                error!("failed_to_execute_migration_statement {}: {}_{}: {}", i, migration.version, migration.name, e);
                err_with_loc!(PostgresClientError::QueryError(format!(
                    "failed_to_execute_migration_statement {}: {}_{}: {}",
                    i, migration.version, migration.name, e
                )))
            })?;
        }

        // Record that we applied this migration
        let now = Utc::now();

        tx.execute("INSERT INTO migrations (version, name, applied_at) VALUES ($1, $2, $3)", &[
            &migration.version,
            &migration.name,
            &now,
        ])
        .await
        .map_err(|e| {
            error!("failed_to_record_migration: {}_{}: {}", migration.version, migration.name, e);
            err_with_loc!(PostgresClientError::QueryError(format!(
                "failed_to_record_migration: {}_{}: {}",
                migration.version, migration.name, e
            )))
        })?;

        // Commit the transaction
        tx.commit().await.map_err(|e| {
            error!("failed_to_commit_transaction: {}", e);
            err_with_loc!(PostgresClientError::TransactionError(format!("failed_to_commit_transaction: {}", e)))
        })?;

        info!("Applied migration {}_{}", migration.version, migration.name);
        Ok(())
    }

    /// Get all migrations that should be applied to the database
    fn get_migrations(&self) -> Vec<Migration> {
        // Create migrations for all our database objects
        vec![
            // Migration 1: Create tokens table
            Migration {
                version: 1,
                name: String::from("create_tokens_table"),
                sql: vec![
                    r#"
                CREATE TABLE IF NOT EXISTS tokens (
                    mint TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    symbol TEXT NOT NULL,
                    uri TEXT NOT NULL,
                    creator TEXT NOT NULL,
                    platform TEXT NOT NULL,
                    created_at BIGINT NOT NULL,
                    cex_sources TEXT[] DEFAULT NULL,
                    cex_updated_at BIGINT DEFAULT NULL,
                    updated_at BIGINT DEFAULT NULL,
                    associated_bonding_curve TEXT DEFAULT NULL,
                    is_bonded BOOLEAN NOT NULL DEFAULT FALSE,
                    bonded_at BIGINT DEFAULT NULL,
                    all_time_high_price BIGINT NOT NULL DEFAULT 0,
                    all_time_high_price_at BIGINT NOT NULL
                )
                "#,
                ],
            },
            // Migration 2: Create CEX metrics table
            Migration {
                version: 2,
                name: String::from("create_cex_metrics_table"),
                sql: vec![
                    r#"
                CREATE TABLE IF NOT EXISTS cex_metrics (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL,
                    address TEXT UNIQUE NOT NULL,
                    total_tokens BIGINT NOT NULL DEFAULT 0,
                    ath_tokens BIGINT NOT NULL DEFAULT 0,
                    first_seen_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                    last_token_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
                )
                "#,
                ],
            },
            // Migration 3: Create CEX-token relations table
            Migration {
                version: 3,
                name: String::from("create_cex_token_relations_table"),
                sql: vec![
                    r#"
                CREATE TABLE IF NOT EXISTS cex_token_relations (
                    id SERIAL PRIMARY KEY,
                    cex_address TEXT NOT NULL,
                    token_mint TEXT NOT NULL,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                    UNIQUE(cex_address, token_mint)
                )
                "#,
                ],
            },
            // Migration 4: Create CEX token ATH table
            Migration {
                version: 4,
                name: String::from("create_cex_token_ath_table"),
                sql: vec![
                    r#"
                CREATE TABLE IF NOT EXISTS cex_token_ath (
                    id SERIAL PRIMARY KEY,
                    cex_address TEXT NOT NULL,
                    token_mint TEXT NOT NULL,
                    ath_price BIGINT NOT NULL,
                    ath_at TIMESTAMP WITH TIME ZONE NOT NULL,
                    UNIQUE(cex_address, token_mint)
                )
                "#,
                ],
            },
            // Migration 5: Create indexes for tokens table
            Migration {
                version: 5,
                name: String::from("create_tokens_indexes"),
                sql: vec![
                    "CREATE INDEX IF NOT EXISTS idx_tokens_creator ON tokens(creator)",
                    "CREATE INDEX IF NOT EXISTS idx_tokens_mint ON tokens(mint)",
                ],
            },
            // Migration 6: Create indexes for CEX metrics table
            Migration {
                version: 6,
                name: String::from("create_cex_metrics_indexes"),
                sql: vec!["CREATE INDEX IF NOT EXISTS idx_cex_metrics_address ON cex_metrics(address)"],
            },
            // Migration 7: Create indexes for CEX-token relations table
            Migration {
                version: 7,
                name: String::from("create_cex_token_relations_indexes"),
                sql: vec![
                    "CREATE INDEX IF NOT EXISTS idx_cex_token_relations_cex ON cex_token_relations(cex_address)",
                    "CREATE INDEX IF NOT EXISTS idx_cex_token_relations_token ON cex_token_relations(token_mint)",
                ],
            },
            // Migration 8: Create indexes for CEX token ATH table
            Migration {
                version: 8,
                name: String::from("create_cex_token_ath_indexes"),
                sql: vec![
                    "CREATE INDEX IF NOT EXISTS idx_cex_token_ath_cex ON cex_token_ath(cex_address)",
                    "CREATE INDEX IF NOT EXISTS idx_cex_token_ath_token ON cex_token_ath(token_mint)",
                ],
            },
            // Migration 9: Create token price history table
            Migration {
                version: 9,
                name: String::from("create_token_price_history_table"),
                sql: vec![
                    r#"
                CREATE TABLE IF NOT EXISTS token_price_history (
                    id SERIAL PRIMARY KEY,
                    mint TEXT NOT NULL,
                    price BIGINT NOT NULL,
                    timestamp BIGINT NOT NULL,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                    UNIQUE(mint, timestamp)
                )
                "#,
                ],
            },
            // Migration 10: Create token volume history table
            Migration {
                version: 10,
                name: String::from("create_token_volume_history_table"),
                sql: vec![
                    r#"
                CREATE TABLE IF NOT EXISTS token_volume_history (
                    id SERIAL PRIMARY KEY,
                    mint TEXT NOT NULL,
                    volume BIGINT NOT NULL,
                    timestamp BIGINT NOT NULL,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                    UNIQUE(mint, timestamp)
                )
                "#,
                ],
            },
            // Migration 11: Create CEX activity history table
            Migration {
                version: 11,
                name: String::from("create_cex_activity_history_table"),
                sql: vec![
                    r#"
                CREATE TABLE IF NOT EXISTS cex_activity_history (
                    id SERIAL PRIMARY KEY,
                    cex_address TEXT NOT NULL,
                    token_count BIGINT NOT NULL,
                    timestamp BIGINT NOT NULL,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                    UNIQUE(cex_address, timestamp)
                )
                "#,
                ],
            },
            // Migration 12: Create indexes for time series tables
            Migration {
                version: 12,
                name: String::from("create_time_series_indexes"),
                sql: vec![
                    "CREATE INDEX IF NOT EXISTS idx_token_price_history_mint ON token_price_history(mint)",
                    "CREATE INDEX IF NOT EXISTS idx_token_price_history_timestamp ON token_price_history(timestamp)",
                    "CREATE INDEX IF NOT EXISTS idx_token_volume_history_mint ON token_volume_history(mint)",
                    "CREATE INDEX IF NOT EXISTS idx_token_volume_history_timestamp ON token_volume_history(timestamp)",
                    "CREATE INDEX IF NOT EXISTS idx_cex_activity_history_cex ON cex_activity_history(cex_address)",
                    "CREATE INDEX IF NOT EXISTS idx_cex_activity_history_timestamp ON cex_activity_history(timestamp)",
                ],
            },
            // Migration 13: Create PostGIS extension
            Migration {
                version: 13,
                name: String::from("create_postgis_extension"),
                sql: vec!["CREATE EXTENSION IF NOT EXISTS postgis"],
            },
            // Migration 14: Create pgRouting extension
            Migration {
                version: 14,
                name: String::from("create_pgrouting_extension"),
                sql: vec!["CREATE EXTENSION IF NOT EXISTS pgrouting"],
            },
            // Migration 15: Create wallet_nodes table
            Migration {
                version: 15,
                name: String::from("create_wallet_nodes_table"),
                sql: vec![
                    r#"
                CREATE TABLE IF NOT EXISTS wallet_nodes (
                    id SERIAL PRIMARY KEY,
                    pubkey TEXT UNIQUE NOT NULL,
                    is_cex BOOLEAN NOT NULL,
                    cex_name TEXT,
                    total_received FLOAT DEFAULT 0.0,
                    total_balance FLOAT DEFAULT 0.0,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
                )
                "#,
                ],
            },
            // Migration 16: Create wallet_edges table
            Migration {
                version: 16,
                name: String::from("create_wallet_edges_table"),
                sql: vec![
                    r#"
                CREATE TABLE IF NOT EXISTS wallet_edges (
                    id SERIAL PRIMARY KEY,
                    source_id INTEGER REFERENCES wallet_nodes(id),
                    target_id INTEGER REFERENCES wallet_nodes(id),
                    source_pubkey TEXT NOT NULL,
                    target_pubkey TEXT NOT NULL,
                    cost FLOAT DEFAULT 1.0,
                    reverse_cost FLOAT DEFAULT -1.0,
                    amount FLOAT NOT NULL,
                    timestamp BIGINT NOT NULL,
                    mint TEXT NOT NULL,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                    UNIQUE(source_pubkey, target_pubkey, mint, timestamp)
                )
                "#,
                ],
            },
            // Migration 17: Create indexes for wallet nodes and edges tables
            Migration {
                version: 17,
                name: String::from("create_wallet_indexes"),
                sql: vec![
                    "CREATE INDEX IF NOT EXISTS idx_wallet_nodes_pubkey ON wallet_nodes(pubkey)",
                    "CREATE INDEX IF NOT EXISTS idx_wallet_edges_source_target ON wallet_edges(source_id, target_id)",
                    "CREATE INDEX IF NOT EXISTS idx_wallet_edges_pubkeys ON wallet_edges(source_pubkey, target_pubkey)",
                    "CREATE INDEX IF NOT EXISTS idx_wallet_edges_mint ON wallet_edges(mint)",
                ],
            },
            // Migration 18: Add missing columns to tokens table
            Migration {
                version: 18,
                name: String::from("add_missing_tokens_columns"),
                sql: vec![
                    "ALTER TABLE tokens ADD COLUMN IF NOT EXISTS updated_at BIGINT DEFAULT NULL",
                    "ALTER TABLE tokens ADD COLUMN IF NOT EXISTS cex_sources TEXT[] DEFAULT NULL",
                    "ALTER TABLE tokens ADD COLUMN IF NOT EXISTS cex_updated_at BIGINT DEFAULT NULL",
                    "ALTER TABLE tokens ADD COLUMN IF NOT EXISTS associated_bonding_curve TEXT DEFAULT NULL",
                    "ALTER TABLE tokens ADD COLUMN IF NOT EXISTS is_bonded BOOLEAN NOT NULL DEFAULT FALSE",
                    "ALTER TABLE tokens ADD COLUMN IF NOT EXISTS bonded_at BIGINT DEFAULT NULL",
                    "ALTER TABLE tokens ADD COLUMN IF NOT EXISTS all_time_high_price BIGINT NOT NULL DEFAULT 0",
                    "ALTER TABLE tokens ADD COLUMN IF NOT EXISTS all_time_high_price_at BIGINT NOT NULL DEFAULT 0",
                ],
            },
        ]
    }
}
