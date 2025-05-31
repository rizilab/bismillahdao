// ─────────────────────────────────────────────────────────────────────────────
//  Siraaj — Price Updater
//  Part of the Al-Hafiz Project, the Guardian Layer of BismillahDAO.
//
//  Siraj (سراج): "The Radiant Lamp" — continuously illuminates the trading path
//  by providing real-time price updates, empowering traders with clear information.
//
//  Designed to guide decisions, enhance awareness, and safeguard transactions.
//
//  In the name of Allah, the Most Gracious, the Most Merciful.
// ─────────────────────────────────────────────────────────────────────────────
use muhafidh::engine::siraaj::Siraaj;
use muhafidh::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    Siraaj::run().await?;
    Ok(())
}
