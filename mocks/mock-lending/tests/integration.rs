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

use mock_lending::{BorrowOutcome, DataKey, MockLendingError};
use safe_oracle::{Asset, SafeOracleConfig};
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger as _},
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

/// `TestEnv::new()` initializes the lending contract in Phase 2.7; verify
/// that the expected fields landed in storage.
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
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    test_env.lending_client.borrow(&caller, &asset, &1000);

    let events = test_env.env.events().all();
    assert_eq!(events.events().len(), 1);
}

#[test]
fn test_borrow_succeeds_with_valid_oracle_data() {
    let test_env = TestEnv::new();
    let user = Address::generate(&test_env.env);
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

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
/// propagates transparently — the pitch demo (("if this test passes, the project works")).
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
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::ExcessiveDeviation as u32
        ))),
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
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 50_000);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 50_300);

    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_000);
    assert_eq!(
        result,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::StaleData as u32
        ))),
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

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let user = Address::generate(&test_env.env);
    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    assert_eq!(
        result,
        Ok(Ok(BorrowOutcome::Ok)),
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

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);

    // Healthy snapshot proves Layer 1 surfaces before Layer 2 is consulted.
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let user = Address::generate(&test_env.env);
    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    assert_eq!(
        result,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::ExcessiveDeviation as u32
        ))),
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

    let previous_price = TestEnv::ONE_DOLLAR;
    let current_price = TestEnv::ONE_DOLLAR + (TestEnv::ONE_DOLLAR / 20); // 5% spike
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
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::InsufficientLiquidity as u32
        ))),
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

    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let now = test_env.env.ledger().timestamp();
    let stale_ts = now.saturating_sub(3600);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, stale_ts.saturating_sub(100));
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, stale_ts);

    let user = Address::generate(&test_env.env);
    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    assert_eq!(
        result,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::StaleData as u32
        ))),
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

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    let now = test_env.env.ledger().timestamp();
    let stale_ts = now.saturating_sub(3600);
    test_env.write_snapshot(
        &asset_address,
        TestEnv::HEALTHY_VOLUME_USD,
        10_u32,
        stale_ts,
    );

    let user = Address::generate(&test_env.env);
    let result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    assert_eq!(
        result,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::StaleSnapshot as u32
        ))),
        "1-hour-old registry snapshot must block borrow (default 5-minute threshold)"
    );
}

// ===== Phase 5.4 v2: Circuit breaker via borrow() =====
//
// 5 e2e tests verifying circuit breaker behavior through MockLending::borrow()
// (Ok-API, Phase 5.4 v2). Auto-halt now commits because borrow() returns Ok at
// the Soroban boundary (BorrowOutcome::Failed wrapped in Ok), allowing
// safe_oracle::circuit_breaker::open_circuit_breaker writes to persist.
//
// Pre-5.2.D empirically eliminated 8 alternative mechanisms; caller Ok-API
// is the only viable path. This file is the lending-perspective complement
// to crates/safe-oracle/tests/circuit_breaker.rs (library perspective).

/// Auto-halt fires through the borrow path. First borrow surfaces the
/// guardrail violation; second borrow short-circuits with `CircuitBreakerOpen`
/// because the breaker write committed (Ok-API at the Soroban boundary).
/// This is the regression guard for the Phase 5.4 v1 bug — that test failed
/// with the Result-returning borrow because the breaker write rolled back.
#[test]
fn test_borrow_circuit_breaker_opens_after_first_violation() {
    let test_env = TestEnv::with_circuit_breaker_enabled();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let user = Address::generate(&test_env.env);

    let result1 = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        result1,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::ExcessiveDeviation as u32
        ))),
        "first borrow surfaces guardrail violation"
    );

    let result2 = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        result2,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::CircuitBreakerOpen as u32
        ))),
        "second borrow hits open breaker — auto-halt committed via Ok-API"
    );
}

/// Default config (`circuit_breaker_enabled = false`) preserves Phase 1-4
/// behavior across the borrow path. Five repeated violations surface the
/// underlying `ExcessiveDeviation`; the breaker is never opened.
#[test]
fn test_borrow_default_config_does_not_open_breaker() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let user = Address::generate(&test_env.env);

    for i in 0..5 {
        let result = test_env
            .lending_client
            .try_borrow(&user, &asset, &1_000_i128);
        assert_eq!(
            result,
            Ok(Ok(BorrowOutcome::Failed(
                MockLendingError::ExcessiveDeviation as u32
            ))),
            "call {} — same violation, no breaker open (default disabled)",
            i + 1
        );
    }
}

/// After the halt window expires, the next borrow auto-recovers the breaker
/// and re-runs the chain. Underlying `ExcessiveDeviation` surfaces again —
/// the breaker buys a cool-down, it does not paper over a still-broken oracle.
#[test]
fn test_borrow_breaker_auto_recovers_after_halt_window() {
    let test_env = TestEnv::with_circuit_breaker_enabled_and_halt_ledgers(10);
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let user = Address::generate(&test_env.env);
    let initial_seq = test_env.env.ledger().sequence();

    // Open breaker.
    let _ = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    // During halt window: short-circuit.
    let result_during = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        result_during,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::CircuitBreakerOpen as u32
        ))),
        "during halt window, borrow short-circuits"
    );

    // Advance past halt window.
    test_env.env.ledger().with_mut(|li| {
        li.sequence_number = initial_seq + 11;
    });

    let result_after = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        result_after,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::ExcessiveDeviation as u32
        ))),
        "after halt expired, breaker auto-closes — underlying violation re-surfaces"
    );
}

/// Per-asset halt does not bleed across assets through the borrow path.
/// Production property for lending pools serving multiple collateral types —
/// a manipulated feed for one asset must not freeze borrows against others.
#[test]
fn test_borrow_breaker_isolation_between_assets() {
    let test_env = TestEnv::with_circuit_breaker_enabled();
    let asset_a_addr = Address::generate(&test_env.env);
    let asset_b_addr = Address::generate(&test_env.env);
    let asset_a = Asset::Stellar(asset_a_addr.clone());
    let asset_b = Asset::Stellar(asset_b_addr.clone());

    // asset_a: violation
    test_env.set_oracle_price(&asset_a, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset_a, TestEnv::ONE_DOLLAR * 100, 99_950);
    test_env.write_snapshot_now(&asset_a_addr, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    // asset_b: healthy
    test_env.set_oracle_price(&asset_b, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset_b, TestEnv::ONE_DOLLAR, 99_950);
    test_env.write_snapshot_now(&asset_b_addr, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let user = Address::generate(&test_env.env);

    // Trigger asset_a halt.
    let _ = test_env
        .lending_client
        .try_borrow(&user, &asset_a, &1_000_i128);

    let result_a = test_env
        .lending_client
        .try_borrow(&user, &asset_a, &1_000_i128);
    assert_eq!(
        result_a,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::CircuitBreakerOpen as u32
        ))),
        "asset_a borrow halted"
    );

    let result_b = test_env
        .lending_client
        .try_borrow(&user, &asset_b, &1_000_i128);
    assert_eq!(
        result_b,
        Ok(Ok(BorrowOutcome::Ok)),
        "asset_b borrow succeeds despite asset_a halt"
    );
}

/// Separation of concerns: the breaker protects price-dependent operations
/// (borrow). Operations that do not consult the oracle (deposit) MUST remain
/// functional during a halt — otherwise the breaker traps user funds, turning
/// a defensive measure into a denial-of-service vector.
#[test]
fn test_borrow_halted_asset_still_allows_deposit() {
    let test_env = TestEnv::with_circuit_breaker_enabled();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let user = Address::generate(&test_env.env);

    // Trigger halt.
    let _ = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    let borrow_result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        borrow_result,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::CircuitBreakerOpen as u32
        ))),
        "borrow must be halted to set up the test premise"
    );

    // Deposit does not consult the oracle — must succeed.
    let deposit_result = test_env
        .lending_client
        .try_deposit(&user, &asset, &500_i128);
    assert!(
        deposit_result.is_ok(),
        "deposit must succeed during borrow halt — circuit breaker only \
         affects price-dependent operations: got {:?}",
        deposit_result
    );
}

// ===== Hardening Phase debt #19: Asset::Other CB e2e =====
//
// Phase 5.4 v2 added 5 e2e circuit-breaker tests through `borrow()`, all
// against `Asset::Stellar`. This block mirrors those scenarios with
// `Asset::Other` so off-chain Reflector feeds (CEX-priced assets like BTC,
// ETH, SOL — no SDEX presence) get the same coverage. Layer 2 is
// structurally skipped for `Asset::Other` (no `LiquidityRegistry`
// snapshot path), so each scenario triggers a Layer 1 violation
// (`ExcessiveDeviation`) to drive the breaker.

/// Asset::Other counterpart of `test_borrow_circuit_breaker_opens_after_first_violation`.
/// Pins that auto-halt commits via `borrow()` for off-chain feeds too.
#[test]
fn test_borrow_circuit_breaker_opens_after_first_violation_other() {
    let test_env = TestEnv::with_circuit_breaker_enabled();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    // Layer 1 violation: 100x spike between consecutive Reflector ticks.
    // Asset::Other skips Layer 2 entirely.
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);

    let user = Address::generate(&test_env.env);

    let result1 = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        result1,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::ExcessiveDeviation as u32
        ))),
        "first borrow surfaces guardrail violation for Asset::Other"
    );

    let result2 = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        result2,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::CircuitBreakerOpen as u32
        ))),
        "second borrow hits open breaker — auto-halt committed for Asset::Other"
    );
}

/// Asset::Other counterpart of `test_borrow_default_config_does_not_open_breaker`.
/// Default config (`circuit_breaker_enabled = false`) preserves Phase 1-4
/// behavior on the off-chain feed path: repeated violations never open the
/// breaker.
#[test]
fn test_borrow_default_config_does_not_open_breaker_other() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);

    let user = Address::generate(&test_env.env);

    for i in 0..5 {
        let result = test_env
            .lending_client
            .try_borrow(&user, &asset, &1_000_i128);
        assert_eq!(
            result,
            Ok(Ok(BorrowOutcome::Failed(
                MockLendingError::ExcessiveDeviation as u32
            ))),
            "Asset::Other call {} — same violation, no breaker open (default disabled)",
            i + 1
        );
    }
}

/// Asset::Other counterpart of `test_borrow_breaker_auto_recovers_after_halt_window`.
/// The halt expires; the next call auto-closes and re-runs the chain — the
/// underlying violation re-surfaces, proving the breaker buys a cool-down
/// rather than papering over a still-broken oracle.
#[test]
fn test_borrow_breaker_auto_recovers_after_halt_window_other() {
    let test_env = TestEnv::with_circuit_breaker_enabled_and_halt_ledgers(10);
    let asset = Asset::Other(Symbol::new(&test_env.env, "SOL"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);

    let user = Address::generate(&test_env.env);
    let initial_seq = test_env.env.ledger().sequence();

    // Open breaker.
    let _ = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    // During halt window: short-circuit.
    let result_during = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        result_during,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::CircuitBreakerOpen as u32
        ))),
        "Asset::Other halt window: short-circuit"
    );

    // Advance past halt window.
    test_env.env.ledger().with_mut(|li| {
        li.sequence_number = initial_seq + 11;
    });

    let result_after = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        result_after,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::ExcessiveDeviation as u32
        ))),
        "Asset::Other auto-recovery: underlying violation re-surfaces"
    );
}

/// Asset::Other counterpart of `test_borrow_breaker_isolation_between_assets`.
/// Two off-chain feeds: a halt on one must not bleed into the other. The
/// `CBStorageKey::OtherAsset(Symbol)` partitioning makes this isolation a
/// type-system property; this test pins it through the borrow path.
#[test]
fn test_borrow_breaker_isolation_between_other_assets() {
    let test_env = TestEnv::with_circuit_breaker_enabled();
    let asset_btc = Asset::Other(Symbol::new(&test_env.env, "BTC"));
    let asset_eth = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    // BTC: violation on the next read.
    test_env.set_oracle_price(&asset_btc, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset_btc, TestEnv::ONE_DOLLAR * 100, 99_950);

    // ETH: stable, all guardrails pass.
    test_env.set_oracle_price(&asset_eth, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset_eth, TestEnv::ONE_DOLLAR, 99_950);

    let user = Address::generate(&test_env.env);

    // Trigger BTC halt.
    let _ = test_env
        .lending_client
        .try_borrow(&user, &asset_btc, &1_000_i128);

    let result_btc = test_env
        .lending_client
        .try_borrow(&user, &asset_btc, &1_000_i128);
    assert_eq!(
        result_btc,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::CircuitBreakerOpen as u32
        ))),
        "BTC (Asset::Other) borrow halted"
    );

    let result_eth = test_env
        .lending_client
        .try_borrow(&user, &asset_eth, &1_000_i128);
    assert_eq!(
        result_eth,
        Ok(Ok(BorrowOutcome::Ok)),
        "ETH (Asset::Other) borrow succeeds despite BTC halt — separate storage path"
    );
}

/// Asset::Other counterpart of `test_borrow_halted_asset_still_allows_deposit`.
/// Separation of concerns: the breaker affects only oracle-dependent
/// operations. Deposits on a halted Asset::Other must still succeed —
/// otherwise the breaker traps user funds and turns a defensive measure
/// into a denial-of-service.
#[test]
fn test_borrow_halted_other_asset_still_allows_deposit() {
    let test_env = TestEnv::with_circuit_breaker_enabled();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);

    let user = Address::generate(&test_env.env);

    // Trigger halt.
    let _ = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);

    let borrow_result = test_env
        .lending_client
        .try_borrow(&user, &asset, &1_000_i128);
    assert_eq!(
        borrow_result,
        Ok(Ok(BorrowOutcome::Failed(
            MockLendingError::CircuitBreakerOpen as u32
        ))),
        "Asset::Other borrow must be halted to set up the test premise"
    );

    // Deposit does not consult the oracle — must succeed even on halted asset.
    let deposit_result = test_env
        .lending_client
        .try_deposit(&user, &asset, &500_i128);
    assert!(
        deposit_result.is_ok(),
        "Asset::Other deposit must succeed during borrow halt: got {:?}",
        deposit_result
    );
}
