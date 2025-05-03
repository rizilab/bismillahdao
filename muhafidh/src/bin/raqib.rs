// ─────────────────────────────────────────────────────────────────────────────
//  Raqib — Blockchain Activity Monitor
//  Part of the Al-Hafiz Project, the Guardian Layer of BismillahDAO.
//
//  Raqib (رقيب): "The Watchful Guardian" — monitors blockchain direct activity
//  to detect and highlight new activity..
//
//  Designed to watch diligently, fast, and accurately.
//
//  In the name of Allah, the Most Gracious, the Most Merciful.
// ─────────────────────────────────────────────────────────────────────────────
use muhafidh::config::load_config;
use muhafidh::engine::raqib::Raqib;
use muhafidh::error::Result;
use muhafidh::storage::StorageEngine;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
  Raqib::run().await?;
  Ok(())
}
