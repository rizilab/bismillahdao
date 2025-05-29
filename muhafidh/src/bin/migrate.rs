// ─────────────────────────────────────────────────────────────────────────────
//  Database Migration Tool
//  Part of the Al-Hafiz Project, the Guardian Layer of BismillahDAO.
//
//  Applies all database migrations before services are started.
//
//  In the name of Allah, the Most Gracious, the Most Merciful.
// ─────────────────────────────────────────────────────────────────────────────

use muhafidh::config::load_config;
use muhafidh::error::Result;
use muhafidh::setup_tracing;
use muhafidh::storage::run_database_migrations;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    setup_tracing("migrate");

    info!("Database Migration Tool starting...");

    // Load configuration
    let config = load_config("Config.toml")?;

    // Run migrations
    run_database_migrations("migration-tool", &config).await?;

    info!("Database Migration Tool completed successfully");
    Ok(())
}
