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
use muhafidh::handler::shutdown::ShutdownSignal;
use muhafidh::storage::run_database_migrations;
use muhafidh::tracing::setup_tracing;
use tracing::error;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let shutdown_signal = ShutdownSignal::new();
    // Load configuration
    let config = load_config("Config.toml").await?;

    // Initialize logging
    info!("Initializing logging...");
    if let Err(e) = setup_tracing(config.clone(), "migrate", shutdown_signal.clone()).await {
        error!("failed_to_setup_tracing: {}", e);
    }

    // Run migrations
    run_database_migrations("migration-tool", &config).await?;

    info!("Database Migration Tool completed successfully");
    Ok(())
}
