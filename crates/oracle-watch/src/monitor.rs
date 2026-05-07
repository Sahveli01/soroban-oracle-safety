//! Anomaly detection for liquidity snapshots.
//!
//! Pure-function detection module. Compares consecutive snapshots and
//! current oracle prices against configurable thresholds, surfacing
//! anomalies that would warrant operator alerts.
//!
//! # Phase 6.6 vs 6.7
//!
//! This module performs **detection only** — it returns a `Vec<Anomaly>`
//! describing what was found. **Alert dispatch** (Discord/Telegram
//! webhook posting) is implemented in Phase 6.7. The two responsibilities
//! are separated to keep detection pure and easily testable.
//!
//! # Spec reference
//!
//! See spec Bölüm 5 — oracle-watch İşlev B.

use crate::aggregator::MIN_TRADE_USD_VALUE;
use crate::types::AggregatedSnapshot;

/// Anomaly detection thresholds.
///
/// All thresholds are inclusive (`<=` for low values, `>=` for high).
/// The defaults reflect operational expectations for a reasonably-active
/// SDEX pair on testnet/mainnet:
///
/// - **`max_price_change_bps = 2000`** (20%): a single-poll price move
///   exceeding 20% is structurally unusual and worth surfacing.
/// - **`min_volume_threshold_usd = 10_000.0`**: less than $10k of 30-min
///   volume on a watched asset is too thin to support reliable Layer 2
///   liquidity attestation.
/// - **`min_trade_count = 5`**: fewer than 5 unique trades in 1 hour is
///   the spec's structural thin-sampling boundary.
#[derive(Debug, Clone, Copy)]
pub struct AnomalyDetector {
    pub max_price_change_bps: u32,
    pub min_volume_threshold_usd: f64,
    pub min_trade_count: u32,
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self {
            max_price_change_bps: 2_000,
            min_volume_threshold_usd: 10_000.0,
            min_trade_count: 5,
        }
    }
}

/// A detected anomaly worth surfacing to operators.
///
/// Each variant carries enough context to populate a human-readable
/// alert (asset identification, observed values, threshold reference).
#[derive(Debug, Clone, PartialEq)]
pub enum Anomaly {
    /// Single-poll price movement exceeded the configured BPS threshold.
    ExcessivePriceChange {
        asset_code: String,
        asset_issuer: String,
        prev_price: f64,
        curr_price: f64,
        change_bps: u32,
        threshold_bps: u32,
    },

    /// 30-minute USD volume fell below the operator threshold.
    ///
    /// Note: This is distinct from on-chain `Layer2::InsufficientLiquidity`,
    /// which uses a per-config `min_liquidity_usd` (typically $100k+).
    /// This off-chain monitor threshold is set lower to surface
    /// degrading-liquidity warnings before they reach the on-chain
    /// fail-safe boundary.
    InsufficientLiquidity {
        asset_code: String,
        asset_issuer: String,
        volume_usd: f64,
        threshold_usd: f64,
    },

    /// Unique trade count fell below the structural thin-sampling threshold.
    ThinSampling {
        asset_code: String,
        asset_issuer: String,
        trade_count: u32,
        threshold_count: u32,
    },
}

impl AnomalyDetector {
    /// Detects anomalies between two consecutive snapshots and a current price.
    ///
    /// # Parameters
    ///
    /// - `prev_snapshot`: the previous snapshot (e.g., from the previous poll)
    /// - `curr_snapshot`: the current snapshot just computed
    /// - `prev_price`: the asset's price at the previous poll (USD)
    /// - `curr_price`: the asset's current price (USD)
    ///
    /// # Returns
    ///
    /// A vector of `Anomaly` instances. Empty vector means no anomalies
    /// detected — operationally healthy state.
    ///
    /// # Detection rules
    ///
    /// 1. **ExcessivePriceChange**: if `|curr_price - prev_price| / prev_price`
    ///    in BPS exceeds `max_price_change_bps`. NaN/inf/zero `prev_price`
    ///    are skipped (indeterminate change cannot be classified as anomaly).
    /// 2. **InsufficientLiquidity**: if `curr_snapshot.volume_30m_usd_i128`
    ///    converted to USD is below `min_volume_threshold_usd`.
    /// 3. **ThinSampling**: if `curr_snapshot.unique_trades_1h` is below
    ///    `min_trade_count`.
    ///
    /// All three rules are checked independently; the returned vector may
    /// contain 0, 1, 2, or 3 anomalies.
    pub fn check(
        &self,
        prev_snapshot: &AggregatedSnapshot,
        curr_snapshot: &AggregatedSnapshot,
        prev_price: f64,
        curr_price: f64,
    ) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        // Rule 1: ExcessivePriceChange
        if let Some(change_bps) = compute_price_change_bps(prev_price, curr_price) {
            if change_bps > self.max_price_change_bps {
                anomalies.push(Anomaly::ExcessivePriceChange {
                    asset_code: curr_snapshot.asset_code.clone(),
                    asset_issuer: curr_snapshot.asset_issuer.clone(),
                    prev_price,
                    curr_price,
                    change_bps,
                    threshold_bps: self.max_price_change_bps,
                });
            }
        }

        // Rule 2: InsufficientLiquidity
        let volume_usd = stroops_to_usd(curr_snapshot.volume_30m_usd_i128);
        if volume_usd < self.min_volume_threshold_usd {
            anomalies.push(Anomaly::InsufficientLiquidity {
                asset_code: curr_snapshot.asset_code.clone(),
                asset_issuer: curr_snapshot.asset_issuer.clone(),
                volume_usd,
                threshold_usd: self.min_volume_threshold_usd,
            });
        }

        // Rule 3: ThinSampling
        if curr_snapshot.unique_trades_1h < self.min_trade_count {
            anomalies.push(Anomaly::ThinSampling {
                asset_code: curr_snapshot.asset_code.clone(),
                asset_issuer: curr_snapshot.asset_issuer.clone(),
                trade_count: curr_snapshot.unique_trades_1h,
                threshold_count: self.min_trade_count,
            });
        }

        // Phase 6.6: log to stdout. Phase 6.7 will replace with webhook dispatch.
        if !anomalies.is_empty() {
            for anomaly in &anomalies {
                eprintln!("ORACLE-WATCH ANOMALY: {anomaly:?}");
            }
            // TODO Phase 6.7: dispatch_alerts(&anomalies, &alert_config).await
        }

        // prev_snapshot is currently only used as future-facing context
        // (e.g., volume-trend or trade-count-delta detection in Phase 7+).
        // Kept in signature for API stability.
        let _ = prev_snapshot;

        anomalies
    }
}

/// Computes price change in basis points between two prices.
///
/// Returns `None` if `prev_price` is non-positive or non-finite (indeterminate
/// change), or if the BPS computation overflows or produces non-finite output.
/// A return of `Some(bps)` is always a finite, non-negative u32.
fn compute_price_change_bps(prev_price: f64, curr_price: f64) -> Option<u32> {
    if !prev_price.is_finite() || !curr_price.is_finite() || prev_price <= 0.0 {
        return None;
    }

    let diff = (curr_price - prev_price).abs();
    let ratio = diff / prev_price;
    let bps = (ratio * 10_000.0).round();

    if !bps.is_finite() || bps < 0.0 {
        return None;
    }

    if bps >= u32::MAX as f64 {
        return Some(u32::MAX);
    }

    Some(bps as u32)
}

/// Converts i128 stroops back to USD f64.
///
/// Inverse of the aggregator's STROOP_MULTIPLIER scaling. Used here to
/// compare on-chain-formatted volume against the operator's USD-cents
/// threshold.
fn stroops_to_usd(stroops: i128) -> f64 {
    stroops as f64 / 10_000_000.0
}

// Reference to MIN_TRADE_USD_VALUE for documentation cross-link consistency.
// (Currently informational; Phase 7+ may use this to refine thresholds.)
const _MIN_TRADE_VALUE_DOC_REF: f64 = MIN_TRADE_USD_VALUE;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(
        asset: &str,
        issuer: &str,
        volume_stroops: i128,
        trades: u32,
    ) -> AggregatedSnapshot {
        AggregatedSnapshot {
            asset_code: asset.to_string(),
            asset_issuer: issuer.to_string(),
            volume_30m_usd_i128: volume_stroops,
            unique_trades_1h: trades,
            computed_at: 1_715_000_000,
        }
    }

    fn healthy_snapshot() -> AggregatedSnapshot {
        // $50k volume, 25 unique trades — well above defaults
        make_snapshot("USDC", "GA5ZSEJ", 500_000_000_000, 25)
    }

    // ===== compute_price_change_bps tests =====

    #[test]
    fn test_price_change_bps_no_change() {
        assert_eq!(compute_price_change_bps(1.0, 1.0), Some(0));
    }

    #[test]
    fn test_price_change_bps_10_percent() {
        // 10% increase = 1000 BPS
        assert_eq!(compute_price_change_bps(1.0, 1.1), Some(1000));
    }

    #[test]
    fn test_price_change_bps_10_percent_decrease() {
        // 10% decrease = 1000 BPS (absolute value)
        assert_eq!(compute_price_change_bps(1.0, 0.9), Some(1000));
    }

    #[test]
    fn test_price_change_bps_zero_prev_returns_none() {
        assert_eq!(compute_price_change_bps(0.0, 1.0), None);
    }

    #[test]
    fn test_price_change_bps_negative_prev_returns_none() {
        assert_eq!(compute_price_change_bps(-1.0, 1.0), None);
    }

    #[test]
    fn test_price_change_bps_nan_returns_none() {
        assert_eq!(compute_price_change_bps(f64::NAN, 1.0), None);
        assert_eq!(compute_price_change_bps(1.0, f64::NAN), None);
    }

    #[test]
    fn test_price_change_bps_inf_returns_none() {
        assert_eq!(compute_price_change_bps(f64::INFINITY, 1.0), None);
        assert_eq!(compute_price_change_bps(1.0, f64::INFINITY), None);
    }

    // ===== stroops_to_usd =====

    #[test]
    fn test_stroops_to_usd() {
        assert_eq!(stroops_to_usd(10_000_000), 1.0);
        assert_eq!(stroops_to_usd(50_000_000_000), 5_000.0);
        assert_eq!(stroops_to_usd(0), 0.0);
    }

    // ===== AnomalyDetector::default =====

    #[test]
    fn test_detector_defaults() {
        let d = AnomalyDetector::default();
        assert_eq!(d.max_price_change_bps, 2_000);
        assert_eq!(d.min_volume_threshold_usd, 10_000.0);
        assert_eq!(d.min_trade_count, 5);
    }

    // ===== check() — no anomalies =====

    #[test]
    fn test_check_healthy_returns_empty() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        let anomalies = detector.check(&prev, &curr, 1.0, 1.05); // 5% < 20% threshold
        assert!(anomalies.is_empty());
    }

    // ===== check() — single anomalies =====

    #[test]
    fn test_check_excessive_price_change_detected() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        // 50% increase = 5000 BPS > 2000 threshold
        let anomalies = detector.check(&prev, &curr, 1.0, 1.5);
        assert_eq!(anomalies.len(), 1);
        match &anomalies[0] {
            Anomaly::ExcessivePriceChange {
                change_bps,
                threshold_bps,
                ..
            } => {
                assert_eq!(*change_bps, 5000);
                assert_eq!(*threshold_bps, 2000);
            }
            other => panic!("expected ExcessivePriceChange, got {other:?}"),
        }
    }

    #[test]
    fn test_check_insufficient_liquidity_detected() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        // $5k volume — below $10k threshold
        let curr = make_snapshot("USDC", "GA5ZSEJ", 50_000_000_000, 25);
        let anomalies = detector.check(&prev, &curr, 1.0, 1.0);
        assert_eq!(anomalies.len(), 1);
        match &anomalies[0] {
            Anomaly::InsufficientLiquidity {
                volume_usd,
                threshold_usd,
                ..
            } => {
                assert_eq!(*volume_usd, 5_000.0);
                assert_eq!(*threshold_usd, 10_000.0);
            }
            other => panic!("expected InsufficientLiquidity, got {other:?}"),
        }
    }

    #[test]
    fn test_check_thin_sampling_detected() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        // 3 unique trades — below 5 threshold
        let curr = make_snapshot("USDC", "GA5ZSEJ", 500_000_000_000, 3);
        let anomalies = detector.check(&prev, &curr, 1.0, 1.0);
        assert_eq!(anomalies.len(), 1);
        match &anomalies[0] {
            Anomaly::ThinSampling {
                trade_count,
                threshold_count,
                ..
            } => {
                assert_eq!(*trade_count, 3);
                assert_eq!(*threshold_count, 5);
            }
            other => panic!("expected ThinSampling, got {other:?}"),
        }
    }

    // ===== check() — multiple anomalies =====

    #[test]
    fn test_check_yieldblox_scenario_all_three() {
        // YieldBlox-style attack: thin liquidity + price spike + thin sampling
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = make_snapshot("USDC", "GA5ZSEJ", 50_000_000, 1); // $5 vol, 1 trade
        let anomalies = detector.check(&prev, &curr, 1.0, 5.0); // 400% spike

        assert_eq!(anomalies.len(), 3);
        let kinds: Vec<&str> = anomalies
            .iter()
            .map(|a| match a {
                Anomaly::ExcessivePriceChange { .. } => "price",
                Anomaly::InsufficientLiquidity { .. } => "liquidity",
                Anomaly::ThinSampling { .. } => "thin",
            })
            .collect();
        assert!(kinds.contains(&"price"));
        assert!(kinds.contains(&"liquidity"));
        assert!(kinds.contains(&"thin"));
    }

    // ===== check() — boundary conditions =====

    #[test]
    fn test_check_boundary_exactly_at_threshold_passes() {
        // Volume exactly at threshold ($10k = 100_000_000_000 stroops): no anomaly
        // (rule is strict <, not <=)
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = make_snapshot("USDC", "GA5ZSEJ", 100_000_000_000, 5);
        let anomalies = detector.check(&prev, &curr, 1.0, 1.0);
        // Volume exact threshold → not below → no anomaly. Trade count exact → not below → no anomaly.
        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_check_price_at_threshold_no_anomaly() {
        // Exactly 20% (= 2000 BPS) is the threshold; not exceeded.
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        let anomalies = detector.check(&prev, &curr, 1.0, 1.2); // exactly 2000 BPS
                                                                // Strict `>`: equal-to threshold not flagged.
        assert!(anomalies
            .iter()
            .all(|a| !matches!(a, Anomaly::ExcessivePriceChange { .. })));
    }

    // ===== check() — defensive: NaN price doesn't crash =====

    #[test]
    fn test_check_nan_prices_skipped() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        // NaN prices: ExcessivePriceChange skipped (no flag), other rules unaffected
        let anomalies = detector.check(&prev, &curr, f64::NAN, 1.0);
        assert!(anomalies
            .iter()
            .all(|a| !matches!(a, Anomaly::ExcessivePriceChange { .. })));
    }

    #[test]
    fn test_check_zero_prev_price_skipped() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        let anomalies = detector.check(&prev, &curr, 0.0, 1.0);
        assert!(anomalies
            .iter()
            .all(|a| !matches!(a, Anomaly::ExcessivePriceChange { .. })));
    }

    // ===== Custom thresholds =====

    #[test]
    fn test_check_custom_thresholds() {
        let detector = AnomalyDetector {
            max_price_change_bps: 500,             // 5% — much stricter
            min_volume_threshold_usd: 1_000_000.0, // $1M — much higher
            min_trade_count: 100,                  // 100 trades — much higher
        };
        let prev = healthy_snapshot();
        let curr = healthy_snapshot(); // $50k vol, 25 trades (now both below)
        let anomalies = detector.check(&prev, &curr, 1.0, 1.06); // 6% > 5%

        assert_eq!(anomalies.len(), 3);
    }
}
