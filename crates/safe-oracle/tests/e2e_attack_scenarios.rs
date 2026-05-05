//! End-to-end attack scenario tests (Phase 4.4).
//!
//! Each test maps a real-world attack pattern to the `OracleSafetyViolation`
//! variant that blocks it, exercising the full `lastprice()` flow against
//! the integrated `LiquidityRegistry` mock — the same code path a lending
//! contract executes in production. Together they prove safe_oracle's
//! 5-guardrail design defends against the YieldBlox attack (Feb 22, 2026 —
//! $10.2M drained from Blend's YieldBlox pool via a $5 SDEX trade) and
//! sophisticated variants designed to evade Layer 1.
//!
//! The Scenario 2 / Scenario 3 pair carries the library's central narrative:
//! Layer 1 alone catches aggressive manipulation (huge price spikes), but
//! Layer 2 is required to catch sub-threshold manipulation against a thin
//! order book — exactly the gap that an attacker who has read past
//! post-mortems will target next.
//!
//! Demo command: `cargo test --test e2e_attack_scenarios`

use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::{testutils::Address as _, Address};
use test_utils::TestEnv;

/// Scenario 1: Normal borrow — happy path.
///
/// All five guardrails pass under healthy market conditions: stable price,
/// fresh snapshot, sufficient SDEX volume, active trading. Pins the contract
/// that safe_oracle is a *pass-through* in the absence of attack signals —
/// integrators relying on it must not pay an availability cost in normal
/// operation.
#[test]
fn scenario_1_normal_borrow_happy_path() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    let price = result.expect("happy path must return Ok");
    assert_eq!(
        price.price,
        TestEnv::ONE_DOLLAR,
        "returned price must match Reflector current"
    );
}

/// Scenario 2: YieldBlox classic — $5 trade, 100× price spike.
///
/// Direct replication of the Feb 22, 2026 attack pattern: an attacker drives
/// Reflector's VWAP from $1 to $100 with a tiny SDEX trade in an illiquid
/// market, then borrows against the inflated collateral. Default
/// `max_deviation_bps = 2000` (20%); a 9900% jump is well past it.
///
/// Layer 1's `check_deviation` short-circuits before Layer 2 runs, so the
/// returned variant is `ExcessiveDeviation` — even though the snapshot
/// shipped here is healthy. Pairs with Scenario 3 to show the layered
/// defense: Layer 1 catches the aggressive case, Layer 2 catches the
/// sophisticated one.
#[test]
fn scenario_2_yieldblox_classic_blocked_by_layer1() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    // YieldBlox-shape spike: $1 → $100 between consecutive Reflector ticks.
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);

    // Healthy snapshot proves Layer 1 surfaces the violation before Layer 2
    // is even consulted.
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "100× price spike must be blocked by Layer 1 deviation check"
    );
}

/// Scenario 3: Sophisticated YieldBlox — sub-threshold spike + thin order book.
///
/// The realistic post-incident threat. An attacker who has read the
/// post-mortem dials the spike *just under* Layer 1's deviation threshold
/// (5% < 20%) but exploits the same root cause: a near-empty order book
/// where any next trade can move price arbitrarily. Layer 1 sees nothing
/// suspicious; Layer 2's `check_liquidity` reads the registry-attested
/// 30-minute volume and surfaces `InsufficientLiquidity`.
///
/// **This is the scenario that justifies LiquidityRegistry's existence.**
/// Without it, the only signal available is Reflector's own price feed —
/// which by construction lags the manipulation it is being used to verify.
/// The off-chain attestation pipeline closes that loop.
#[test]
fn scenario_3_yieldblox_sophisticated_blocked_by_layer2() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    // 5% spike: above retail noise, below Layer 1's 20% / 2000 BPS default.
    let previous_price = TestEnv::ONE_DOLLAR;
    let current_price = TestEnv::ONE_DOLLAR + (TestEnv::ONE_DOLLAR / 20);
    test_env.set_oracle_price(&asset, previous_price, 99_900);
    test_env.set_oracle_price(&asset, current_price, 99_950);

    // Drained order book: $0.0000005 of 30-minute volume — the YieldBlox
    // pre-condition that made the manipulation possible in the first place.
    test_env.write_snapshot_now(&asset_address, 5_i128, 10_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::InsufficientLiquidity),
        "Sophisticated 5%-spike attack must be blocked by Layer 2 liquidity check"
    );
}

/// Scenario 4: Direct liquidity manipulation — drained order book, healthy feed.
///
/// Reflector reports a clean, stable price (no manipulation visible to
/// Layer 1), but the attester's snapshot shows near-zero 30-minute SDEX
/// volume. Even with a stable feed, lending against an asset whose order
/// book has been drained is unsafe: the *next* trade — including the
/// borrower's liquidation — can move price by an unbounded amount.
/// `check_liquidity` is the structural defense for this asymmetry.
///
/// Volume is `1_i128` (= $0.0000001 at 7-decimal precision) rather than
/// literal zero because `LiquidityRegistry::write_snapshot` rejects
/// `volume_30m_usd <= 0` at write time as a defensive sanity guard against
/// a buggy or malicious attester (registry `InvalidSnapshot` error). The
/// smallest valid attestation is the most faithful representation of a
/// "drained" market available through the real write path.
#[test]
fn scenario_4_liquidity_manipulation_drained_orderbook() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);

    // Trade count is healthy on purpose — the asymmetry between
    // sub-threshold volume and adequate trade count isolates
    // `check_liquidity` from `check_thin_sampling`, which Scenario 5 covers.
    test_env.write_snapshot_now(&asset_address, 1_i128, 10_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::InsufficientLiquidity),
        "Drained SDEX must block borrowing even with a stable feed"
    );
}

/// Scenario 5: Thin sampling — healthy volume, single-trade window.
///
/// Volume comfortably clears `min_liquidity_usd`, but only one unique trade
/// occurred in the past hour. This is the "VWAP-of-one" pattern: a single
/// attacker-initiated trade dominates the price calculation, even though
/// the dollar value of that trade looks normal. `check_thin_sampling` is
/// the defense-in-depth complement to `check_liquidity` — volume thresholds
/// alone do not detect this shape.
#[test]
fn scenario_5_thin_sampling_single_trade() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);
    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 1_u32);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ThinSampling),
        "Single-trade window must be blocked even with sufficient volume"
    );
}

/// Scenario 6: Stale Reflector — feed not refreshing.
///
/// Reflector's most recent record is 1 hour old (default
/// `max_staleness_seconds = 300`). The off-chain feed has stalled — paused
/// upstream, network partition, or a deliberate hold — and the on-chain
/// price no longer reflects market reality. `check_staleness` rejects the
/// price; the lending contract surfaces the same discriminant up to its
/// caller.
#[test]
fn scenario_6_stale_oracle_no_recent_updates() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.write_snapshot_now(&asset_address, TestEnv::HEALTHY_VOLUME_USD, 10_u32);

    let now = test_env.env.ledger().timestamp();
    let stale_ts = now.saturating_sub(3600);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, stale_ts.saturating_sub(100));
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, stale_ts);

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::StaleData),
        "1-hour-old Reflector data must be rejected (default 5-minute threshold)"
    );
}

/// Scenario 7: Stale snapshot — attester pipeline offline.
///
/// Reflector and SDEX are healthy, but `oracle-watch` has stopped writing
/// snapshots — process crashed, RPC endpoint unreachable, signing key
/// rotated incorrectly. Without fresh attestation evidence, Layer 2 cannot
/// assert anything about the current liquidity state, and the integrator's
/// freshness threshold (default 300s) is what enforces this. Surfaces
/// `StaleSnapshot` rather than silently treating the old snapshot as
/// current — the fail-safe contract that lets integrators page on attester
/// downtime.
#[test]
fn scenario_7_stale_snapshot_attester_offline() {
    let test_env = TestEnv::new();
    let asset_address = Address::generate(&test_env.env);
    let asset = Asset::Stellar(asset_address.clone());

    test_env.prime_layer1(&asset);

    // Snapshot is 1 hour old; default `max_snapshot_age_seconds` is 300.
    let now = test_env.env.ledger().timestamp();
    let stale_ts = now.saturating_sub(3600);
    test_env.write_snapshot(
        &asset_address,
        TestEnv::HEALTHY_VOLUME_USD,
        10_u32,
        stale_ts,
    );

    let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

    assert_eq!(
        result,
        Err(OracleSafetyViolation::StaleSnapshot),
        "1-hour-old snapshot must be rejected (default 5-minute threshold)"
    );
}
