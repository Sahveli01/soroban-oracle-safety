//! Loads real, publicly-sourced attack price series from the JSON evidence
//! files in `data/` and replays them through the live `safe_oracle::lastprice`
//! path via `test_utils::TestEnv`.
//!
//! # Provenance contract
//!
//! Every `price_i128` in the JSON files comes from a cited public source
//! (block explorers, post-mortems — see each file's `sources` array). No
//! price is invented. Non-Stellar attacks carry `adapted: true` and replay
//! the real foreign-chain prices through Soroban's `PriceData` / `Asset`
//! types — the deviation guardrail compares consecutive oracle ticks and is
//! chain-agnostic.
//!
//! # Timestamp adaptation (documented)
//!
//! The harness uses `TestEnv`'s synthetic ledger clock (`now = 100_000`), so
//! absolute real-world unix timestamps (which would all read as decades-stale)
//! cannot be used directly. The `ExcessiveDeviation` guardrail is
//! *time-invariant* — it depends only on the ratio between consecutive prices
//! — so [`set_series_in_window`] assigns synthetic in-window ledger timestamps
//! while preserving the real price magnitudes and their order. Each attack's
//! real date is recorded in the JSON `date` field for the audit trail.

#![allow(dead_code)] // Several JSON fields are carried for the docs/audit trail.

use serde::Deserialize;
use soroban_sdk::Address;
use test_utils::TestEnv;

use safe_oracle::Asset;

/// One sourced price point from an attack's `price_series`.
#[derive(Deserialize, Debug, Clone)]
pub struct PricePoint {
    /// Price in Reflector's 14-decimal i128 precision (see `ONE_DOLLAR = 1e14`).
    pub price_i128: i128,
    /// Short machine label, e.g. `pre_attack` / `manipulated`.
    pub label: String,
    /// Human note describing the source/meaning of this point.
    pub note: String,
}

/// A single attack's full sourced record, deserialized from `data/*.json`.
#[derive(Deserialize, Debug, Clone)]
pub struct AttackData {
    pub attack_name: String,
    pub date: String,
    pub chain: String,
    pub stellar_native: bool,
    pub adapted: bool,
    pub asset_pair: String,
    /// Headline loss as widely reported (may include at-risk / theoretical).
    pub loss_usd_headline: u64,
    /// Loss attributable to *oracle manipulation* that safe-oracle is in
    /// scope to defend. `0` for non-oracle attacks (e.g. Euler).
    pub loss_usd_in_scope: u64,
    /// Whether this was an oracle-price manipulation at all.
    pub oracle_manipulation: bool,
    pub sources: Vec<String>,
    pub summary: String,
    pub price_series: Vec<PricePoint>,
    /// `REJECTED` or `PASS` — the empirically-expected default-config outcome.
    pub expected_outcome: String,
    /// The guardrail expected to fire (or "None" for out-of-scope).
    pub expected_guardrail: String,
    pub config_mode: String,
    #[serde(default)]
    pub notes: String,
}

impl AttackData {
    /// The pre-manipulation reference price (first series point).
    pub fn pre_attack_price(&self) -> i128 {
        self.price_series
            .first()
            .expect("series must have a pre-attack point")
            .price_i128
    }

    /// The manipulated / final price (last series point).
    pub fn final_price(&self) -> i128 {
        self.price_series
            .last()
            .expect("series must have a final point")
            .price_i128
    }
}

fn parse(json: &str) -> AttackData {
    serde_json::from_str(json).expect("attack JSON must deserialize")
}

pub fn load_yieldblox() -> AttackData {
    parse(include_str!("data/yieldblox_prices.json"))
}

pub fn load_mango() -> AttackData {
    parse(include_str!("data/mango_prices.json"))
}

pub fn load_euler() -> AttackData {
    parse(include_str!("data/euler_prices.json"))
}

pub fn load_bonqdao() -> AttackData {
    parse(include_str!("data/bonqdao_prices.json"))
}

/// Replays an attack's price series into the primary Reflector mock, assigning
/// synthetic in-window ledger timestamps (the deviation guardrail is
/// time-invariant — see the module docs).
///
/// Timestamps are spaced 50s apart and end at 99_950 (50s before the
/// `TestEnv` baseline `now = 100_000`), so the newest two records both clear
/// the default staleness gates (`max_staleness = 300s`,
/// `previous_max_staleness = 900s`). The two newest records — the
/// pre-attack reference and the manipulated price — are the pair the
/// `ExcessiveDeviation` guardrail compares.
pub fn set_series_in_window(env: &TestEnv, asset: &Asset, data: &AttackData) {
    let n = data.price_series.len() as u64;
    for (i, point) in data.price_series.iter().enumerate() {
        let ts = 99_950 - (n - 1 - i as u64) * 50;
        env.set_oracle_price(asset, point.price_i128, ts);
    }
}

/// Convenience: generate a fresh `Asset::Stellar` for an attack replay.
pub fn fresh_stellar_asset(env: &TestEnv) -> (Address, Asset) {
    use soroban_sdk::testutils::Address as _;
    let addr = Address::generate(&env.env);
    let asset = Asset::Stellar(addr.clone());
    (addr, asset)
}
