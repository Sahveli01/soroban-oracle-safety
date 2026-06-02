//! Euler Finance — Ethereum, 13 March 2023 ($197M). HONEST NEGATIVE CONTROL.
//!
//! Euler was the largest hack of 2023, but it was **not an oracle-price
//! manipulation**. The root cause was `donateToReserves()` lacking a solvency
//! check — it burned eTokens but not dTokens, creating an artificial
//! under-collateralized position the attacker self-liquidated at a discount.
//! The price feed reported legitimate prices throughout.
//!
//! safe-oracle validates oracle prices. It therefore (correctly) does NOT
//! fire on Euler's honest feed, and would NOT have prevented this attack.
//! This test exists to prove two things at once:
//!   1. safe-oracle does not false-positive on a non-oracle attack, and
//!   2. we report what the defaults do NOT catch instead of hiding it.
//!
//! `loss_usd_in_scope` is 0 in the JSON on purpose — Euler's $197M is
//! deliberately excluded from any "prevented" total.
//!
//! Source: see `data/euler_prices.json` (Chainalysis / Hacken / BlockSec).

use safe_oracle::SafeOracleConfig;
use test_utils::TestEnv;

use crate::data_loader::{self, set_series_in_window};

#[test]
fn replay_euler_out_of_scope_no_price_anomaly() {
    let data = data_loader::load_euler();
    assert!(
        !data.oracle_manipulation,
        "Euler was NOT an oracle manipulation — this is the scope boundary"
    );
    assert_eq!(
        data.loss_usd_in_scope, 0,
        "Euler is excluded from prevented totals"
    );
    assert_eq!(data.expected_outcome, "PASS");

    let env = TestEnv::new();
    let (_addr, asset) = data_loader::fresh_stellar_asset(&env);
    set_series_in_window(&env, &asset, &data);

    // The feed was honest, so safe-oracle returns Ok — there was no price
    // signal to act on. This is the correct (and honest) result: a price
    // validator cannot stop a collateral-accounting / liquidation-logic bug.
    let result = env.lastprice(&asset, &SafeOracleConfig::default());
    assert!(
        result.is_ok(),
        "Euler's feed was legitimate; safe-oracle correctly does not \
         false-positive — and could not have prevented a non-oracle attack"
    );
    assert_eq!(
        result.unwrap().price,
        data.final_price(),
        "the honest, unmanipulated price passes through unchanged"
    );
}
