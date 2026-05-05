//! Property-based tests for safe_oracle's 5 guardrails.
//!
//! Uses `proptest` to drive random inputs across the input space, verifying
//! that each guardrail's invariants hold uniformly. Complements the
//! fixed-fixture suites (`integration.rs`, `e2e_attack_scenarios.rs`, etc.)
//! by sweeping the boundaries the fixed tests pin at single points.
//!
//! # Strategy
//!
//! For each guardrail, two properties:
//!
//! - **Soundness:** any input value (including extremes — `u32::MAX` BPS,
//!   future timestamps, zero volume) leaves the public API in a
//!   well-defined state — `Ok(PriceData)` or `Err(<known variant>)`,
//!   never a panic or silent garbage. The guardrails' defensive arithmetic
//!   (`checked_mul`, `<= 0` early-returns) is what this property pins.
//!
//! - **Threshold boundary:** the configured threshold partitions the input
//!   space into pass/fail regions exactly. Every value in the "should
//!   pass" region yields `Ok`; every value in the "should fail" region
//!   yields the matching `Err` variant. All five guardrails use strict
//!   comparisons (`>`, `<`), so exact equality with the threshold passes.
//!
//! Plus two composition properties: under healthy inputs `lastprice` is
//! always `Ok`; under any single-guardrail violation it always returns the
//! matching `Err` variant.
//!
//! # Snapshot files
//!
//! Soroban writes one `test_snapshot` file per case (default 256 cases per
//! property = ~3000 files for this suite). Unlike the fixed-fixture
//! snapshots elsewhere in the repo — which serve as deterministic
//! regression artifacts — these are non-reproducible across runs (each
//! case has different generated inputs). The
//! `crates/safe-oracle/test_snapshots/prop_*.json` pattern is therefore
//! `.gitignore`d.

use proptest::prelude::*;
use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::{testutils::Address as _, Address};
use test_utils::TestEnv;

/// Register a fresh asset on a fresh `TestEnv` and return both the
/// `Asset::Stellar(addr)` enum (for `lastprice`) and the inner `Address`
/// (for `write_snapshot_now`). Property tests call this so each generated
/// case starts from clean state.
fn fresh_asset<'a>(test_env: &'a TestEnv<'a>) -> (Asset, Address) {
    let addr = Address::generate(&test_env.env);
    (Asset::Stellar(addr.clone()), addr)
}

// ===== Layer 1, Guardrail 1 — Maximum Deviation =====

proptest! {
    /// Soundness: arbitrary deviation values (full u32 range) never panic
    /// the lastprice pipeline. `check_deviation`'s `checked_mul(10_000)`
    /// guards against i128 overflow on `abs_diff * 10_000`; this property
    /// pins that the guard surfaces as `Err(ExcessiveDeviation)` rather
    /// than aborting the transaction.
    #[test]
    fn prop_deviation_soundness(deviation_bps in 0u32..=u32::MAX) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
        let new_price = TestEnv::ONE_DOLLAR
            .saturating_add(TestEnv::ONE_DOLLAR.saturating_mul(deviation_bps as i128) / 10_000);
        test_env.set_oracle_price(&asset, new_price, 99_950);
        test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

        // The contract: any outcome OTHER than panic. Specific Ok/Err is
        // covered by the threshold property below.
        let _ = test_env.lastprice(&asset, &SafeOracleConfig::default());
    }

    /// Threshold boundary: deviation strictly greater than
    /// `max_deviation_bps` returns `ExcessiveDeviation`; values at-or-under
    /// the threshold pass. `check_deviation` uses strict `>` (not `>=`),
    /// so equality passes.
    #[test]
    fn prop_deviation_threshold_boundary(deviation_bps in 0u32..5_000u32) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
        // ONE_DOLLAR (1e14) is divisible by 10_000, so this multiplication
        // is exact — the deviation_bps recovered inside check_deviation
        // equals the input deviation_bps without truncation drift.
        let new_price =
            TestEnv::ONE_DOLLAR + (TestEnv::ONE_DOLLAR * deviation_bps as i128) / 10_000;
        test_env.set_oracle_price(&asset, new_price, 99_950);
        test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

        let config = SafeOracleConfig::default(); // max_deviation_bps = 2000
        let result = test_env.lastprice(&asset, &config);

        if deviation_bps > config.max_deviation_bps {
            prop_assert_eq!(result, Err(OracleSafetyViolation::ExcessiveDeviation));
        } else {
            prop_assert!(
                result.is_ok(),
                "deviation_bps {} <= threshold {} should pass, got {:?}",
                deviation_bps,
                config.max_deviation_bps,
                result
            );
        }
    }
}

// ===== Layer 1, Guardrail 3 — Staleness =====

proptest! {
    /// Soundness: arbitrary timestamp positions relative to `now` never
    /// panic — including future-dated prices (timestamp > now) which
    /// `check_staleness` rejects defensively as `StaleData`.
    #[test]
    fn prop_staleness_soundness(timestamp in 0u64..200_000u64) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        // Two records with the same timestamp difference is irrelevant
        // here — we only care that the pipeline does not panic for any
        // `current.timestamp` value, including ones beyond the
        // `TestEnv` baseline `now = 100_000`.
        let prev_ts = timestamp.saturating_sub(50);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, prev_ts);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, timestamp);
        test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

        let _ = test_env.lastprice(&asset, &SafeOracleConfig::default());
    }

    /// Threshold boundary: a price whose age exceeds
    /// `max_staleness_seconds` returns `StaleData`; ages at-or-under the
    /// threshold pass. `check_staleness` uses strict `>`, so equality
    /// passes.
    #[test]
    fn prop_staleness_threshold_boundary(seconds_old in 0u64..1_000u64) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        let now: u64 = 100_000; // TestEnv baseline
        let current_ts = now.saturating_sub(seconds_old);
        let prev_ts = current_ts.saturating_sub(50);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, prev_ts);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, current_ts);
        test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

        let config = SafeOracleConfig::default(); // max_staleness_seconds = 300
        let result = test_env.lastprice(&asset, &config);

        if seconds_old > config.max_staleness_seconds as u64 {
            prop_assert_eq!(result, Err(OracleSafetyViolation::StaleData));
        } else {
            prop_assert!(
                result.is_ok(),
                "seconds_old {} <= threshold {} should pass, got {:?}",
                seconds_old,
                config.max_staleness_seconds,
                result
            );
        }
    }
}

// ===== Layer 1, Guardrail 4 — Multi-Source Cross-Check =====

proptest! {
    /// Soundness: arbitrary secondary-feed deviation never panics. The
    /// `checked_mul(10_000)` inside `check_cross_source` guards i128
    /// overflow the same way `check_deviation` does.
    #[test]
    fn prop_cross_source_soundness(secondary_diff_bps in 0u32..=u32::MAX) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
        let secondary_price = TestEnv::ONE_DOLLAR.saturating_add(
            TestEnv::ONE_DOLLAR.saturating_mul(secondary_diff_bps as i128) / 10_000,
        );
        test_env.set_secondary_oracle_price(&asset, secondary_price, 99_950);
        test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

        let config = SafeOracleConfig {
            secondary_oracle: Some(test_env.secondary_reflector_address.clone()),
            ..SafeOracleConfig::default()
        };

        let _ = test_env.lastprice(&asset, &config);
    }

    /// Threshold boundary: secondary deviation strictly greater than
    /// `max_cross_source_bps` returns `CrossSourceMismatch`; values
    /// at-or-under pass. Strict `>` again.
    #[test]
    fn prop_cross_source_threshold_boundary(secondary_diff_bps in 0u32..2_000u32) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
        // ONE_DOLLAR is divisible by 10_000, so the recovered BPS inside
        // check_cross_source equals the input exactly.
        let secondary_price =
            TestEnv::ONE_DOLLAR + (TestEnv::ONE_DOLLAR * secondary_diff_bps as i128) / 10_000;
        test_env.set_secondary_oracle_price(&asset, secondary_price, 99_950);
        test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, 10);

        // Default `max_cross_source_bps = 500`; opt the secondary feed in.
        let config = SafeOracleConfig {
            secondary_oracle: Some(test_env.secondary_reflector_address.clone()),
            ..SafeOracleConfig::default()
        };
        let result = test_env.lastprice(&asset, &config);

        if secondary_diff_bps > config.max_cross_source_bps {
            prop_assert_eq!(result, Err(OracleSafetyViolation::CrossSourceMismatch));
        } else {
            prop_assert!(
                result.is_ok(),
                "secondary_diff_bps {} <= threshold {} should pass, got {:?}",
                secondary_diff_bps,
                config.max_cross_source_bps,
                result
            );
        }
    }
}

// ===== Layer 2, Guardrail 4 — Minimum Liquidity =====

proptest! {
    /// Soundness: arbitrary `volume_30m_usd` (positive, since the registry
    /// rejects `<= 0` at write time) flows through `check_liquidity`
    /// without panic.
    #[test]
    fn prop_liquidity_soundness(volume_usd in 1i128..1_000_000_000_000_000i128) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
        test_env.write_snapshot_now(&asset_addr, volume_usd, 10);

        let _ = test_env.lastprice(&asset, &SafeOracleConfig::default());
    }

    /// Threshold boundary: `volume < min_liquidity_usd` returns
    /// `InsufficientLiquidity`; volumes at-or-above pass.
    /// `check_liquidity` uses strict `<`, so equality passes.
    #[test]
    fn prop_liquidity_threshold_boundary(volume_usd in 1i128..200_000_000_000i128) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
        test_env.write_snapshot_now(&asset_addr, volume_usd, 10);

        let config = SafeOracleConfig::default(); // min_liquidity_usd = 100_000_000_000
        let result = test_env.lastprice(&asset, &config);

        if volume_usd < config.min_liquidity_usd {
            prop_assert_eq!(result, Err(OracleSafetyViolation::InsufficientLiquidity));
        } else {
            prop_assert!(
                result.is_ok(),
                "volume_usd {} >= threshold {} should pass, got {:?}",
                volume_usd,
                config.min_liquidity_usd,
                result
            );
        }
    }
}

// ===== Layer 2, Guardrail 5 — Thin Sampling =====

proptest! {
    /// Soundness: arbitrary `unique_trades_1h` value (full u32 range)
    /// never panics. There is no arithmetic to overflow here — the check
    /// is a direct comparison — so this property is effectively pinning
    /// that the field round-trips through `LiquiditySnapshot`
    /// serialization for every legal value.
    #[test]
    fn prop_thin_sampling_soundness(trade_count in 0u32..=u32::MAX) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
        test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, trade_count);

        let _ = test_env.lastprice(&asset, &SafeOracleConfig::default());
    }

    /// Threshold boundary: `unique_trades_1h < min_trade_count_1h`
    /// returns `ThinSampling`; counts at-or-above pass.
    #[test]
    fn prop_thin_sampling_threshold_boundary(trade_count in 0u32..50u32) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
        test_env.write_snapshot_now(&asset_addr, TestEnv::HEALTHY_VOLUME_USD, trade_count);

        let config = SafeOracleConfig::default(); // min_trade_count_1h = 5
        let result = test_env.lastprice(&asset, &config);

        if trade_count < config.min_trade_count_1h {
            prop_assert_eq!(result, Err(OracleSafetyViolation::ThinSampling));
        } else {
            prop_assert!(
                result.is_ok(),
                "trade_count {} >= threshold {} should pass, got {:?}",
                trade_count,
                config.min_trade_count_1h,
                result
            );
        }
    }
}

// ===== Composition properties =====

proptest! {
    /// Healthy inputs across the safe regions of every guardrail always
    /// yield `Ok`. This is the no-false-positive property: a well-behaved
    /// market should never see a borrow rejected as long as inputs stay
    /// inside the configured tolerances.
    #[test]
    fn prop_healthy_inputs_always_ok(
        // All values strictly under their respective threshold so every
        // guardrail passes. Volumes well above $10k threshold; trades
        // well above 5; no secondary so cross-source skips.
        deviation_bps in 0u32..2_000u32,
        seconds_old in 0u64..300u64,
        volume_usd in 100_000_000_000i128..1_000_000_000_000i128,
        trade_count in 5u32..1_000u32,
    ) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        let now: u64 = 100_000;
        let current_ts = now.saturating_sub(seconds_old);
        let prev_ts = current_ts.saturating_sub(50);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, prev_ts);
        let new_price =
            TestEnv::ONE_DOLLAR + (TestEnv::ONE_DOLLAR * deviation_bps as i128) / 10_000;
        test_env.set_oracle_price(&asset, new_price, current_ts);
        test_env.write_snapshot_now(&asset_addr, volume_usd, trade_count);

        let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

        prop_assert!(
            result.is_ok(),
            "healthy inputs (dev={}bps, age={}s, vol={}, trades={}) must pass: {:?}",
            deviation_bps,
            seconds_old,
            volume_usd,
            trade_count,
            result
        );
    }

    /// Any single Layer 2 violation must surface as the corresponding
    /// `Err` variant, regardless of whether other Layer 2 fields are
    /// healthy. Pins the no-silent-pass contract: a broken liquidity
    /// signal cannot be papered over by a healthy trade count.
    ///
    /// Layer 2 ordering is `check_liquidity` before `check_thin_sampling`
    /// (pinned in `layer2_thin_sampling.rs`), so a sub-threshold volume
    /// must surface `InsufficientLiquidity` even when the trade count
    /// would also fail.
    #[test]
    fn prop_no_silent_pass_under_layer2_violation(
        volume_usd in 1i128..50_000_000_000i128,    // strictly < $10k threshold
        trade_count in 0u32..u32::MAX,              // arbitrary
    ) {
        let test_env = TestEnv::new();
        let (asset, asset_addr) = fresh_asset(&test_env);

        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_900);
        test_env.set_oracle_price(&asset, TestEnv::ONE_DOLLAR, 99_950);
        test_env.write_snapshot_now(&asset_addr, volume_usd, trade_count);

        let result = test_env.lastprice(&asset, &SafeOracleConfig::default());

        // Layer 1 inputs are healthy; Layer 2 liquidity always fails.
        // Liquidity runs before thin-sampling (Phase 4.2 ordering), so
        // the surfaced variant must be InsufficientLiquidity even if
        // trade_count would also have tripped ThinSampling.
        prop_assert_eq!(result, Err(OracleSafetyViolation::InsufficientLiquidity));
    }
}
