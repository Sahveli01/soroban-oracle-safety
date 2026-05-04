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

/// 7-decimal USD volume that comfortably clears the $10,000 default
/// `min_liquidity_usd` for Phase 4.5 Layer 2 happy-path tests.
const HEALTHY_VOLUME_USD: i128 = 500_000_000_000;

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
        assert_eq!(stored_registry, test_env.registry);
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

// ===== Phase 4.5: Layer 2 Integration via borrow() =====
//
// These tests exercise `MockLending::borrow()` against `Asset::Stellar`
// (rather than `Asset::Other`), forcing the lending → safe_oracle →
// LiquidityRegistry cross-contract path. Each scenario mirrors one in
// `crates/safe-oracle/tests/e2e_attack_scenarios.rs` from the lending
// protocol's perspective: the same attack vectors surface the same
// `MockLendingError` variants the integrator's caller will observe in
// production. Together they prove transparent passthrough — the lending
// contract never collapses oracle violations into a single bucket.

/// Phase 4.5 happy path: every Layer 1 + Layer 2 guardrail clears, so
/// `borrow()` returns `Ok(())`. This is the lending-contract-perspective
/// counterpart of `e2e_attack_scenarios::scenario_1`. Pins that wiring
/// `MockLending` to a real `LiquidityRegistry` (Phase 4.5 TestEnv update)
/// does not introduce a false-positive rejection in the normal flow.
#[test]
fn test_borrow_happy_path_passes_all_guardrails() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_950);
    test_env.write_snapshot_now(&asset_address, HEALTHY_VOLUME_USD, 10_u32);

    let user = Address::generate(&test_env.env);
    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    assert_eq!(
        result,
        Ok(Ok(())),
        "borrow with healthy oracle and registry conditions must succeed"
    );
}

/// Phase 4.5 — YieldBlox classic (100× spike). Mirrors
/// `test_borrow_fails_when_oracle_deviation_excessive` but goes through the
/// `Asset::Stellar` path, so the registry is *consulted* (no longer
/// short-circuited by the `Asset::Other → Ok(None)` helper). Layer 1 still
/// short-circuits before Layer 2 runs, so `ExcessiveDeviation` propagates —
/// this proves Layer 1 ordering survives the registry-active path too.
#[test]
fn test_borrow_blocks_yieldblox_classic_via_layer1() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, ONE_DOLLAR * 100, 99_950);

    // Healthy snapshot proves Layer 1 surfaces before Layer 2 is consulted.
    test_env.write_snapshot_now(&asset_address, HEALTHY_VOLUME_USD, 10_u32);

    let user = Address::generate(&test_env.env);
    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    assert_eq!(
        result,
        Err(Ok(MockLendingError::ExcessiveDeviation)),
        "100x price spike must block borrow via Layer 1 deviation check"
    );
}

/// Phase 4.5 — Sophisticated YieldBlox (sub-threshold spike + thin SDEX).
/// **The library's headline value proposition from the lending protocol
/// perspective.** A 5% spike clears Layer 1's 20% deviation tolerance, so
/// without Layer 2 the lending contract would accept the manipulated price
/// and hand out an over-borrow. With `check_liquidity` reading the
/// registry-attested 30-minute volume, `borrow()` returns
/// `InsufficientLiquidity` instead — the same discriminant a production
/// integrator would observe and surface to its caller.
#[test]
fn test_borrow_blocks_yieldblox_sophisticated_via_layer2() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    let previous_price = ONE_DOLLAR;
    let current_price = ONE_DOLLAR + (ONE_DOLLAR / 20); // 5% spike
    test_env.set_oracle_price(&asset, previous_price, 99_900);
    test_env.set_oracle_price(&asset, current_price, 99_950);

    // Drained order book: $0.0000005 of 30-minute volume.
    test_env.write_snapshot_now(&asset_address, 5_i128, 10_u32);

    let user = Address::generate(&test_env.env);
    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    assert_eq!(
        result,
        Err(Ok(MockLendingError::InsufficientLiquidity)),
        "sophisticated 5%-spike attack on a thin orderbook must block borrow via Layer 2"
    );
}

/// Phase 4.5 — Stale Reflector. Like `test_borrow_fails_when_oracle_data_stale`
/// but on the `Asset::Stellar` path so the registry is live in the call
/// chain. Pins that staleness still surfaces *before* the registry call
/// even when a fresh snapshot is available — Layer 1 ordering does not
/// degrade when Layer 2 is wired.
#[test]
fn test_borrow_blocks_stale_oracle_on_stellar_asset() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.write_snapshot_now(&asset_address, HEALTHY_VOLUME_USD, 10_u32);

    let now = test_env.env.ledger().timestamp();
    let stale_ts = now.saturating_sub(3600);
    test_env.set_oracle_price(&asset, ONE_DOLLAR, stale_ts.saturating_sub(100));
    test_env.set_oracle_price(&asset, ONE_DOLLAR, stale_ts);

    let user = Address::generate(&test_env.env);
    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    assert_eq!(
        result,
        Err(Ok(MockLendingError::StaleData)),
        "1-hour-old Reflector data must block borrow"
    );
}

/// Phase 4.5 — Stale registry snapshot. Reflector is fresh, but the
/// attestation pipeline (`oracle-watch`) has stalled and the registry's
/// snapshot is 1 hour old. Layer 2's freshness check (consumer-side, against
/// `config.max_snapshot_age_seconds`) surfaces `StaleSnapshot`. This is the
/// failure mode that pages an integrator's on-call when the off-chain
/// attestation infrastructure breaks — operationally distinct from
/// `StaleData` (Reflector outage) so dashboards can route them differently.
#[test]
fn test_borrow_blocks_stale_snapshot() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_950);

    let now = test_env.env.ledger().timestamp();
    let stale_ts = now.saturating_sub(3600);
    test_env.write_snapshot(&asset_address, HEALTHY_VOLUME_USD, 10_u32, stale_ts);

    let user = Address::generate(&test_env.env);
    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    assert_eq!(
        result,
        Err(Ok(MockLendingError::StaleSnapshot)),
        "1-hour-old registry snapshot must block borrow (default 5-minute threshold)"
    );
}
