//! Circuit breaker tests.
//!
//! Two layers of coverage:
//!
//! - Phase 5.1 unit tests exercise the state machine in isolation through a
//!   thin `TestHost` harness. Soroban's `instance()` storage is only
//!   accessible from inside a contract context, so the harness registers a
//!   contract whose methods delegate to the public library functions and the
//!   auto-generated client exercises them.
//!
//! - Phase 5.2 integration tests exercise the breaker through the real
//!   `lastprice()` wrapper, using `TestEnv`'s `OracleHost` harness (Pre-5.2.B).
//!   These pin the integrator-facing contract: default config does not
//!   open the breaker on violation; opt-in halts on first failure.

use safe_oracle::circuit_breaker::{
    check_circuit_breaker, close_circuit_breaker, open_circuit_breaker,
};
use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger as _},
    Address, Env, Symbol,
};
use test_utils::TestEnv;

/// 14-decimal Reflector price helper: dollars → ×10^14.
const ONE_DOLLAR: i128 = 100_000_000_000_000;

/// 7-decimal USD volume that comfortably clears the $10,000 default
/// `min_liquidity_usd` so liquidity-passing scenarios isolate the failure
/// they actually want to demonstrate.
const HEALTHY_VOLUME_USD: i128 = 500_000_000_000;

/// Harness contract: hosts the breaker storage and surfaces the three
/// public functions as contract methods so the test client can exercise
/// them inside a real contract invocation context.
#[contract]
struct TestHost;

#[contractimpl]
impl TestHost {
    pub fn run_check(env: Env, asset: Asset) -> Result<(), OracleSafetyViolation> {
        check_circuit_breaker(&env, &asset)
    }

    pub fn run_open(env: Env, asset: Asset, duration: u32) {
        open_circuit_breaker(&env, &asset, duration);
    }

    pub fn run_close(env: Env, asset: Asset) {
        close_circuit_breaker(&env, &asset);
    }
}

fn setup() -> (Env, TestHostClient<'static>) {
    let env = Env::default();
    let id = env.register(TestHost, ());
    let client = TestHostClient::new(&env, &id);
    (env, client)
}

/// Default state for an asset never touched by `open_circuit_breaker`
/// must be `Closed`. `unwrap_or(Closed)` on the `get` is what makes
/// integration ergonomic — no per-asset bootstrap step required.
#[test]
fn test_initial_state_is_closed() {
    let (env, client) = setup();
    let asset = Asset::Stellar(Address::generate(&env));

    let result = client.try_run_check(&asset);
    assert!(
        result.is_ok(),
        "initial state must be Closed (no storage entry yet), got {:?}",
        result
    );
}

/// After `open_circuit_breaker`, `check_circuit_breaker` must return
/// `CircuitBreakerOpen` rather than running the guardrail chain. The
/// halt window is well in the future here so auto-recovery does not
/// fire — that path is exercised by the next test.
#[test]
fn test_open_then_check_returns_circuit_breaker_open() {
    let (env, client) = setup();
    let asset = Asset::Stellar(Address::generate(&env));

    client.run_open(&asset, &720);

    let result = client.try_run_check(&asset);
    assert_eq!(
        result,
        Err(Ok(OracleSafetyViolation::CircuitBreakerOpen)),
        "open breaker must short-circuit lastprice via check"
    );
}

/// Once the ledger sequence passes `halt_until_ledger`, the breaker
/// must auto-close on the next `check_circuit_breaker` call. Pins the
/// auto-recovery contract: integrators do not need a manual reset path.
#[test]
fn test_open_breaker_auto_recovers_after_halt_window() {
    let (env, client) = setup();
    let asset = Asset::Stellar(Address::generate(&env));

    let initial_seq = env.ledger().sequence();
    client.run_open(&asset, &10);

    // Advance the ledger past `halt_until_ledger = initial_seq + 10`.
    env.ledger().with_mut(|li| {
        li.sequence_number = initial_seq + 11;
    });

    let result = client.try_run_check(&asset);
    assert!(
        result.is_ok(),
        "halt window expired — breaker must auto-close, got {:?}",
        result
    );
}

/// `close_circuit_breaker` must reset state regardless of how it got
/// there. Pins the governance override path: an admin-driven close
/// produces the same observable state as auto-recovery.
#[test]
fn test_close_after_open_resets_state() {
    let (env, client) = setup();
    let asset = Asset::Stellar(Address::generate(&env));

    client.run_open(&asset, &720);
    client.run_close(&asset);

    let result = client.try_run_check(&asset);
    assert!(
        result.is_ok(),
        "manual close must reset state, got {:?}",
        result
    );
}

/// The breaker is per-asset: a halt on one asset must not block another.
/// This is the central isolation property that makes the breaker safe to
/// integrate at the library level — a manipulated price feed for asset A
/// cannot freeze borrowing for unrelated asset B in the same lending pool.
#[test]
fn test_breaker_isolated_per_asset() {
    let (env, client) = setup();
    let asset_a = Asset::Stellar(Address::generate(&env));
    let asset_b = Asset::Stellar(Address::generate(&env));

    client.run_open(&asset_a, &720);

    let result_a = client.try_run_check(&asset_a);
    assert_eq!(
        result_a,
        Err(Ok(OracleSafetyViolation::CircuitBreakerOpen)),
        "asset_a must be halted"
    );

    let result_b = client.try_run_check(&asset_b);
    assert!(
        result_b.is_ok(),
        "asset_b must remain Closed despite asset_a halt, got {:?}",
        result_b
    );
}

/// `Asset::Stellar` and `Asset::Other` use distinct `CBStorageKey`
/// variants, so opening a breaker for one must not affect the other —
/// even when the addresses/symbols would otherwise look "the same" to a
/// caller treating the two variants interchangeably. Locks the
/// type-partitioned key space at the storage boundary.
#[test]
fn test_asset_other_uses_separate_storage_path() {
    let (env, client) = setup();
    let stellar_asset = Asset::Stellar(Address::generate(&env));
    let other_asset = Asset::Other(Symbol::new(&env, "BTC"));

    client.run_open(&stellar_asset, &720);

    let result = client.try_run_check(&other_asset);
    assert!(
        result.is_ok(),
        "Asset::Other must have independent breaker state from Asset::Stellar, got {:?}",
        result
    );
}

/// A second `open_circuit_breaker` call must overwrite the first's
/// `halt_until_ledger`, not preserve the shorter window. A fresh
/// violation extends the halt forward — anything else would let a
/// rapid-fire attacker effectively shorten the breaker by re-triggering
/// it just before the prior window expires.
#[test]
fn test_open_overwrites_existing_halt_window() {
    let (env, client) = setup();
    let asset = Asset::Stellar(Address::generate(&env));

    let initial_seq = env.ledger().sequence();

    client.run_open(&asset, &10);
    client.run_open(&asset, &1000);

    // Advance to a sequence where the first 10-ledger window would have
    // already auto-recovered if it had not been overwritten.
    env.ledger().with_mut(|li| {
        li.sequence_number = initial_seq + 50;
    });

    let result = client.try_run_check(&asset);
    assert_eq!(
        result,
        Err(Ok(OracleSafetyViolation::CircuitBreakerOpen)),
        "second open must overwrite first; longer window still active"
    );
}

// ===== Phase 5.2: lastprice() integration tests =====
//
// Run through the real `lastprice` wrapper via `TestEnv`'s `OracleHost`
// harness (Pre-5.2.B). Unlike the unit tests above — which exercise the
// state machine in isolation through a thin `TestHost` — these tests pin
// the integrator-facing contract that Phase 5.2's wrapper logic must hold.

/// Default config (`circuit_breaker_enabled = false`) must NOT open the
/// breaker when a guardrail trips. Two consecutive `lastprice` calls with
/// the same violating setup must surface the *same* guardrail variant —
/// not `CircuitBreakerOpen` — proving the breaker was never persisted.
///
/// Regression guard for the opt-in contract: a future refactor that flips
/// the default to `true` (or runs `open_circuit_breaker` unconditionally)
/// would fail this test on the second call.
#[test]
fn test_lastprice_does_not_open_breaker_when_disabled_by_default() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    // Layer 1 trip: 100× spike between consecutive Reflector ticks.
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, ONE_DOLLAR * 100, 99_950);
    // Healthy snapshot — proves Layer 2 is not what's surfacing the error.
    test_env.write_snapshot_now(&asset_address, HEALTHY_VOLUME_USD, 10_u32);

    let config = SafeOracleConfig::default();
    assert!(
        !config.circuit_breaker_enabled,
        "default config must keep the breaker disabled"
    );

    let result1 = test_env.lastprice(&asset, &config);
    assert_eq!(
        result1,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "first call must surface the deviation guardrail"
    );

    // Second call hits the same setup. If the breaker had been opened by
    // the first failure, this would return `CircuitBreakerOpen` instead.
    let result2 = test_env.lastprice(&asset, &config);
    assert_eq!(
        result2,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "second call must still hit the same guardrail, NOT CircuitBreakerOpen — \
         breaker was disabled and must not have been opened by first call"
    );
}
