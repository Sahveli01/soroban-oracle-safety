//! Layer 1 combined-behavior integration tests.
//!
//! This file verifies that the full Layer 1 guardrail chain in
//! `safe_oracle::lastprice` runs together and in the correct execution order.
//! Individual guardrail tests (deviation, staleness, cross-source — 6/4/6
//! scenarios respectively) live in `tests/integration.rs`; this file is
//! different: it focuses on Layer 1's *combined* behavior and error
//! precedence.

use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::Symbol;
use test_utils::TestEnv;

/// All Layer 1 guardrails (deviation, staleness, cross-source) pass → Ok.
/// The returned price must reflect the newest entry.
#[test]
fn test_layer1_happy_path_all_guardrails_pass() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
    test_env.set_secondary_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    let mut config = TestEnv::relaxed_config();
    config.secondary_oracle = Some(test_env.secondary_reflector_address.clone());

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok when all guardrails pass, got {:?}",
        result
    );
    let price = result.unwrap();
    assert_eq!(price.price, TestEnv::ONE_DOLLAR);
    assert_eq!(price.timestamp, 99_950);
}

/// When both deviation and current-price staleness would fail,
/// `ExcessiveDeviation` is returned because deviation runs first. A
/// regression that swaps the execution order is caught here by the failing
/// assertion.
#[test]
fn test_layer1_execution_order_deviation_before_staleness() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    // Phase 7.2: keep `previous` within the new previous-staleness gate
    // (default 900s) so that gate doesn't preempt the deviation check.
    //   - now=100_000 (TestEnv default)
    //   - prev_ts=99_500 → 500s old (within 900s prev gate)
    //   - curr_ts=99_650 → 350s old (> 300s current gate, fails staleness)
    // 100% deviation = 10_000 BPS still exceeds strict max=2000.
    test_env.set_oracle_price(&asset, 100 * TestEnv::ONE_DOLLAR, 99_500);
    test_env.set_oracle_price(&asset, 200 * TestEnv::ONE_DOLLAR, 99_650);

    let config = TestEnv::strict_config();

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "deviation check should run before staleness — expected ExcessiveDeviation"
    );
}

/// Deviation passes (small), staleness fails (old) → `StaleData`.
/// Verifies the pipeline actually reaches staleness after deviation.
#[test]
fn test_layer1_execution_order_staleness_after_deviation_pass() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    // ts=50_000/50_300 → ~49_700s elapsed (strict max_staleness=300 → stale)
    // 1% deviation = 100 BPS (strict max=2000 → pass)
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 50_000);
    test_env.set_oracle_price(
        &asset,
        TestEnv::ONE_DOLLAR + TestEnv::ONE_DOLLAR / 100,
        50_300,
    );

    let config = TestEnv::strict_config();

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::StaleData),
        "deviation passed but staleness failed — expected StaleData"
    );
}

/// Deviation + staleness pass, cross-source fails → `CrossSourceMismatch`.
/// Verifies the pipeline actually reaches cross-source after staleness.
#[test]
fn test_layer1_execution_order_cross_source_after_staleness_pass() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    // Fresh (50s elapsed ≤ 300), 0 BPS deviation (≤ 2000)
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
    // Secondary $1.10 = 1000 BPS cross-source delta (strict max=500 → mismatch)
    test_env.set_secondary_oracle_price(&asset, TestEnv::ONE_DOLLAR * 110 / 100, 99_950);

    let mut config = TestEnv::strict_config();
    config.secondary_oracle = Some(test_env.secondary_reflector_address.clone());

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::CrossSourceMismatch),
        "deviation + staleness passed but cross-source failed — expected CrossSourceMismatch"
    );
}

/// Production-realistic scenario with `SafeOracleConfig::default()`:
/// 1% deviation, 150s old, no secondary → all guardrails pass. Verifies
/// the default values accept a reasonable workflow.
#[test]
fn test_layer1_with_default_config_passes_normal_scenario() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    // baseline=100_000; ts=99_750/99_850 → 250/150s elapsed (default max=300 → ok)
    // 1% deviation = 100 BPS (default max=2000 → ok)
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_750);
    test_env.set_oracle_price(
        &asset,
        TestEnv::ONE_DOLLAR + TestEnv::ONE_DOLLAR / 100,
        99_850,
    );

    // Default: secondary_oracle = None → cross-source skip
    let config = SafeOracleConfig::default();

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "default config should pass normal scenario, got {:?}",
        result
    );
}
