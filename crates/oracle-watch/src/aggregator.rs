//! Aggregates raw trade records into volume and trade-count snapshots.
//!
//! Pure-function module — no I/O, no async, fully deterministic given
//! identical inputs. Implements the spec's "Trade Sayım Tanımı":
//!
//! - 30-minute USD volume sum (i128 stroops, 7-decimal convention)
//! - 1-hour unique trade count (distinct `source_account`, $10 minimum)
//!
//! # Spec reference
//!
//! See spec Bölüm 5 — oracle-watch İşlev A.

use crate::types::{AggregatedSnapshot, TradeRecord};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

/// Stroop multiplier — Stellar's standard 7-decimal convention.
///
/// USD values are scaled by this factor when emitted as `i128` for
/// on-chain consumption. Example: `$1.50` → `15_000_000` stroops.
const STROOP_MULTIPLIER: i128 = 10_000_000;

/// Minimum trade USD value to count toward `unique_trades_1h`.
///
/// Trades below this threshold are excluded from the unique-account count
/// to defend against Sybil-style wash trading where an attacker spawns
/// many tiny trades from distinct accounts. Per spec's Trade Sayım Tanımı.
pub const MIN_TRADE_USD_VALUE: f64 = 10.0;

/// Aggregates trades into a snapshot ready for on-chain submission.
///
/// # Parameters
///
/// - `asset_code` / `asset_issuer`: identifies which asset this snapshot
///   describes. Passed through to the output unchanged.
/// - `trades_30m`: trades within the last 30 minutes (caller filters by
///   `ledger_close_time`). Used for `volume_30m_usd_i128`.
/// - `trades_1h`: trades within the last 1 hour. Used for
///   `unique_trades_1h`.
/// - `usd_per_counter`: USD value of one counter-asset unit (e.g., if
///   counter is XLM and 1 XLM = $0.12, pass `0.12`). Used to convert
///   `counter_amount` to USD.
///
/// # USD valuation strategy
///
/// Each trade contributes `counter_amount * usd_per_counter` USD to the
/// volume. This assumes the **counter asset is the price reference** for
/// USD valuation — typical for SDEX pairs where the watched asset is the
/// base and trades against USDC or XLM as counter. If the watched asset
/// itself is USDC, the caller passes `usd_per_counter = 1.0` (USDC → USD
/// 1:1) and the math still works.
///
/// # Returns
///
/// An [`AggregatedSnapshot`] with `computed_at` set to current wall-clock
/// time. The `volume_30m_usd_i128` field is scaled by [`STROOP_MULTIPLIER`].
///
/// # Empty inputs
///
/// Empty `trades_30m` → `volume_30m_usd_i128 = 0`.
/// Empty `trades_1h` → `unique_trades_1h = 0`.
///
/// Both empty is valid (the asset had no SDEX activity in the window).
/// The Layer 2 guardrail will surface this as `InsufficientLiquidity`
/// downstream.
pub fn aggregate_trades(
    asset_code: &str,
    asset_issuer: &str,
    sac_contract_id: Option<String>,
    trades_30m: &[TradeRecord],
    trades_1h: &[TradeRecord],
    usd_per_counter: f64,
) -> AggregatedSnapshot {
    let volume_30m_usd_i128 = compute_volume_i128(trades_30m, usd_per_counter);
    let unique_trades_1h = count_unique_trades(trades_1h, usd_per_counter);

    let computed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    AggregatedSnapshot {
        asset_code: asset_code.to_string(),
        asset_issuer: asset_issuer.to_string(),
        sac_contract_id,
        volume_30m_usd_i128,
        unique_trades_1h,
        computed_at,
    }
}

/// Computes the 30-minute USD volume scaled to i128 stroops.
///
/// Sums `counter_amount * usd_per_counter` across all trades. Trades with
/// unparseable `counter_amount` are silently skipped (Horizon should not
/// emit these, but defensive against malformed input).
fn compute_volume_i128(trades: &[TradeRecord], usd_per_counter: f64) -> i128 {
    let mut total_usd: f64 = 0.0;

    for trade in trades {
        let counter_amount: f64 = match trade.counter_amount.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        total_usd += counter_amount * usd_per_counter;
    }

    // Scale to stroop units. Saturating cast handles f64 overflow gracefully:
    // a single asset's 30-minute USD volume cannot realistically reach
    // i128::MAX / 10^7 ≈ 1.7e31 USD, so this is defensive only.
    let scaled = total_usd * STROOP_MULTIPLIER as f64;
    if scaled.is_finite() {
        scaled as i128
    } else {
        0
    }
}

/// Counts distinct `source_account` values among trades meeting the $10
/// minimum value threshold.
///
/// Per spec: same `source_account` across multiple trades in the 1-hour
/// window counts once. Trades below `MIN_TRADE_USD_VALUE` are excluded
/// (Sybil spam filter).
fn count_unique_trades(trades: &[TradeRecord], usd_per_counter: f64) -> u32 {
    let mut accounts: HashSet<&str> = HashSet::new();

    for trade in trades {
        let counter_amount: f64 = match trade.counter_amount.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let usd_value = counter_amount * usd_per_counter;
        // NaN/inf would slip past `<` (NaN comparisons are always false), so
        // require finite first. Mirrors the `is_finite` guard in compute_volume_i128.
        if !usd_value.is_finite() || usd_value < MIN_TRADE_USD_VALUE {
            continue;
        }
        accounts.insert(&trade.source_account);
    }

    // Cap at u32::MAX (HashSet can technically hold more on 64-bit, but
    // 4 billion unique accounts on a single SDEX pair is impossible in
    // practice — defensive cast).
    accounts.len().min(u32::MAX as usize) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PriceRatio;

    fn make_trade(id: &str, counter: &str, source: &str) -> TradeRecord {
        TradeRecord {
            id: id.to_string(),
            ledger_close_time: "2026-05-06T10:00:00Z".to_string(),
            base_amount: "1.0000000".to_string(),
            counter_amount: counter.to_string(),
            price_r: PriceRatio { n: 1, d: 1 },
            source_account: source.to_string(),
        }
    }

    // ===== aggregate_trades tests =====

    #[test]
    fn test_aggregate_empty_inputs() {
        let snapshot = aggregate_trades("USDC", "GA5ZSEJ", None, &[], &[], 1.0);
        assert_eq!(snapshot.asset_code, "USDC");
        assert_eq!(snapshot.asset_issuer, "GA5ZSEJ");
        assert_eq!(snapshot.volume_30m_usd_i128, 0);
        assert_eq!(snapshot.unique_trades_1h, 0);
    }

    #[test]
    fn test_aggregate_passes_through_asset_identification() {
        let snapshot = aggregate_trades("XLM", "native", None, &[], &[], 0.12);
        assert_eq!(snapshot.asset_code, "XLM");
        assert_eq!(snapshot.asset_issuer, "native");
    }

    #[test]
    fn test_aggregate_computes_at_recent_timestamp() {
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let snapshot = aggregate_trades("USDC", "GA5ZSEJ", None, &[], &[], 1.0);
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(snapshot.computed_at >= before && snapshot.computed_at <= after);
    }

    // ===== compute_volume_i128 tests =====

    #[test]
    fn test_volume_single_trade() {
        let trades = vec![make_trade("1", "100.0000000", "GACCT1")];
        let v = compute_volume_i128(&trades, 1.0);
        // 100 USD × 10^7 = 1_000_000_000
        assert_eq!(v, 1_000_000_000);
    }

    #[test]
    fn test_volume_multiple_trades_summed() {
        let trades = vec![
            make_trade("1", "50.0", "GACCT1"),
            make_trade("2", "75.0", "GACCT2"),
            make_trade("3", "25.0", "GACCT3"),
        ];
        let v = compute_volume_i128(&trades, 1.0);
        // (50 + 75 + 25) USD × 10^7 = 1_500_000_000
        assert_eq!(v, 1_500_000_000);
    }

    #[test]
    fn test_volume_with_usd_per_counter_conversion() {
        // XLM/USDC pair: 1 XLM ≈ $0.12, counter_amount in XLM
        let trades = vec![
            make_trade("1", "1000.0", "GACCT1"), // 1000 XLM × $0.12 = $120
            make_trade("2", "500.0", "GACCT2"),  // 500 XLM × $0.12 = $60
        ];
        let v = compute_volume_i128(&trades, 0.12);
        // (120 + 60) USD × 10^7 = 1_800_000_000
        assert_eq!(v, 1_800_000_000);
    }

    #[test]
    fn test_volume_skips_unparseable_counter_amount() {
        let bad = make_trade("bad", "not-a-number", "GACCT1");
        let trades = vec![
            make_trade("1", "100.0", "GACCT1"),
            bad.clone(),
            make_trade("2", "50.0", "GACCT2"),
        ];
        let v = compute_volume_i128(&trades, 1.0);
        // (100 + 50) × 10^7 = 1_500_000_000 (bad trade skipped)
        assert_eq!(v, 1_500_000_000);
    }

    #[test]
    fn test_volume_empty_input_returns_zero() {
        let v = compute_volume_i128(&[], 1.0);
        assert_eq!(v, 0);
    }

    // ===== count_unique_trades tests =====

    #[test]
    fn test_unique_trades_distinct_accounts() {
        let trades = vec![
            make_trade("1", "100.0", "GACCT1"),
            make_trade("2", "100.0", "GACCT2"),
            make_trade("3", "100.0", "GACCT3"),
        ];
        let count = count_unique_trades(&trades, 1.0);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_unique_trades_same_account_counted_once() {
        // Spec: same source_account across multiple trades = 1 unique trade
        let trades = vec![
            make_trade("1", "100.0", "GACCT1"),
            make_trade("2", "200.0", "GACCT1"), // same account
            make_trade("3", "300.0", "GACCT1"), // same account
            make_trade("4", "100.0", "GACCT2"),
        ];
        let count = count_unique_trades(&trades, 1.0);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_unique_trades_below_minimum_value_excluded() {
        // Spec: trades < $10 USD excluded (spam filter)
        let trades = vec![
            make_trade("1", "5.0", "GACCT1"),   // $5 — below threshold
            make_trade("2", "9.99", "GACCT2"),  // $9.99 — below threshold
            make_trade("3", "10.0", "GACCT3"), // $10 — at threshold (excluded? need exact-boundary clarity)
            make_trade("4", "100.0", "GACCT4"), // $100 — counted
        ];
        let count = count_unique_trades(&trades, 1.0);
        // Strict `<` per implementation: $10 itself is excluded ($10 < $10 is false → included)
        // GACCT3 ($10.0) and GACCT4 ($100) both counted = 2
        assert_eq!(count, 2);
    }

    #[test]
    fn test_unique_trades_strictly_below_threshold_excluded() {
        // Boundary: counter_amount = 9.99 with usd_per_counter = 1.0 → $9.99 < $10 → excluded
        let trades = vec![
            make_trade("1", "9.99", "GACCT1"),
            make_trade("2", "9.999", "GACCT2"),
        ];
        let count = count_unique_trades(&trades, 1.0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_unique_trades_with_usd_per_counter_threshold() {
        // counter_amount = 100 XLM × $0.12 = $12 → above threshold
        // counter_amount = 50 XLM × $0.12 = $6 → below threshold
        let trades = vec![
            make_trade("1", "100.0", "GACCT1"), // $12 — counted
            make_trade("2", "50.0", "GACCT2"),  // $6 — excluded
        ];
        let count = count_unique_trades(&trades, 0.12);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_unique_trades_skips_unparseable_counter() {
        let trades = vec![
            make_trade("1", "100.0", "GACCT1"),
            make_trade("2", "not-a-number", "GACCT2"),
            make_trade("3", "50.0", "GACCT3"),
        ];
        let count = count_unique_trades(&trades, 1.0);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_unique_trades_skips_nan_counter() {
        // "NaN" parses successfully to f64::NAN, but `NaN < 10.0` is false so
        // without the explicit is_finite guard a NaN trade would slip past the
        // $10 spam filter. This test pins the defensive guard.
        let trades = vec![
            make_trade("1", "100.0", "GACCT1"),
            make_trade("2", "NaN", "GATTACKER"),
            make_trade("3", "inf", "GATTACKER2"),
        ];
        let count = count_unique_trades(&trades, 1.0);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_unique_trades_empty_input_returns_zero() {
        let count = count_unique_trades(&[], 1.0);
        assert_eq!(count, 0);
    }

    // ===== Integration: YieldBlox-style scenario =====

    #[test]
    fn test_aggregate_yieldblox_scenario_low_liquidity() {
        // Spec: thin liquidity scenario — single $5 trade in 30-min window
        // → volume below threshold, single account in 1h window
        let single_trade = vec![make_trade("1", "5.0", "GATTACKER")];
        let snapshot = aggregate_trades("USDC", "GA5ZSEJ", None, &single_trade, &single_trade, 1.0);
        // Volume: $5 × 10^7 = 50_000_000 stroops (below $10k Layer 2 threshold)
        assert_eq!(snapshot.volume_30m_usd_i128, 50_000_000);
        // Unique trades: 0 ($5 < $10 spam threshold → excluded)
        assert_eq!(snapshot.unique_trades_1h, 0);
    }

    #[test]
    fn test_aggregate_healthy_scenario() {
        let trades_30m = vec![
            make_trade("1", "10000.0", "GACCT1"),
            make_trade("2", "8000.0", "GACCT2"),
            make_trade("3", "5000.0", "GACCT3"),
        ];
        let trades_1h = trades_30m.clone(); // simplified for test
        let snapshot = aggregate_trades("USDC", "GA5ZSEJ", None, &trades_30m, &trades_1h, 1.0);

        // Volume: (10000 + 8000 + 5000) × 10^7 = 230_000_000_000 stroops ($23k)
        assert_eq!(snapshot.volume_30m_usd_i128, 230_000_000_000);
        // 3 distinct accounts, all above $10
        assert_eq!(snapshot.unique_trades_1h, 3);
    }
}
