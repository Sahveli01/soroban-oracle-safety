//! Mango Markets — Solana, 11 October 2022 (~$114M). ADAPTED to Soroban.
//!
//! Avraham Eisenberg pumped the oracle-reported MNGO price >13x (~$0.038 ->
//! ~$0.91) by buying across the three exchanges feeding Mango's oracle, then
//! borrowed ~$114M against the inflated collateral. The price magnitudes are
//! real (CFTC / Chainalysis / CoinDesk); they are replayed through Soroban's
//! `PriceData` / `Asset` types. The deviation guardrail compares consecutive
//! oracle ticks and is chain-agnostic.
//!
//! Source: see `data/mango_prices.json`.

use safe_oracle::{OracleSafetyViolation, SafeOracleConfig};
use test_utils::TestEnv;

use crate::data_loader::{self, set_series_in_window};

#[test]
fn replay_mango_default_config_rejects() {
    let data = data_loader::load_mango();
    assert!(data.adapted, "Mango is a Solana attack adapted to Soroban");
    assert!(data.oracle_manipulation, "Mango was an oracle manipulation");

    let env = TestEnv::new();
    let (_addr, asset) = data_loader::fresh_stellar_asset(&env);
    set_series_in_window(&env, &asset, &data);

    let result = env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "MNGO oracle pump ({} -> {}, >13x) must be rejected by the Layer 1 \
         deviation guardrail under the default config",
        data.pre_attack_price(),
        data.final_price(),
    );
}
