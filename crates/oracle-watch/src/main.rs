//! oracle-watch — off-chain Stellar SDEX liquidity watcher.
//!
//! Polls Horizon for trade data, aggregates volume and trade counts,
//! submits signed snapshots to the on-chain `LiquidityRegistry`, and
//! dispatches alerts on anomalies.
//!
//! Spec reference: Bölüm 5 (oracle-watch).
//!
//! Module composition:
//! - `config`: environment configuration
//! - `horizon_client`: SDEX trade polling
//! - `aggregator`: volume + trade count computation
//! - `signer`: Ed25519 attestation signing
//! - `registry_writer`: on-chain snapshot submission
//! - `monitor`: anomaly detection + alert dispatch
//! - `types`: shared data types

mod aggregator;
mod config;
mod horizon_client;
mod monitor;
mod registry_writer;
mod signer;
mod types;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO Phase 6.2: Config::from_env() loading
    // TODO Phase 6.2: HorizonClient initialization
    // TODO Phase 6.4: Signer initialization
    // TODO Phase 6.5: RegistryWriter initialization
    // TODO Phase 6.8: main poll loop (per-watched-asset every 5 ledgers)

    println!("oracle-watch starting (Phase 6.1 skeleton — main loop not yet implemented)");
    Ok(())
}
