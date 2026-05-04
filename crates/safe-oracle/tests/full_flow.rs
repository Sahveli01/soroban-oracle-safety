//! End-to-end flow verification for `lastprice()` (Phase 4.3).
//!
//! Layer 1 (`check_deviation` → `check_staleness` → `check_cross_source`) and
//! Layer 2 (`check_liquidity` → `check_thin_sampling`) each have their own
//! test files pinning *individual* guardrail behavior. This file pins the
//! *order* in which they run, by injecting overlapping failures and asserting
//! which `OracleSafetyViolation` surfaces. Together with the per-guardrail
//! suites these tests freeze the spec §7 fail-fast contract: each violation
//! short-circuits, no later guardrail observes data the earlier one already
//! rejected.
//!
//! Layer-2 internal order (`check_liquidity` before `check_thin_sampling`)
//! is pinned in `layer2_thin_sampling.rs::test_layer2_check_order_liquidity_before_thin_sampling`
//! (Phase 4.2 commit `9879e25`); not duplicated here.
//!
//! No implementation changes — every assertion verifies behavior already
//! shipped in Phases 4.1 and 4.2.

use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::{testutils::Address as _, Address};
use test_utils::TestEnv;

/// 14-decimal Reflector price helper: dollars → ×10^14.
const ONE_DOLLAR: i128 = 100_000_000_000_000;

/// 7-decimal USD volume that comfortably clears the $10,000 default
/// `min_liquidity_usd`, used by tests that need Layer 2 to pass so a Layer 1
/// failure can surface uncontested.
const HEALTHY_VOLUME_USD: i128 = 500_000_000_000;

/// Two same-priced Reflector records make Layer 1 deterministic when a test
/// only wants to exercise a *later* guardrail. Identical pricing keeps
/// deviation at 0 BPS; the 99_950 newest timestamp sits 50s before the
/// `TestEnv` baseline of 100_000 so staleness clears the 300s default.
fn prime_layer1(test_env: &TestEnv, asset: &Asset) {
    test_env.set_oracle_price(asset, ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(asset, ONE_DOLLAR, 99_950);
}

/// Layer 1 internal order: deviation runs before staleness. A scenario that
/// trips *both* must surface `ExcessiveDeviation`. Pinning this stops a future
/// reorder from silently swapping the surfaced error — important for incident
/// triage, where the violation variant is the first signal an integrator sees.
#[test]
fn test_full_flow_deviation_caught_before_staleness() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    // Layer 2 would pass if reached; this test isolates Layer 1 ordering.
    test_env.write_snapshot_now(&asset_address, HEALTHY_VOLUME_USD, 10_u32);

    // Two failures live on the same Reflector reads:
    //   - 9900% jump from $1 → $100  → check_deviation fails (default 2000 BPS)
    //   - newest timestamp 5000s old → check_staleness fails (default 300s)
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 94_950);
    test_env.set_oracle_price(&asset, ONE_DOLLAR * 100, 95_000);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "check_deviation must fire before check_staleness"
    );
}

/// Layer 1 internal order: staleness runs before cross-source. A scenario that
/// trips *both* a 3600s-old timestamp and a 50× secondary disagreement must
/// surface `StaleData`. Cross-source is the most expensive Layer 1 check
/// (extra cross-contract call); pinning the early-exit ordering documents
/// that the cheaper staleness check gates that cost.
#[test]
fn test_full_flow_staleness_caught_before_cross_source() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.write_snapshot_now(&asset_address, HEALTHY_VOLUME_USD, 10_u32);

    // Same price → deviation = 0; both timestamps are 3600s old → staleness fails.
    let old_ts = test_env.env.ledger().timestamp().saturating_sub(3600);
    test_env.set_oracle_price(&asset, ONE_DOLLAR, old_ts.saturating_sub(50));
    test_env.set_oracle_price(&asset, ONE_DOLLAR, old_ts);

    // Secondary disagrees by 50× — cross-source would fire if reached.
    test_env.set_secondary_oracle_price(&asset, ONE_DOLLAR * 50, old_ts);

    let config = SafeOracleConfig {
        secondary_oracle: Some(test_env.secondary_reflector_address.clone()),
        ..SafeOracleConfig::default()
    };

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::StaleData),
        "check_staleness must fire before check_cross_source"
    );
}

/// Layer 1 → Layer 2 boundary: a healthy Reflector signal must reach Layer 2,
/// where a sub-threshold snapshot then surfaces `InsufficientLiquidity`. This
/// proves the full chain is wired — the per-guardrail tests verify each link
/// in isolation, but only an end-to-end scenario with Layer 1 *passing* shows
/// that the Layer 2 call site is actually reached from `lastprice`.
#[test]
fn test_full_flow_layer1_pass_then_layer2_blocks() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    prime_layer1(&test_env, &asset);

    // Volume of 5 (7-decimal) is $0.0000005 — far below the default
    // `min_liquidity_usd` of $10,000.
    test_env.write_snapshot_now(&asset_address, 5_i128, 10_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::InsufficientLiquidity),
        "Layer 1 passes; Layer 2 must catch insufficient liquidity"
    );
}

/// Happy path: every guardrail passes and `lastprice` returns the newest
/// Reflector record verbatim. Pins the return value (price *and* timestamp)
/// so a future change can't silently mutate the data on its way through —
/// the safety layer is meant to be a pass-through when nothing fires.
#[test]
fn test_full_flow_all_guardrails_pass() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    prime_layer1(&test_env, &asset);
    test_env.write_snapshot_now(&asset_address, HEALTHY_VOLUME_USD, 10_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    let price = result.expect("all healthy conditions must pass");
    assert_eq!(
        price.price, ONE_DOLLAR,
        "must return the current Reflector price"
    );
    assert_eq!(
        price.timestamp, 99_950,
        "must return the newest Reflector timestamp from prime_layer1"
    );
}

/// Cross-source skip semantics: `secondary_oracle = None` disables the check
/// entirely, even when a secondary feed is *available* and would disagree.
/// The `set_secondary_oracle_price` mismatch here is the falsifying setup —
/// without the explicit skip, this scenario would surface `CrossSourceMismatch`.
#[test]
fn test_full_flow_cross_source_skipped_without_secondary() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    prime_layer1(&test_env, &asset);
    test_env.write_snapshot_now(&asset_address, HEALTHY_VOLUME_USD, 10_u32);

    // Mismatched secondary: would trip CrossSourceMismatch if cross-source ran.
    test_env.set_secondary_oracle_price(&asset, ONE_DOLLAR * 50, 99_950);

    let config = SafeOracleConfig {
        secondary_oracle: None,
        ..SafeOracleConfig::default()
    };

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "secondary_oracle=None must skip cross-source despite a mismatched secondary, got {:?}",
        result
    );
}
