use anyhow::Result;
use sqlx::{Pool, Postgres};
use tracing::info;

pub async fn run_migrations(pool: &Pool<Postgres>) -> Result<()> {
    info!("Running database migrations...");
    
    // Create users table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            username VARCHAR(255) NOT NULL UNIQUE,
            email VARCHAR(255) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;
    
    info!("Database migrations completed successfully");
    
    Ok(())
} 