//! BonqDAO — Polygon, 1-2 February 2023 (~$120M at-risk). ADAPTED to Soroban.
//!
//! The attacker staked the minimum 10 TRB to become a Tellor reporter and
//! `submitValue`'d a massively inflated WALBT price (BonqDAO used instantaneous
//! Tellor updates), minted ~100M BEUR against 0.1 WALBT, then reported a
//! near-zero price to liquidate other troves. The pre-attack WALBT price is
//! real (~$0.05); the manipulated price is a conservative lower-bound stand-in
//! (the on-chain value was far larger). Any move >20% trips the guardrail, so
//! the verdict is robust to the exact figure.
//!
//! Source: see `data/bonqdao_prices.json` (Halborn / Hacken / Immunefi).

use safe_oracle::{OracleSafetyViolation, SafeOracleConfig};
use test_utils::TestEnv;

use crate::data_loader::{self, set_series_in_window};

#[test]
fn replay_bonqdao_default_config_rejects() {
    let data = data_loader::load_bonqdao();
    assert!(
        data.adapted,
        "BonqDAO is a Polygon attack adapted to Soroban"
    );
    assert_eq!(data.expected_guardrail, "ExcessiveDeviation");

    let env = TestEnv::new();
    let (_addr, asset) = data_loader::fresh_stellar_asset(&env);
    set_series_in_window(&env, &asset, &data);

    let result = env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "Tellor WALBT inflation ({} -> {}) must be rejected by the Layer 1 \
         deviation guardrail under the default config",
        data.pre_attack_price(),
        data.final_price(),
    );
}

/// Defense-in-depth note: the BonqDAO market was also extremely thin (the
/// attacker moved price with 0.1 WALBT), so even a hypothetical sub-threshold
/// Tellor nudge would be caught by the opt-in Layer 2 liquidity guardrail —
/// the same structural defense as the YieldBlox sophisticated variant.
#[test]
fn replay_bonqdao_thin_market_layer2_backstop() {
    let env = TestEnv::new();
    let (addr, asset) = data_loader::fresh_stellar_asset(&env);

    // Sub-threshold price nudge (within Layer 1's tolerance)...
    env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    env.set_oracle_price(
        &asset,
        TestEnv::ONE_DOLLAR + TestEnv::ONE_DOLLAR / 25,
        99_950,
    );
    // ...against the same drained-book precondition.
    env.write_snapshot_now(&addr, 5_i128, 10_u32);

    let result = env.lastprice(&asset, &TestEnv::layer2_config());
    assert_eq!(
        result,
        Err(OracleSafetyViolation::InsufficientLiquidity),
        "a thin-market Tellor-style nudge is backstopped by the opt-in Layer 2 \
         liquidity guardrail"
    );
}
