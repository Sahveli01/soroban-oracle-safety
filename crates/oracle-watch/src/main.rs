//! oracle-watch — off-chain Stellar SDEX liquidity watcher.
//!
//! Polls Horizon for trade data, aggregates volume and trade counts,
//! submits signed snapshots to the on-chain `LiquidityRegistry`, and
//! dispatches alerts on anomalies via the pluggable `WebhookSink` trait.
//!
//! # Spec reference
//!
//! See spec Bölüm 5 (oracle-watch responsibilities) and Bölüm 7
//! (write-loop diagram).
//!
//! # Module composition
//!
//! - `config` — environment configuration
//! - `horizon_client` — SDEX trade polling
//! - `aggregator` — volume + trade count computation
//! - `signer` — Ed25519 attestation signing
//! - `registry_writer` — on-chain snapshot submission
//! - `monitor` — anomaly detection + alert dispatch
//! - `discord_sink`, `telegram_sink` — WebhookSink implementations
//! - `types` — shared data types
//!
//! # Operational model
//!
//! Poll every `poll_interval_ledgers * 5` seconds (≈25s at default 5
//! ledgers). For each watched asset: fetch trades → aggregate → detect
//! anomalies → dispatch alerts → write snapshot to registry.
//!
//! Graceful shutdown on SIGINT (Ctrl+C). The current iteration
//! completes before the loop exits.

mod aggregator;
mod config;
mod discord_sink;
mod horizon_client;
mod monitor;
mod registry_writer;
mod signer;
mod telegram_sink;
mod types;

use std::time::Duration;

use crate::aggregator::aggregate_trades;
use crate::config::Config;
use crate::horizon_client::HorizonClient;
use crate::monitor::{dispatch_alerts, AlertConfig, AnomalyDetector};
use crate::registry_writer::RegistryWriter;
use crate::signer::Signer;
use crate::types::{AggregatedSnapshot, LiquiditySnapshotPayload};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("oracle-watch starting...");

    let config = Config::from_env()?;
    let alert_config = AlertConfig::from_env();

    eprintln!(
        "oracle-watch config loaded: {} watched assets, poll interval {}s, max snapshot age {}s",
        config.watched_assets.len(),
        config.poll_interval_ledgers * 5,
        config.max_snapshot_age_seconds
    );

    let signer = Signer::from_hex_secret(&config.signing_secret_key)
        .map_err(|e| format!("signer init failed: {e}"))?;
    eprintln!(
        "oracle-watch attester pubkey: {}",
        hex::encode(signer.public_key_bytes())
    );

    let horizon = HorizonClient::new(config.horizon_url.clone());
    let registry_writer = RegistryWriter::new(
        config.soroban_rpc_url.clone(),
        config.registry_contract_id.clone(),
        config.network_passphrase.clone(),
        config.signing_secret_key.clone(),
    );
    eprintln!(
        "oracle-watch registry: rpc={} contract={} network=\"{}\"",
        registry_writer.rpc_url(),
        registry_writer.contract_id(),
        registry_writer.network_passphrase()
    );

    let detector = AnomalyDetector::default();
    let sinks = alert_config.build_sinks();
    eprintln!("oracle-watch alert sinks configured: {}", sinks.len());

    let poll_seconds = (config.poll_interval_ledgers as u64) * 5;
    let mut prev_snapshots: std::collections::HashMap<String, AggregatedSnapshot> =
        std::collections::HashMap::new();

    eprintln!("oracle-watch entering poll loop");
    loop {
        tokio::select! {
            _ = run_iteration(&config, &horizon, &registry_writer, &signer, &detector, &sinks, &mut prev_snapshots) => {}
            _ = tokio::signal::ctrl_c() => {
                eprintln!("oracle-watch received SIGINT, shutting down");
                break;
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(poll_seconds)) => {}
            _ = tokio::signal::ctrl_c() => {
                eprintln!("oracle-watch received SIGINT during sleep, shutting down");
                break;
            }
        }
    }

    eprintln!("oracle-watch exited cleanly");
    Ok(())
}

/// Single poll iteration across all watched assets.
///
/// For each asset:
/// 1. Fetch recent trades from Horizon (limit 200, asset/USDC pair)
/// 2. Aggregate into `AggregatedSnapshot`
/// 3. Detect anomalies vs previous snapshot
/// 4. Dispatch alerts via configured sinks
/// 5. Write snapshot to LiquidityRegistry (stub in Phase 6.5; real
///    submission Phase 8)
///
/// Per-asset failures are logged and do not abort the iteration —
/// other assets continue to be polled.
async fn run_iteration(
    config: &Config,
    horizon: &HorizonClient,
    registry_writer: &RegistryWriter,
    signer: &Signer,
    detector: &AnomalyDetector,
    sinks: &[Box<dyn monitor::WebhookSink>],
    prev_snapshots: &mut std::collections::HashMap<String, AggregatedSnapshot>,
) {
    let mut iteration_anomaly_count = 0;
    let mut iteration_write_attempts = 0;
    let mut iteration_write_failures = 0;
    let mut iteration_sign_failures = 0;

    for asset in &config.watched_assets {
        // Currently watching every asset against USDC as counter. Phase 8
        // may parameterize counter selection per asset (XLM-pair, etc.).
        let trades = match horizon
            .get_recent_trades(&asset.code, &asset.issuer, "USDC", "native", 200)
            .await
        {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "oracle-watch fetch failed for {}/{}: {e}",
                    asset.code, asset.issuer
                );
                continue;
            }
        };

        // Phase 6.3 takes the same trades for both 30m and 1h windows
        // when no time-filter is applied. Phase 8 will pre-filter by
        // ledger_close_time before passing.
        let snapshot = aggregate_trades(
            &asset.code,
            &asset.issuer,
            &trades,
            &trades,
            config.usdc_price_usd,
        );

        let asset_key = format!("{}/{}", asset.code, asset.issuer);

        // Anomaly check (only if we have a prev snapshot for delta)
        if let Some(prev) = prev_snapshots.get(&asset_key) {
            let prev_price = config.usdc_price_usd;
            let curr_price = config.usdc_price_usd;
            let anomalies = detector.check(prev, &snapshot, prev_price, curr_price);
            iteration_anomaly_count += anomalies.len();
            if !anomalies.is_empty() {
                dispatch_alerts(&anomalies, sinks).await;
            }
        }

        // Snapshot write (Phase 6.5 stub — Phase 8 wires real submission)
        match registry_writer.build_invoke_args(&snapshot) {
            Ok(_args) => {
                iteration_write_attempts += 1;
                // In Phase 6.8 we don't actually submit (no live testnet account).
                // Phase 8 will add: registry_writer.submit_transaction_stub(envelope_xdr).
                // For now, the build itself validates the path end-to-end.
            }
            Err(e) => {
                iteration_write_failures += 1;
                eprintln!("oracle-watch snapshot build failed for {asset_key}: {e}");
            }
        }

        // Off-chain ed25519 attestation signature (spec compliance — see
        // signer.rs design note; redundant with Stellar require_auth_for_args
        // but produced for forward-compat and independent off-chain audit).
        let payload = LiquiditySnapshotPayload::from(&snapshot);
        if let Err(e) = signer.sign_snapshot(&payload) {
            iteration_sign_failures += 1;
            eprintln!("oracle-watch sign failed for {asset_key}: {e}");
        }

        prev_snapshots.insert(asset_key, snapshot);
    }

    eprintln!(
        "oracle-watch iteration: {} assets, {} anomalies, {} writes, {} write-failures, {} sign-failures",
        config.watched_assets.len(),
        iteration_anomaly_count,
        iteration_write_attempts,
        iteration_write_failures,
        iteration_sign_failures
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn sample_trade_response() -> &'static str {
        r#"{
          "_embedded": {
            "records": [
              {
                "id": "1-0",
                "ledger_close_time": "2026-05-06T10:00:00Z",
                "base_amount": "100.0000000",
                "counter_amount": "100.0000000",
                "price_r": { "n": 1, "d": 1 },
                "base_account": "GACCT1XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
              },
              {
                "id": "2-0",
                "ledger_close_time": "2026-05-06T09:59:55Z",
                "base_amount": "200.0000000",
                "counter_amount": "200.0000000",
                "price_r": { "n": 1, "d": 1 },
                "base_account": "GACCT2YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY"
              }
            ]
          }
        }"#
    }

    fn make_test_config(horizon_url: String) -> Config {
        Config {
            horizon_url,
            soroban_rpc_url: "https://example.test/rpc".to_string(),
            registry_contract_id:
                "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            poll_interval_ledgers: 5,
            signing_secret_key: "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
                .to_string(),
            watched_assets: vec![config::WatchedAsset {
                code: "TESTASSET".to_string(),
                issuer: "GISSUER".to_string(),
            }],
            max_snapshot_age_seconds: 300,
            usdc_price_usd: 1.0,
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
        }
    }

    #[tokio::test]
    async fn test_single_iteration_completes() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/trades\?.*".to_string()))
            .with_status(200)
            .with_body(sample_trade_response())
            .create_async()
            .await;

        let config = make_test_config(server.url());
        let horizon = HorizonClient::new(config.horizon_url.clone());
        let registry_writer = RegistryWriter::new(
            config.soroban_rpc_url.clone(),
            config.registry_contract_id.clone(),
            config.network_passphrase.clone(),
            config.signing_secret_key.clone(),
        );
        let signer = Signer::from_hex_secret(&config.signing_secret_key).unwrap();
        let detector = AnomalyDetector::default();
        let sinks: Vec<Box<dyn monitor::WebhookSink>> = Vec::new();
        let mut prev_snapshots = std::collections::HashMap::new();

        // First iteration: no previous snapshot, no anomaly check possible
        run_iteration(
            &config,
            &horizon,
            &registry_writer,
            &signer,
            &detector,
            &sinks,
            &mut prev_snapshots,
        )
        .await;

        // After iteration, prev_snapshots should contain the asset
        let key = format!("{}/{}", "TESTASSET", "GISSUER");
        assert!(prev_snapshots.contains_key(&key));
    }

    #[tokio::test]
    async fn test_two_iterations_anomaly_detection_path() {
        // Simulates 2 polls: first establishes baseline, second detects
        // (or doesn't detect) anomaly based on snapshot delta.
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/trades\?.*".to_string()))
            .with_status(200)
            .with_body(sample_trade_response())
            .expect_at_least(2)
            .create_async()
            .await;

        let config = make_test_config(server.url());
        let horizon = HorizonClient::new(config.horizon_url.clone());
        let registry_writer = RegistryWriter::new(
            config.soroban_rpc_url.clone(),
            config.registry_contract_id.clone(),
            config.network_passphrase.clone(),
            config.signing_secret_key.clone(),
        );
        let signer = Signer::from_hex_secret(&config.signing_secret_key).unwrap();
        let detector = AnomalyDetector::default();
        let sinks: Vec<Box<dyn monitor::WebhookSink>> = Vec::new();
        let mut prev_snapshots = std::collections::HashMap::new();

        // Iteration 1
        run_iteration(
            &config,
            &horizon,
            &registry_writer,
            &signer,
            &detector,
            &sinks,
            &mut prev_snapshots,
        )
        .await;

        // Iteration 2 (now we have prev snapshot, anomaly check runs)
        run_iteration(
            &config,
            &horizon,
            &registry_writer,
            &signer,
            &detector,
            &sinks,
            &mut prev_snapshots,
        )
        .await;

        // Both iterations completed without panic
        assert_eq!(prev_snapshots.len(), 1);
    }

    #[tokio::test]
    async fn test_iteration_horizon_failure_does_not_panic() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/trades\?.*".to_string()))
            .with_status(500)
            .with_body("internal error")
            .create_async()
            .await;

        let config = make_test_config(server.url());
        let horizon = HorizonClient::new(config.horizon_url.clone());
        let registry_writer = RegistryWriter::new(
            config.soroban_rpc_url.clone(),
            config.registry_contract_id.clone(),
            config.network_passphrase.clone(),
            config.signing_secret_key.clone(),
        );
        let signer = Signer::from_hex_secret(&config.signing_secret_key).unwrap();
        let detector = AnomalyDetector::default();
        let sinks: Vec<Box<dyn monitor::WebhookSink>> = Vec::new();
        let mut prev_snapshots = std::collections::HashMap::new();

        // Iteration must absorb the Horizon 500 error and continue
        run_iteration(
            &config,
            &horizon,
            &registry_writer,
            &signer,
            &detector,
            &sinks,
            &mut prev_snapshots,
        )
        .await;

        // Asset should NOT be in prev_snapshots (fetch failed before snapshot creation)
        let key = format!("{}/{}", "TESTASSET", "GISSUER");
        assert!(!prev_snapshots.contains_key(&key));
    }
}
