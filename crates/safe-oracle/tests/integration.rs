//! Integration tests for `safe_oracle`.
//!
//! These tests use the `test-utils` crate (which itself depends on `safe-oracle`),
//! so they must live in the integration test directory rather than `lib.rs`'s
//! `mod test`. Inline unit tests would force `safe-oracle` to be compiled twice
//! (once as a normal dep of `test-utils`, once as a test target), and Rust would
//! treat the two builds as different crates — every shared type would mismatch.
//! Integration tests in `tests/` see `safe-oracle` as a single normal dependency,
//! which matches `test-utils`' view — types unify, and the cycle disappears.

use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
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

    // Phase 7.2: align ledger time to keep `previous` within
    // `previous_max_staleness_seconds` (default 900s); otherwise the new
    // previous-staleness gate fires first and `ExcessiveDeviation` never
    // gets to surface. Same pattern as `test_deviation_passes_at_exact_threshold`.
    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

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

    // Phase 7.2: align ledger time so the previous-price freshness gate
    // (default 900s) does not preempt the deviation check.
    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

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

// ===== Hardening Phase debt #3: Secondary oracle staleness skip =====

/// Stale secondary feed → cross-source check silently skipped. The
/// secondary's old reading is "no fresh evidence" rather than "evidence of
/// mismatch", consistent with the `None` and "secondary returned `None`"
/// skip paths.
///
/// Behavior change from pre-Hardening: pre-3B this scenario would have
/// computed the BPS divergence against the stale secondary value and
/// produced a false-positive `CrossSourceMismatch`. Post-3B the stale
/// branch short-circuits to `Ok(())`.
#[test]
fn test_cross_source_stale_secondary_silently_skipped() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    // TestEnv baseline `now = 100_000`. Primary fresh (50s old).
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    // Secondary timestamp 99_000 → 1000 seconds old, well past default
    // `max_staleness_seconds = 300`. Price wildly diverges from primary
    // (100x) so the BPS check WOULD fire if cross-source ran.
    test_env.set_secondary_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_000);

    let config = SafeOracleConfig {
        secondary_oracle: Some(test_env.secondary_reflector_address.clone()),
        ..SafeOracleConfig::default()
    };

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "stale secondary must be silently skipped — not treated as mismatch evidence: {:?}",
        result
    );
}

/// Regression guard: a *fresh* secondary with large divergence still
/// produces `CrossSourceMismatch`. Pins that the staleness skip does NOT
/// disable cross-source checking entirely — only the stale-secondary
/// branch short-circuits.
#[test]
fn test_cross_source_fresh_secondary_disagreement_still_caught() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    // Secondary fresh (matches primary's 99_950 timestamp = 50s old) but
    // diverges 100x. BPS check must fire.
    test_env.set_secondary_oracle_price(&asset, TestEnv::ONE_DOLLAR * 100, 99_950);

    let config = SafeOracleConfig {
        secondary_oracle: Some(test_env.secondary_reflector_address.clone()),
        ..SafeOracleConfig::default()
    };

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::CrossSourceMismatch),
        "fresh secondary with major divergence must still produce CrossSourceMismatch"
    );
}

// ============================================================
// Phase 7.2 — design debt closure tests
// ============================================================

/// Phase 7.2: cross-source check rejects mismatched decimals between
/// primary and secondary oracles with `DecimalsMismatch` rather than
/// silently producing always-fires `CrossSourceMismatch`.
///
/// Pre-7.2 this scenario would hit the BPS comparison with raw `i128`
/// values across different scales — a `100 * 10^14` primary against a
/// `100 * 10^7` secondary registers as ~99.99% deviation and trips
/// `CrossSourceMismatch`. The new explicit guard surfaces the actual
/// configuration error so operators can fix the precision pair rather
/// than chase phantom price disagreements.
#[test]
fn test_cross_source_decimals_mismatch_rejects() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    // Primary stays at the TestEnv default of 14 decimals.
    // Secondary reconfigured to 7 decimals → mismatch.
    test_env.override_secondary_decimals(7);

    test_env.prime_layer1(&asset);
    // Secondary needs to return a fresh price so the cross-source check
    // reaches the decimals comparison (silent skip otherwise).
    test_env.set_secondary_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    let config = SafeOracleConfig {
        secondary_oracle: Some(test_env.secondary_reflector_address.clone()),
        ..SafeOracleConfig::default()
    };

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::DecimalsMismatch),
        "primary=14 / secondary=7 must surface DecimalsMismatch"
    );
}

/// Phase 7.2: when primary and secondary report identical decimals (the
/// expected production configuration), the cross-source check proceeds to
/// the BPS comparison as before. Regression guard against an over-eager
/// decimals gate that misclassifies same-precision pairs.
#[test]
fn test_cross_source_decimals_match_proceeds_to_bps_check() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    // Both default to 14; no override needed. Prices match → BPS = 0 → Ok.
    test_env.prime_layer1(&asset);
    test_env.set_secondary_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    let config = SafeOracleConfig {
        secondary_oracle: Some(test_env.secondary_reflector_address.clone()),
        ..SafeOracleConfig::default()
    };

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "matched decimals + matching prices must pass cross-source, got {:?}",
        result
    );
}

/// Phase 7.2: primary Reflector reporting a `decimals()` other than the
/// expected 14 is rejected with `UnexpectedDecimals`. This catches
/// misconfigured oracle addresses (e.g., wired to a non-Reflector contract)
/// and Reflector contract upgrades that change precision out from under
/// the integrator.
#[test]
fn test_primary_unexpected_decimals_rejects() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    // Reconfigure primary to 7 decimals. Prices remain at the standard
    // 14-decimal magnitude (irrelevant — the `decimals()` check fires
    // before any deviation/staleness math).
    test_env.override_primary_decimals(7);
    test_env.prime_layer1(&asset);

    let config = SafeOracleConfig::default();
    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::UnexpectedDecimals),
        "primary decimals=7 (expected 14) must surface UnexpectedDecimals"
    );
}

/// Phase 7.2: `previous` price older than `previous_max_staleness_seconds`
/// surfaces as `StaleData` (not misclassified `ExcessiveDeviation`). This
/// is the post-gap recovery scenario from the lib.rs:713 plan: an attestation
/// pipeline that goes silent for hours and then resumes leaves an ancient
/// `previous` paired with a fresh `current`; without this gate, the wide
/// price gap between the two would surface as a deviation violation.
#[test]
fn test_previous_price_too_stale_rejects() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    // TestEnv ledger now is 100_000.
    // - prev_ts = 99_000 → 1000s old (> 900s default prev gate → fail)
    // - curr_ts = 99_950 → 50s old (< 300s current gate → fresh)
    // - 0% deviation between identical prices (would otherwise pass)
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_000);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    let config = SafeOracleConfig::default();
    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::StaleData),
        "previous price beyond previous_max_staleness_seconds must surface StaleData"
    );
}

/// Phase 7.2: when `previous` is fresh and `current` is fresh, the path
/// proceeds to deviation/cross-source as before. Regression guard against
/// an over-strict prev-staleness gate that misclassifies normal cadence.
#[test]
fn test_previous_price_fresh_proceeds_normally() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    // Both within their respective gates: prev=99_500 (500s old, < 900s),
    // curr=99_950 (50s old, < 300s). Identical prices → no deviation. Ok.
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_500);
    test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);

    let config = SafeOracleConfig::default();
    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "previous price within previous_max_staleness_seconds must allow flow, got {:?}",
        result
    );
}
