//! YieldBlox / Blend V2 — Stellar, 22 February 2026 (~$10.2M).
//!
//! Stellar-native, 1:1 replay. USTRY/USDC was a dead SDEX market; a single
//! trade drove Reflector's VWAP from ~$1.00 to ~$106 (100x), and the attacker
//! borrowed against the inflated collateral. This is the founding attack the
//! whole library is built to stop.
//!
//! Source: see `data/yieldblox_prices.json` (Halborn / protos / QuillAudits).

use safe_oracle::{OracleSafetyViolation, SafeOracleConfig};
use test_utils::TestEnv;

use crate::data_loader::{self, set_series_in_window};

/// The headline result: with the **default** trustless config (Layer 1 only,
/// no opt-in required), the 100x VWAP spike is rejected by the deviation
/// guardrail before any borrow can proceed.
#[test]
fn replay_yieldblox_default_config_rejects() {
    let data = data_loader::load_yieldblox();
    assert!(data.stellar_native, "YieldBlox is a Stellar-native replay");
    assert_eq!(data.expected_guardrail, "ExcessiveDeviation");

    let env = TestEnv::new();
    let (_addr, asset) = data_loader::fresh_stellar_asset(&env);
    set_series_in_window(&env, &asset, &data);

    // SafeOracleConfig::default() == trustless Layer 1, layer2_enabled = false.
    let result = env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "100x USTRY spike ({} -> {}) must be rejected by the Layer 1 deviation \
         guardrail under the default config",
        data.pre_attack_price(),
        data.final_price(),
    );
}

/// Defense-in-depth: the realistic post-mortem variant. An attacker who has
/// read the YieldBlox writeup keeps the spike *under* Layer 1's 20% deviation
/// threshold (here 5%) but exploits the same root cause — a drained order book
/// where the next trade moves price arbitrarily. Layer 1 sees nothing; Layer 2
/// (opt-in) reads the registry-attested 30-minute volume and rejects with
/// `InsufficientLiquidity`.
///
/// This is the honest companion to the headline test: it documents that the
/// *default* (Layer 1 only) would NOT catch a sophisticated sub-threshold
/// variant, and that the thin-liquidity precondition — the real reason
/// YieldBlox was exploitable — is what Layer 2 closes.
#[test]
fn replay_yieldblox_subthreshold_needs_layer2() {
    let env = TestEnv::new();
    let (addr, asset) = data_loader::fresh_stellar_asset(&env);

    // 5% move: above retail noise, below the default 2000 BPS (20%) threshold.
    let pre = TestEnv::ONE_DOLLAR;
    let nudged = TestEnv::ONE_DOLLAR + TestEnv::ONE_DOLLAR / 20;
    env.set_oracle_price(&asset, pre, 99_900);
    env.set_oracle_price(&asset, nudged, 99_950);

    // Drained book: the smallest valid attestation (the registry rejects
    // volume <= 0), faithfully modelling YieldBlox's near-empty market.
    env.write_snapshot_now(&addr, 5_i128, 10_u32);

    // Layer 1 alone (default) lets the sub-threshold move through.
    let layer1 = env.lastprice(&asset, &SafeOracleConfig::default());
    assert!(
        layer1.is_ok(),
        "a 5% move is below the default deviation threshold — Layer 1 alone \
         does not catch the sophisticated variant (this is the honest gap)"
    );

    // Layer 2 (opt-in) catches the drained-market precondition.
    let layer2 = env.lastprice(&asset, &TestEnv::layer2_config());
    assert_eq!(
        layer2,
        Err(OracleSafetyViolation::InsufficientLiquidity),
        "the thin-liquidity precondition that made YieldBlox possible is \
         caught by the opt-in Layer 2 liquidity guardrail"
    );
}
