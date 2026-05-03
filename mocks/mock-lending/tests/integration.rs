//! Integration tests for `mock-lending`.
//!
//! These tests use the `test-utils` crate (which itself depends on
//! `mock-lending`), so they live in the integration test directory rather
//! than `lib.rs`'s `mod test`. Inline unit tests would force `mock-lending`
//! to be compiled twice (once as a normal dep of `test-utils`, once as a
//! test target) and Rust would treat the two builds as different crates —
//! every shared type would mismatch (the cycle bites at error type
//! comparison via `try_borrow`). Integration tests in `tests/` see
//! `mock-lending` as a single normal dependency, which matches `test-utils`'
//! view — types unify, the cycle disappears.

use mock_lending::{DataKey, MockLendingError};
use safe_oracle::{Asset, SafeOracleConfig};
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    Address, Symbol,
};
use test_utils::TestEnv;

/// Reinitialization protection: `TestEnv::new()` already initializes the
/// lending contract (Phase 2.7 wiring). A second `initialize` call from any
/// caller — even with a different admin — must be rejected with
/// `AlreadyInitialized` rather than silently overwriting oracle/registry/config
/// addresses. Mirrors the LiquidityRegistry reinit guard added in Phase 3.1.
#[test]
fn test_initialize_prevents_reinitialization() {
    let test_env = TestEnv::new();
    let attacker_admin = Address::generate(&test_env.env);

    let result = test_env.lending_client.try_initialize(
        &attacker_admin,
        &test_env.reflector_address,
        &test_env.lending_address,
        &SafeOracleConfig::default(),
    );

    assert_eq!(
        result,
        Err(Ok(MockLendingError::AlreadyInitialized)),
        "second initialize must be rejected to prevent admin override"
    );
}

/// 14-decimal helper: dollars → Reflector-scale price (×10^14).
const ONE_DOLLAR: i128 = 100_000_000_000_000;

/// `TestEnv::new()` Phase 2.7'de lending'i initialize ediyor; storage'da
/// beklenen alanların yazılı olduğunu doğrula.
#[test]
fn test_initialize_sets_storage() {
    let test_env = TestEnv::new();

    test_env.env.as_contract(&test_env.lending_address, || {
        let stored_oracle: Address = test_env
            .env
            .storage()
            .instance()
            .get(&DataKey::Oracle)
            .unwrap();
        let stored_registry: Address = test_env
            .env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap();
        let _stored_config: SafeOracleConfig = test_env
            .env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .unwrap();
        let _stored_admin: Address = test_env
            .env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap();

        assert_eq!(stored_oracle, test_env.reflector_address);
        // registry placeholder == lending_address until Phase 4 wires up
        // a real `LiquidityRegistry`.
        assert_eq!(stored_registry, test_env.lending_address);
    });
}

#[test]
fn test_deposit_records_amount() {
    let test_env = TestEnv::new();
    let caller = Address::generate(&test_env.env);
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.lending_client.deposit(&caller, &asset, &100);
    test_env.lending_client.deposit(&caller, &asset, &50);

    test_env.env.as_contract(&test_env.lending_address, || {
        let total: i128 = test_env
            .env
            .storage()
            .persistent()
            .get(&DataKey::Deposit(caller.clone(), asset.clone()))
            .unwrap();
        assert_eq!(total, 150);
    });
}

#[test]
fn test_borrow_emits_event() {
    let test_env = TestEnv::new();
    let caller = Address::generate(&test_env.env);
    let asset = Asset::Other(Symbol::new(&test_env.env, "XLM"));

    // Inject valid oracle data so Layer 1 lets the borrow through.
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_950);

    test_env.lending_client.borrow(&caller, &asset, &1000);

    let events = test_env.env.events().all();
    assert_eq!(events.events().len(), 1);
}

#[test]
fn test_borrow_succeeds_with_valid_oracle_data() {
    let test_env = TestEnv::new();
    let user = Address::generate(&test_env.env);
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_950);

    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_000);
    assert!(
        result.is_ok(),
        "expected Ok with valid oracle data, got {:?}",
        result
    );
}

/// YieldBlox-class attack: $1.05 → $106 SDEX pump. `safe_oracle::lastprice`
/// deviation guardrail blocks the borrow and `MockLendingError::ExcessiveDeviation`
/// propagates transparently — the pitch demo (("bu test pass olunca proje çalışıyor")).
#[test]
fn test_borrow_fails_when_oracle_deviation_excessive() {
    let test_env = TestEnv::new();
    let user = Address::generate(&test_env.env);
    let asset = Asset::Other(Symbol::new(&test_env.env, "USTRY"));

    test_env.set_oracle_price(&asset, 105_000_000_000_000, 99_900); // $1.05
    test_env.set_oracle_price(&asset, 10_600_000_000_000_000, 99_950); // $106

    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_000);
    assert_eq!(
        result,
        Err(Ok(MockLendingError::ExcessiveDeviation)),
        "YieldBlox-class attack must be blocked by deviation guardrail"
    );
}

/// Stale oracle (~49 700s eski, default max_staleness_seconds=300) →
/// staleness guardrail blocks the borrow; `MockLendingError::StaleData`
/// propagates transparently.
#[test]
fn test_borrow_fails_when_oracle_data_stale() {
    let test_env = TestEnv::new();
    let user = Address::generate(&test_env.env);
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    // baseline=100_000; ts=50_000/50_300 → ~49_700s elapsed (default max=300)
    // prices identical → 0 BPS deviation, so deviation guardrail passes
    // and the pipeline actually reaches staleness.
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 50_000);
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 50_300);

    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_000);
    assert_eq!(
        result,
        Err(Ok(MockLendingError::StaleData)),
        "stale oracle data must block borrow"
    );
}
