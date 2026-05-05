//! Graceful cross-contract failure tests (Hardening Phase debt #4).
//!
//! Verifies that Reflector and `LiquidityRegistry` contract traps surface
//! as `OracleSafetyViolation::ExternalContractFailure` rather than
//! propagating to the lending contract. Without this guarantee a failed
//! cross-contract invocation would prevent auto-halt from committing —
//! same root-cause family as the Phase 5.2 v1 revert.
//!
//! Each test registers a deliberately-panicking contract for the
//! component under test and routes the call through `TestEnv`'s
//! `OracleHost` harness, which exercises the same `safe_oracle::lastprice`
//! call path production integrators use.

use safe_oracle::{Asset, LiquiditySnapshot, OracleSafetyViolation, PriceData, SafeOracleConfig};
use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, Vec};
use test_utils::TestEnv;

// ===== Panicking contract fixtures =====

/// A `Reflector`-shaped contract that panics on every method. Used to
/// exercise the `ExternalContractFailure` path for primary-feed traps and
/// the silent-skip path for secondary-feed traps.
#[contract]
pub struct PanickingReflector;

#[contractimpl]
impl PanickingReflector {
    pub fn lastprice(_env: Env, _asset: Asset) -> Option<PriceData> {
        panic!("simulated Reflector trap in lastprice");
    }

    pub fn lastprices(_env: Env, _asset: Asset, _records: u32) -> Option<Vec<PriceData>> {
        panic!("simulated Reflector trap in lastprices");
    }

    // safe-oracle never calls `decimals` or `resolution`; placeholders are
    // included so the contract surface matches the trait the cross-contract
    // client expects to dispatch against.
    pub fn decimals(_env: Env) -> u32 {
        14
    }

    pub fn resolution(_env: Env) -> u32 {
        300
    }
}

/// A `LiquidityRegistry`-shaped contract whose read path panics. safe-oracle
/// only invokes `get_snapshot`; the other methods are not part of the
/// guardrail call path.
#[contract]
pub struct PanickingRegistry;

#[contractimpl]
impl PanickingRegistry {
    pub fn get_snapshot(_env: Env, _asset: Address) -> Option<LiquiditySnapshot> {
        panic!("simulated LiquidityRegistry trap in get_snapshot");
    }
}

// ===== Helpers =====

/// Drive `safe_oracle::lastprice` through the `OracleHost` harness with
/// caller-chosen reflector and registry addresses, then unwrap the
/// `try_*` and `PriceResult` layers down to a plain `Result`. Mirrors the
/// shape of `TestEnv::lastprice` but accepts a custom reflector and
/// registry rather than hard-coding `TestEnv`'s defaults — needed for
/// these tests, which inject a panicking contract for one of the two.
fn run_with_panicking_addresses(
    test_env: &TestEnv,
    asset: &Asset,
    reflector: &Address,
    registry: &Address,
    config: &SafeOracleConfig,
) -> Result<PriceData, OracleSafetyViolation> {
    let raw = test_env
        .oracle_host_client
        .try_run_lastprice(asset, reflector, registry, config);
    let price_result = raw
        .expect("oracle_host invocation must not host-error")
        .expect("OracleHost must not produce XDR conversion error");
    price_result.into_result()
}

// ===== Tests =====

/// Primary Reflector trap → `ExternalContractFailure`. Without the `try_*`
/// guard the panic would propagate through `safe_oracle::lastprice`,
/// reverting the borrow tx and leaving the breaker uncommitted.
#[test]
fn test_primary_reflector_panic_returns_external_contract_failure() {
    let test_env = TestEnv::new();
    let panicking_primary = test_env.env.register(PanickingReflector, ());

    let asset_addr = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_addr.clone());
    // Layer 2 setup is irrelevant — fetch_reflector_prices runs first and
    // traps on the panicking primary before the registry path is reached.
    test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

    let config = SafeOracleConfig::default();
    let result = run_with_panicking_addresses(
        &test_env,
        &asset,
        &panicking_primary,
        &test_env.registry,
        &config,
    );

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExternalContractFailure),
        "primary Reflector trap must surface as ExternalContractFailure (graceful)"
    );
}

/// `LiquidityRegistry` trap → `ExternalContractFailure`. Layer 1 must
/// pass first (Reflector healthy) so the call reaches `get_validated_snapshot`.
#[test]
fn test_registry_panic_returns_external_contract_failure() {
    let test_env = TestEnv::new();
    let panicking_registry = test_env.env.register(PanickingRegistry, ());

    let asset_addr = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_addr.clone());

    // Healthy Reflector inputs so Layer 1 clears.
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    let config = SafeOracleConfig::default();
    let result = run_with_panicking_addresses(
        &test_env,
        &asset,
        &test_env.reflector_address,
        &panicking_registry,
        &config,
    );

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExternalContractFailure),
        "registry trap must surface as ExternalContractFailure"
    );
}

/// Secondary Reflector trap → silent skip (cross-source short-circuits to
/// `Ok(())`); primary's value is returned unchanged. The cross-source
/// guardrail is opt-in defense-in-depth — a broken secondary must not
/// freeze borrowing on an otherwise-healthy primary.
#[test]
fn test_secondary_reflector_panic_silently_skipped() {
    let test_env = TestEnv::new();
    let panicking_secondary = test_env.env.register(PanickingReflector, ());

    let asset_addr = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_addr.clone());

    // Healthy primary + Layer 2.
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
    test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

    let config = SafeOracleConfig {
        secondary_oracle: Some(panicking_secondary),
        ..SafeOracleConfig::default()
    };

    let result = run_with_panicking_addresses(
        &test_env,
        &asset,
        &test_env.reflector_address,
        &test_env.registry,
        &config,
    );

    let price = result.expect("primary healthy + secondary skipped on trap → Ok");
    assert_eq!(
        price.price,
        TestEnv::ONE_DOLLAR,
        "primary's price must be returned when secondary traps (silent skip)"
    );
}

/// `ExternalContractFailure` triggers auto-halt when the breaker is
/// enabled. First call surfaces the failure; second call short-circuits
/// with `CircuitBreakerOpen` — same Phase 5.2 v2 mechanism, now wired for
/// the new variant. Pins that breaker auto-halt is unconditional on
/// guardrail variant: any violation, including the new one, opens it.
#[test]
fn test_external_failure_triggers_auto_halt() {
    let test_env = TestEnv::new();
    let panicking_primary = test_env.env.register(PanickingReflector, ());

    let asset_addr = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_addr.clone());
    test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

    let config = SafeOracleConfig {
        circuit_breaker_enabled: true,
        ..SafeOracleConfig::default()
    };

    let result1 = run_with_panicking_addresses(
        &test_env,
        &asset,
        &panicking_primary,
        &test_env.registry,
        &config,
    );
    assert_eq!(
        result1,
        Err(OracleSafetyViolation::ExternalContractFailure),
        "first call: primary panic → ExternalContractFailure"
    );

    let result2 = run_with_panicking_addresses(
        &test_env,
        &asset,
        &panicking_primary,
        &test_env.registry,
        &config,
    );
    assert_eq!(
        result2,
        Err(OracleSafetyViolation::CircuitBreakerOpen),
        "second call: breaker auto-halted on first failure, short-circuits"
    );
}

/// Default config (`circuit_breaker_enabled = false`) preserves Phase 1-4
/// behavior on the new variant: repeated `ExternalContractFailure`s never
/// open the breaker. Pins that the auto-halt opt-in remains binary —
/// adding a new `OracleSafetyViolation` variant in 3C does not silently
/// change the default behavior.
#[test]
fn test_external_failure_default_config_does_not_open_breaker() {
    let test_env = TestEnv::new();
    let panicking_primary = test_env.env.register(PanickingReflector, ());

    let asset_addr = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_addr.clone());
    test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

    let config = SafeOracleConfig::default(); // breaker disabled
    assert!(!config.circuit_breaker_enabled);

    for i in 0..3 {
        let result = run_with_panicking_addresses(
            &test_env,
            &asset,
            &panicking_primary,
            &test_env.registry,
            &config,
        );
        assert_eq!(
            result,
            Err(OracleSafetyViolation::ExternalContractFailure),
            "call {} — default config must keep surfacing ExternalContractFailure (no breaker)",
            i + 1
        );
    }
}
