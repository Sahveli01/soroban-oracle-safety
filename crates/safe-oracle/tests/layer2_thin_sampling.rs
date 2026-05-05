//! Layer 2 Guardrail #5 — Thin Sampling (`check_thin_sampling`).
//!
//! Symmetric to `layer2_liquidity.rs`: same cross-contract path
//! (`lastprice` → `get_validated_snapshot` → `LiquidityRegistry::get_snapshot`),
//! same Layer 1 priming so each scenario isolates a single Layer 2 outcome.
//! The asymmetry is the threshold: this file pins behavior on
//! `unique_trades_1h` against `config.min_trade_count_1h` rather than on
//! `volume_30m_usd`.
//!
//! The Layer-2-order test (`liquidity_before_thin_sampling`) lives here
//! rather than in `layer2_liquidity.rs` because the assertion is *about*
//! the relationship between the two guardrails — pairing it with the
//! later-introduced check keeps the file with the new behavior also
//! holding the joint contract.

use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::{testutils::Address as _, Address, Symbol};
use test_utils::TestEnv;

/// YieldBlox-replica from the Layer 2 sampling angle: the attacker's
/// manipulated trade was effectively the *only* trade in the pricing
/// window. Volume in this fixture clears `min_liquidity_usd`, so the
/// `InsufficientLiquidity` guardrail is satisfied — `check_thin_sampling`
/// is the one that catches the attack shape. Pinning this here keeps
/// thin-sampling's defense-in-depth role independent from
/// `check_liquidity`'s threshold path.
#[test]
fn test_check_thin_sampling_blocks_yieldblox_single_trade() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);

    // Healthy volume but only the attacker's single trade in the past hour.
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 1_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ThinSampling),
        "single-trade window must be blocked even with healthy volume"
    );
}

/// Active market (10 trades in the past hour, comfortably above the default
/// 5-trade threshold) clears Layer 2. Pins the happy path: the guardrail
/// does not over-reject when both threshold fields are above their
/// configured minima.
#[test]
fn test_check_thin_sampling_passes_with_active_market() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert!(
        result.is_ok(),
        "active market must pass thin sampling, got {:?}",
        result
    );
}

/// Boundary semantics: `unique_trades_1h == config.min_trade_count_1h`
/// passes. The check uses strict `<` (not `<=`), matching the convention of
/// the deviation guardrail and the `volume_30m_usd < min_liquidity_usd`
/// check in `check_liquidity`. Locking the boundary stops a future
/// "tighten by one" change from silently rejecting attestations that were
/// previously valid.
#[test]
fn test_check_thin_sampling_passes_at_threshold() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);

    // Default `min_trade_count_1h` is 5; pass exactly that.
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 5_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert!(
        result.is_ok(),
        "trade count == threshold must pass under strict `<`, got {:?}",
        result
    );
}

/// `Asset::Other(symbol)` skips Layer 2 entirely (helper returns
/// `Ok(None)` and `lastprice` short-circuits past both guardrails). Mirrors
/// `test_check_liquidity_skips_for_asset_other` to confirm the helper's
/// skip semantics apply to *both* guardrails, not just the first one
/// reached in `lastprice`'s old per-guardrail call site.
#[test]
fn test_check_thin_sampling_skips_for_asset_other() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    test_env.prime_layer1(&asset);
    // No snapshot — skip path means the registry is never consulted.

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert!(
        result.is_ok(),
        "Asset::Other must skip thin sampling, got {:?}",
        result
    );
}

/// Layer 2 execution order: when both guardrails would fail, `lastprice`
/// returns `InsufficientLiquidity` — `check_liquidity` runs before
/// `check_thin_sampling`. This pins the order so a future refactor can't
/// silently swap them without an explicit decision (the order matters for
/// audit/error-reporting: liquidity failure is the structural signal,
/// thin sampling is the secondary one).
#[test]
fn test_layer2_check_order_liquidity_before_thin_sampling() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);

    // Both thresholds violated.
    test_env.write_snapshot_now(&asset_address, 5_i128, 1_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::InsufficientLiquidity),
        "lastprice must run check_liquidity before check_thin_sampling"
    );
}
