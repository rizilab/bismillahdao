// ─────────────────────────────────────────────────────────────────────────────
//  Baseer — Wallet Analyzer
//  Part of the Al-Hafiz Project, the Guardian Layer of BismillahDAO.
//
//  Baseer (بصير): "The Analyzer" — provides deep insight into wallet activities,
//  protecting traders through transparency and detection of hidden risks.
//
//  Designed to preserve trust, empower security, and uphold responsibility in Web3.
//
//  In the name of Allah, the Most Gracious, the Most Merciful.
// ─────────────────────────────────────────────────────────────────────────────

use muhafidh::engine::baseer::Baseer;
use muhafidh::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
  Baseer::run().await?;
  Ok(())
}
