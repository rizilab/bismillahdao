use std::sync::Arc;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::postgres::Postgres;
use sqlx::{PgPool, Pool, Postgres as SqlxPostgres};
use crate::Result;

/// Database test environment
pub struct TestDatabase {
    pub pool: Arc<PgPool>,
    pub container: ContainerAsync<Postgres>,
}

impl TestDatabase {
    /// Create a new test database instance
    pub async fn new() -> Result<Self> {
        let container = Postgres::default()
            .with_db_name("test_muhafidh")
            .with_user("test_user")
            .with_password("test_pass")
            .start()
            .await
            .map_err(|e| crate::error::StorageError::ConnectionError(format!("Failed to start test database: {}", e)))?;

        let host_port = container.get_host_port_ipv4(5432).await
            .map_err(|e| crate::error::StorageError::ConnectionError(format!("Failed to get database port: {}", e)))?;

        let database_url = format!(
            "postgresql://test_user:test_pass@127.0.0.1:{}/test_muhafidh",
            host_port
        );

        let pool = PgPool::connect(&database_url)
            .await
            .map_err(|e| crate::error::StorageError::ConnectionError(format!("Failed to connect to test database: {}", e)))?;

        Ok(Self {
            pool: Arc::new(pool),
            container,
        })
    }

    /// Get a clone of the database pool
    pub fn pool(&self) -> Arc<PgPool> {
        Arc::clone(&self.pool)
    }

    /// Clean all tables for a fresh test state
    pub async fn clean_tables(&self) -> Result<()> {
        let queries = vec![
            "TRUNCATE TABLE token_cex_sources CASCADE",
            "TRUNCATE TABLE cex_activities CASCADE", 
            "TRUNCATE TABLE creator_connection_graphs CASCADE",
        ];

        for query in queries {
            sqlx::query(query)
                .execute(self.pool.as_ref())
                .await
                .map_err(|e| crate::error::StorageError::QueryError(format!("Failed to clean table: {}", e)))?;
        }

        Ok(())
    }
} 