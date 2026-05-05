//! Integration tests for `safe_oracle`.
//!
//! These tests use the `test-utils` crate (which itself depends on `safe-oracle`),
//! so they must live in the integration test directory rather than `lib.rs`'s
//! `mod test`. Inline unit tests would force `safe-oracle` to be compiled twice
//! (once as a normal dep of `test-utils`, once as a test target), and Rust would
//! treat the two builds as different crates — every shared type would mismatch.
//! Integration tests in `tests/` see `safe-oracle` as a single normal dependency,
//! which matches `test-utils`' view — types unify, and the cycle disappears.

use safe_oracle::{Asset, OracleSafetyViolation};
use soroban_sdk::{testutils::Ledger as _, Symbol};
use test_utils::TestEnv;

/// Happy path: after injecting two prices into mock-reflector, the real
/// cross-contract `lastprice` call must return `Ok(PriceData)`. Post Phase
/// 2.3b `check_deviation` requires 2 records; the deviation between them
/// stays under the `relaxed_config` threshold so Layer 1 passes.
#[test]
fn test_lastprice_with_real_reflector_call() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    // 14-decimal precision: $1.00 → $1.01 (1 % change, well under relaxed cap)
    test_env.set_oracle_price(&asset, 100_000_000_000_000, 12000);
    test_env.set_oracle_price(&asset, 101_000_000_000_000, 12345);

    let config = TestEnv::relaxed_config();
    let result = test_env.lastprice(&asset, &config);

    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    let price_data = result.unwrap();
    assert_eq!(price_data.price, 101_000_000_000_000);
    assert_eq!(price_data.timestamp, 12345);
}

/// When Reflector holds no prices for the asset, `lastprices` returns `None`
/// and `fetch_reflector_prices` maps it fail-safe to `Err(StaleData)`.
#[test]
fn test_lastprice_returns_stale_data_when_reflector_has_no_price() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC")); // no price set
    let config = TestEnv::relaxed_config();

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(result, Err(OracleSafetyViolation::StaleData));
}

/// 5% change stays under `relaxed_config` (max=5000 BPS) → Ok.
#[test]
fn test_deviation_passes_with_small_change() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    // $1.00 → $1.05 (5 % change = 500 BPS)
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 1000);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR + TestEnv::ONE_DOLLAR / 20, 1300);

    let config = TestEnv::relaxed_config(); // max_deviation_bps = 5000
    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok for 5% change, got {:?}",
        result
    );
}

/// 25% change exceeds the `strict_config` threshold (max=2000 BPS) → ExcessiveDeviation.
#[test]
fn test_deviation_fails_at_threshold_breach() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    // $100 → $125 (25 % change = 2500 BPS)
    test_env.set_oracle_price(&asset, 100 * TestEnv::ONE_DOLLAR, 1000);
    test_env.set_oracle_price(&asset, 125 * TestEnv::ONE_DOLLAR, 1300);

    let config = TestEnv::strict_config(); // max_deviation_bps = 2000
    let result = test_env.lastprice(&asset, &config);

    assert_eq!(result, Err(OracleSafetyViolation::ExcessiveDeviation));
}

/// Exactly 20% change at the boundary — since the check uses `>` (not `>=`),
/// equality passes; only deviation *exceeding* the threshold is rejected.
#[test]
fn test_deviation_passes_at_exact_threshold() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    // strict_config max_staleness_seconds=300; align ledger time to the price
    // timestamps so this test exercises *deviation* not staleness.
    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    // $100 → $120 (exactly 2000 BPS)
    test_env.set_oracle_price(&asset, 100 * TestEnv::ONE_DOLLAR, 1000);
    test_env.set_oracle_price(&asset, 120 * TestEnv::ONE_DOLLAR, 1300);

    let config = TestEnv::strict_config(); // max_deviation_bps = 2000
    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok at exact threshold (2000 BPS == max), got {:?}",
        result
    );
}

/// YieldBlox-class attack simulation: a small trade in a thin SDEX market
/// pumps the price from $1.05 → $106. The strict guardrail must reject this;
/// pitch slide caption: "if this test passes, the project works".
#[test]
fn test_deviation_yieldblox_attack_simulation() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USTRY"));

    // Baseline: $1.05 — then the attacker pumps it to $106 via a ~$5 SDEX trade.
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR + TestEnv::ONE_DOLLAR / 20, 1000);
    test_env.set_oracle_price(&asset, 106 * TestEnv::ONE_DOLLAR, 1300);

    let config = TestEnv::strict_config();
    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "YieldBlox-class attack must be blocked by deviation guardrail"
    );
}

/// With only one stored price, deviation cannot be computed —
/// `fetch_reflector_prices(records=2)` sees `len < records` and returns StaleData.
#[test]
fn test_deviation_fails_when_only_one_price_in_history() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 1000);
    // intentionally only one price — deviation needs two

    let config = TestEnv::relaxed_config();
    let result = test_env.lastprice(&asset, &config);

    assert_eq!(result, Err(OracleSafetyViolation::StaleData));
}

/// If the previous price is 0, we return ExcessiveDeviation before dividing
/// by it — treating it as a manipulation signal (current positive, zero →
/// any BPS = ∞).
#[test]
fn test_deviation_fails_when_previous_price_is_zero() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "WEIRD"));

    test_env.set_oracle_price(&asset, 0, 1000); // zero baseline (manipulation signal)
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 1300);

    let config = TestEnv::relaxed_config();
    let result = test_env.lastprice(&asset, &config);

    assert_eq!(result, Err(OracleSafetyViolation::ExcessiveDeviation));
}

/// Price is 100 seconds old; `relaxed_config` tolerates 100_000 seconds → Ok.
#[test]
fn test_staleness_passes_when_data_is_fresh() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 5000;
    });

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 4800); // 200s old
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 4900); // 100s old (current)

    let config = TestEnv::relaxed_config();
    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok for 100s stale data, got {:?}",
        result
    );
}

/// A 4000-second-old price returns StaleData under `strict_config` (300s tolerance).
#[test]
fn test_staleness_fails_when_data_too_old() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 5000;
    });

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 800); // 4200s old
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 1000); // 4000s old (current)

    let config = TestEnv::strict_config(); // max_staleness_seconds = 300
    let result = test_env.lastprice(&asset, &config);

    assert_eq!(result, Err(OracleSafetyViolation::StaleData));
}

/// Future timestamp (current.timestamp > now) is a clock-skew or feed-manipulation
/// signal; the defensive future-check returns StaleData.
#[test]
fn test_staleness_fails_with_future_timestamp() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 5000;
    });

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 5500); // previous record (also future)
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 6000); // future (current)

    let config = TestEnv::relaxed_config();
    let result = test_env.lastprice(&asset, &config);

    assert_eq!(result, Err(OracleSafetyViolation::StaleData));
}

/// Exactly at the strict threshold (300s) — since the check uses `>` (not
/// `>=`), equality passes; only ages *exceeding* the threshold are rejected.
/// Consistent with `check_deviation`'s threshold semantics.
#[test]
fn test_staleness_passes_at_exact_threshold() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 5000;
    });

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 4500); // 500s old
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 4700); // exactly 300s old

    let config = TestEnv::strict_config(); // max_staleness_seconds = 300
    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok at exact threshold (300s == max), got {:?}",
        result
    );
}

/// secondary_oracle = None → cross-source check skip → Ok.
/// (Single-source operation must remain valid by default.)
#[test]
fn test_cross_source_skipped_when_secondary_is_none() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    let mut config = TestEnv::relaxed_config();
    config.secondary_oracle = None;

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok when secondary is None, got {:?}",
        result
    );
}

/// Both sources report the same price → 0 BPS deviation → Ok.
#[test]
fn test_cross_source_passes_with_matching_prices() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
    test_env.set_secondary_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    let mut config = TestEnv::relaxed_config();
    config.secondary_oracle = Some(test_env.secondary_reflector_address.clone());

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok for matching prices, got {:?}",
        result
    );
}

/// Primary $1.00 vs secondary $1.03 = 300 BPS, relaxed max=2000 → Ok.
#[test]
fn test_cross_source_passes_with_small_difference() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950); // primary $1.00
    test_env.set_secondary_oracle_price(&asset, TestEnv::ONE_DOLLAR * 103 / 100, 99_950); // secondary $1.03

    let mut config = TestEnv::relaxed_config(); // max_cross_source_bps = 2000
    config.secondary_oracle = Some(test_env.secondary_reflector_address.clone());

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok for 300 BPS deviation, got {:?}",
        result
    );
}

/// Primary $1.00 vs secondary $1.07 = 700 BPS, strict max=500 → CrossSourceMismatch.
#[test]
fn test_cross_source_fails_with_excessive_difference() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    // strict_config max_staleness_seconds=300; align ledger time to the price
    // timestamps so this test exercises *cross-source*, not staleness.
    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 100_000;
    });

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
    test_env.set_secondary_oracle_price(&asset, TestEnv::ONE_DOLLAR * 107 / 100, 99_950); // secondary $1.07

    let mut config = TestEnv::strict_config(); // max_cross_source_bps = 500
    config.secondary_oracle = Some(test_env.secondary_reflector_address.clone());

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(result, Err(OracleSafetyViolation::CrossSourceMismatch));
}

/// Secondary has no price for the asset → "no evidence" semantic → Ok (skip, not fail).
#[test]
fn test_cross_source_skipped_when_secondary_returns_none() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "RARE"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
    // secondary intentionally has no price for this asset

    let mut config = TestEnv::relaxed_config();
    config.secondary_oracle = Some(test_env.secondary_reflector_address.clone());

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok when secondary has no data, got {:?}",
        result
    );
}

/// Secondary price of 0 → "live feed reporting zero" manipulation signal →
/// CrossSourceMismatch. (Distinction: None = no data, 0 = manipulated price.)
#[test]
fn test_cross_source_fails_when_secondary_price_is_zero() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "WEIRD"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
    test_env.set_secondary_oracle_price(&asset, 0, 99_950); // zero = manipulation, not gap

    let mut config = TestEnv::relaxed_config();
    config.secondary_oracle = Some(test_env.secondary_reflector_address.clone());

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(result, Err(OracleSafetyViolation::CrossSourceMismatch));
}
