//! Unit tests for `oracle-validator`.
//!
//! A minimal mock oracle exposes the two methods safe-oracle's primary path
//! calls — `prices(asset, records)` and `decimals()` — so the validator's
//! ValidationResult mapping is exercised across approve/reject outcomes
//! without a live oracle.

use super::*;
use safe_oracle::PriceData;
use soroban_sdk::{contract, contractimpl, contracttype, testutils::Ledger, vec, Env, Symbol, Vec};

#[contracttype]
enum MKey {
    Hist(Asset),
}

/// Mock with correct 14-dp precision.
#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn seed(env: Env, asset: Asset, prices: Vec<PriceData>) {
        env.storage().persistent().set(&MKey::Hist(asset), &prices);
    }
    pub fn prices(env: Env, asset: Asset, _records: u32) -> Option<Vec<PriceData>> {
        env.storage().persistent().get(&MKey::Hist(asset))
    }
    pub fn decimals(_env: Env) -> u32 {
        14
    }
}

/// Mock that publishes a wrong precision (7) → should trip UnexpectedDecimals.
#[contract]
pub struct MockOracleBadDecimals;

#[contractimpl]
impl MockOracleBadDecimals {
    pub fn seed(env: Env, asset: Asset, prices: Vec<PriceData>) {
        env.storage().persistent().set(&MKey::Hist(asset), &prices);
    }
    pub fn prices(env: Env, asset: Asset, _records: u32) -> Option<Vec<PriceData>> {
        env.storage().persistent().get(&MKey::Hist(asset))
    }
    pub fn decimals(_env: Env) -> u32 {
        7
    }
}

fn btc(env: &Env) -> Asset {
    Asset::Other(Symbol::new(env, "BTC"))
}

fn pd(price: i128, timestamp: u64) -> PriceData {
    PriceData { price, timestamp }
}

#[test]
fn test_approved_when_all_guardrails_pass() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let oracle = env.register(MockOracle, ());
    let validator = env.register(OracleValidator, ());
    let mo = MockOracleClient::new(&env, &oracle);
    let v = OracleValidatorClient::new(&env, &validator);

    let asset = btc(&env);
    // newest-first; 0.5% move, both fresh (ledger=1000, ts 950/900).
    mo.seed(&asset, &vec![&env, pd(100_500, 950), pd(100_000, 900)]);

    let r = v.validate(&oracle, &asset);
    assert!(r.approved, "expected APPROVED, got {:?}", r);
    assert_eq!(r.price, 100_500);
    assert_eq!(r.timestamp, 950);
    assert_eq!(r.violation, 0);
}

#[test]
fn test_rejected_excessive_deviation() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let oracle = env.register(MockOracle, ());
    let validator = env.register(OracleValidator, ());
    let mo = MockOracleClient::new(&env, &oracle);
    let v = OracleValidatorClient::new(&env, &validator);

    let asset = btc(&env);
    // 200% jump (100k → 300k) = 20_000 bps ≫ default 2000 bps.
    mo.seed(&asset, &vec![&env, pd(300_000, 950), pd(100_000, 900)]);

    let r = v.validate(&oracle, &asset);
    assert!(!r.approved);
    assert_eq!(r.violation, 1, "expected ExcessiveDeviation (1)");
    assert_eq!(r.price, 0);
}

#[test]
fn test_rejected_stale_current_price() {
    let env = Env::default();
    // Far-future ledger → current price (ts 950) is ancient (> 300s).
    env.ledger().set_timestamp(1_000_000);
    let oracle = env.register(MockOracle, ());
    let validator = env.register(OracleValidator, ());
    let mo = MockOracleClient::new(&env, &oracle);
    let v = OracleValidatorClient::new(&env, &validator);

    let asset = btc(&env);
    // Small 0.5% deviation so the chain reaches the staleness check.
    mo.seed(&asset, &vec![&env, pd(100_500, 950), pd(100_000, 900)]);

    let r = v.validate(&oracle, &asset);
    assert!(!r.approved);
    assert_eq!(r.violation, 2, "expected StaleData (2)");
}

#[test]
fn test_rejected_when_no_price_history() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let oracle = env.register(MockOracle, ());
    let validator = env.register(OracleValidator, ());
    let v = OracleValidatorClient::new(&env, &validator);

    // No seed → prices() returns None → StaleData (fewer than 2 records).
    let r = v.validate(&oracle, &btc(&env));
    assert!(!r.approved);
    assert_eq!(r.violation, 2, "expected StaleData (2)");
}

#[test]
fn test_rejected_unexpected_decimals() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let oracle = env.register(MockOracleBadDecimals, ());
    let validator = env.register(OracleValidator, ());
    let mo = MockOracleBadDecimalsClient::new(&env, &oracle);
    let v = OracleValidatorClient::new(&env, &validator);

    let asset = btc(&env);
    mo.seed(&asset, &vec![&env, pd(100_500, 950), pd(100_000, 900)]);

    let r = v.validate(&oracle, &asset);
    assert!(!r.approved);
    assert_eq!(r.violation, 10, "expected UnexpectedDecimals (10)");
}
