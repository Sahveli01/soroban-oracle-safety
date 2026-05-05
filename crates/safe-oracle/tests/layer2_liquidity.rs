//! Layer 2 Guardrail #4 — Minimum Liquidity (`check_liquidity`).
//!
//! These tests exercise the real cross-contract path: `lastprice` →
//! `check_liquidity` → `LiquidityRegistry::get_snapshot`. The Layer 1
//! guardrails are satisfied (relaxed deviation/staleness, no secondary) so
//! that each scenario isolates a single Layer 2 outcome — fail-safe on
//! missing snapshot, `StaleSnapshot` on age, `InsufficientLiquidity` on
//! threshold, and skip for off-chain (`Asset::Other`) assets.
//!
//! YieldBlox replica intentionally lives here rather than in
//! `layer1_integration.rs`: the attack's defining symptom is sub-threshold
//! SDEX volume, which is structurally a Layer 2 signal even though the
//! visible price move is what most observers notice first.

use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::{testutils::Address as _, Address, Symbol};
use test_utils::TestEnv;

/// YieldBlox-replica: Reflector reports a normal-looking price but the SDEX
/// order book has been drained (a $5 trade was enough to move the feed). The
/// 30-minute volume in the attested snapshot reflects that emptiness, and
/// `check_liquidity` blocks the borrow with `InsufficientLiquidity`.
///
/// Phase 4.4 broadens this into the full e2e attack scenario; here we
/// pin the single-guardrail behavior so a regression in `check_liquidity`
/// surfaces independently of the other guardrails.
#[test]
fn test_check_liquidity_blocks_yieldblox_thin_liquidity() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);

    // YieldBlox-state: market maker withdrew, 30m volume effectively $0
    // (5 stroops = $0.0000005 in 7-decimal). Above 0 so the registry's own
    // `volume <= 0` guard accepts the snapshot and the test reaches
    // `check_liquidity`'s threshold comparison.
    test_env.write_snapshot_now(&asset_address, 5_i128, 1_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::InsufficientLiquidity),
        "YieldBlox-style thin liquidity must be blocked by Layer 2"
    );
}

/// Healthy market ($50k volume, 10 unique trades) clears every guardrail
/// including Layer 2. Pins the happy path: `check_liquidity` does not
/// over-reject when the snapshot is fresh and above threshold.
#[test]
fn test_check_liquidity_passes_with_sufficient_volume() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert!(
        result.is_ok(),
        "healthy snapshot must pass check_liquidity, got {:?}",
        result
    );
}

/// Snapshot stamped well outside `max_snapshot_age_seconds` (default 300s)
/// is rejected with `StaleSnapshot`. Verifies that consumer-side freshness
/// enforcement works even when the registry happily stored the (future-from-
/// its-perspective) write at the time it landed.
#[test]
fn test_check_liquidity_blocks_stale_snapshot() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);

    // Baseline ledger timestamp is 100_000 (TestEnv::new). Stamp the snapshot
    // 1h before that — well past the default 300s freshness window.
    let stale_ts = test_env.env.ledger().timestamp().saturating_sub(3_600);
    test_env.write_snapshot(
        &asset_address,
        TestEnv::HEALTHY_VOLUME_USD,
        10_u32,
        stale_ts,
    );

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::StaleSnapshot),
        "1-hour-old snapshot must be rejected by max_snapshot_age_seconds"
    );
}

/// No snapshot has been written for this asset → `get_snapshot` returns
/// `None` → fail-safe to `InsufficientLiquidity`. This is the conservative
/// answer per spec §3 Layer 2: absence of evidence is treated as evidence
/// of absence so that a forgotten attester pipeline cannot silently bypass
/// the guardrail.
#[test]
fn test_check_liquidity_blocks_missing_snapshot() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);
    // No write_snapshot — registry returns None.

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::InsufficientLiquidity),
        "missing snapshot must fail-safe to InsufficientLiquidity"
    );
}

/// `Asset::Other(symbol)` represents off-chain assets (BTC, ETH on CEX
/// rails). They have no SDEX order book, so `check_liquidity` skips without
/// a registry lookup. Cross-source (Layer 1) is the relevant defense for
/// these assets. The skip path also keeps the registry placeholder address
/// from being dereferenced for Symbol-keyed assets.
#[test]
fn test_check_liquidity_skips_for_asset_other() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    test_env.prime_layer1(&asset);
    // Intentionally no snapshot — skip path means the registry is never asked.

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert!(
        result.is_ok(),
        "Asset::Other must skip check_liquidity, got {:?}",
        result
    );
}
