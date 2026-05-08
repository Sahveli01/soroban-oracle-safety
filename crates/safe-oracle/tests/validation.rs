//! Unit tests for `SafeOracleConfig::validate()` (Hardening Phase debt #2).
//!
//! Each test pins one `ConfigError` variant or one conditional-validation
//! case. Together they freeze the validator's contract: which inputs are
//! rejected, which boundaries pass, which fields are skip-validated when
//! their guardrail is disabled.
//!
//! These tests do not exercise `lastprice` — validation is an init-time
//! concern, not a runtime concern. The `MockLending::initialize` path is
//! covered separately in `mocks/mock-lending/tests/integration.rs`.

use safe_oracle::{ConfigError, SafeOracleConfig};
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Default config must validate. This is a regression guard — if a future
/// tightening of `validate()` accidentally rejected the `Default::default()`
/// values, every existing test that constructs `SafeOracleConfig::default()`
/// would silently break at init time. Pinning this here surfaces the issue
/// in one focused failure rather than a cascade across the suite.
#[test]
fn test_validate_default_config_is_valid() {
    let config = SafeOracleConfig::default();
    assert!(
        config.validate().is_ok(),
        "default config must validate: {:?}",
        config.validate()
    );
}

/// `max_deviation_bps == 0` allows infinite deviation (the check returns
/// `Ok` for any input — silent disabling). Validator must reject.
#[test]
fn test_validate_zero_deviation_bps_rejected() {
    let config = SafeOracleConfig {
        max_deviation_bps: 0,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidDeviationBps),
        "zero max_deviation_bps disables the deviation check — must be rejected"
    );
}

/// `max_deviation_bps > 10_000` (i.e., > 100%) is structurally meaningless
/// for a relative-deviation threshold. Validator must reject.
#[test]
fn test_validate_excessive_deviation_bps_rejected() {
    let config = SafeOracleConfig {
        max_deviation_bps: 10_001,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidDeviationBps),
        "max_deviation_bps > 10_000 is nonsensical — must be rejected"
    );
}

/// `max_staleness_seconds == 0` rejects every recorded price as stale,
/// effectively bricking the oracle. Validator must reject.
#[test]
fn test_validate_zero_staleness_rejected() {
    let config = SafeOracleConfig {
        max_staleness_seconds: 0,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidStalenessSeconds),
        "zero max_staleness_seconds rejects all data — must be rejected"
    );
}

/// `max_staleness_seconds > 86_400` (24h) accepts day-old prices, which
/// is unsafe regardless of the integrator's leniency preferences.
#[test]
fn test_validate_excessive_staleness_rejected() {
    let config = SafeOracleConfig {
        max_staleness_seconds: 86_401,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidStalenessSeconds),
        "max_staleness_seconds > 24h is unsafe — must be rejected"
    );
}

/// `min_liquidity_usd < 0` is representable in `i128` but structurally
/// meaningless (liquidity is non-negative by definition).
#[test]
fn test_validate_negative_liquidity_rejected() {
    let config = SafeOracleConfig {
        min_liquidity_usd: -1,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidLiquidityThreshold),
        "negative min_liquidity_usd is meaningless — must be rejected"
    );
}

/// Cross-source validation is conditional: when `secondary_oracle` is
/// configured AND `max_cross_source_bps > 10_000`, the threshold is
/// nonsensical and validator rejects.
#[test]
fn test_validate_invalid_cross_source_bps_rejected() {
    let env = Env::default();
    let config = SafeOracleConfig {
        secondary_oracle: Some(Address::generate(&env)),
        max_cross_source_bps: 10_001,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidCrossSourceBps),
        "max_cross_source_bps > 10_000 with secondary configured — must be rejected"
    );
}

/// When `secondary_oracle` is `None`, `max_cross_source_bps` is dormant
/// (the cross-source guardrail short-circuits before reading it). The
/// validator skips the field entirely in this case — even an absurdly
/// large value is acceptable because it has no runtime effect.
#[test]
fn test_validate_cross_source_bps_skipped_when_no_secondary() {
    let config = SafeOracleConfig {
        secondary_oracle: None,
        max_cross_source_bps: 10_001, // would be invalid if secondary configured
        ..SafeOracleConfig::default()
    };
    assert!(
        config.validate().is_ok(),
        "max_cross_source_bps validation must skip when secondary_oracle is None"
    );
}

/// Halt-ledgers validation is conditional: when `circuit_breaker_enabled`
/// is true AND `circuit_breaker_halt_ledgers == 0`, the breaker would
/// fire and immediately auto-recover on the same call — providing zero
/// halt window. Validator rejects.
#[test]
fn test_validate_zero_halt_ledgers_rejected_when_breaker_enabled() {
    let config = SafeOracleConfig {
        circuit_breaker_enabled: true,
        circuit_breaker_halt_ledgers: 0,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidHaltLedgers),
        "halt_ledgers == 0 with breaker enabled is degenerate — must be rejected"
    );
}

/// When the circuit breaker is disabled (the Phase 1-4 default), the
/// halt-ledgers field is dormant. Validator skips it — `0` is acceptable.
#[test]
fn test_validate_zero_halt_ledgers_skipped_when_breaker_disabled() {
    let config = SafeOracleConfig {
        circuit_breaker_enabled: false,
        circuit_breaker_halt_ledgers: 0,
        ..SafeOracleConfig::default()
    };
    assert!(
        config.validate().is_ok(),
        "halt_ledgers validation must skip when circuit_breaker_enabled is false"
    );
}

// ===== Hardening Closure (Debt #22): Trade Count + Snapshot Age =====
//
// SafeOracleConfig::validate() runtime checks for the two remaining
// silent-disable cases identified in Hardening 3A but left as audit-trail
// gap until this Closure patch.

#[test]
fn test_validate_zero_trade_count_rejected() {
    let config = SafeOracleConfig {
        min_trade_count_1h: 0,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidTradeCountThreshold),
        "zero min_trade_count_1h disables thin-sampling check — must be rejected"
    );
}

#[test]
fn test_validate_zero_snapshot_age_rejected() {
    let config = SafeOracleConfig {
        max_snapshot_age_seconds: 0,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidSnapshotAge),
        "zero max_snapshot_age_seconds rejects all snapshots — must be rejected"
    );
}

#[test]
fn test_validate_excessive_snapshot_age_rejected() {
    let config = SafeOracleConfig {
        max_snapshot_age_seconds: 86_401, // > 24h
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidSnapshotAge),
        "snapshot age > 24h is unsafe staleness — must be rejected"
    );
}

/// Regression guard: default config must satisfy all 7 validation rules
/// (5 from Hardening 3A + 2 from Hardening Closure / Debt #22).
#[test]
fn test_validate_default_config_passes_all_seven_checks() {
    let config = SafeOracleConfig::default();
    assert!(config.validate().is_ok(), "default config must be valid");
}

// ===== AR.H M1: Zero Liquidity Threshold Silent Disable =====
//
// SafeOracleConfig::validate() runtime check for the third silent-disable
// case. Mirrors test_validate_zero_trade_count_rejected (Debt #22) and
// closes the asymmetry that AR.H (Adversarial Review) identified as the
// residual gap from Hardening Closure.

#[test]
fn test_validate_zero_liquidity_threshold_rejected() {
    let config = SafeOracleConfig {
        min_liquidity_usd: 0,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidLiquidityThreshold),
        "zero min_liquidity_usd silently disables the Layer 2 liquidity check — must be rejected"
    );
}

// ===== AR.H L1: Halt Ledgers Upper Bound =====
//
// SafeOracleConfig::validate() rejects circuit_breaker_halt_ledgers values
// beyond MAX_CIRCUIT_BREAKER_HALT_LEDGERS (~1 week). Without this guard,
// a misconfigured deploy with u32::MAX (~6.8 years) becomes effectively
// unrecoverable without governance.

#[test]
fn test_validate_excessive_halt_ledgers_rejected() {
    let config = SafeOracleConfig {
        circuit_breaker_enabled: true,
        circuit_breaker_halt_ledgers: u32::MAX,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidHaltLedgers),
        "u32::MAX halt_ledgers (~6.8 years) must be rejected"
    );
}

#[test]
fn test_validate_halt_ledgers_at_max_passes() {
    // Boundary regression: exactly MAX_CIRCUIT_BREAKER_HALT_LEDGERS is valid.
    let config = SafeOracleConfig {
        circuit_breaker_enabled: true,
        circuit_breaker_halt_ledgers: safe_oracle::MAX_CIRCUIT_BREAKER_HALT_LEDGERS,
        ..SafeOracleConfig::default()
    };
    assert!(
        config.validate().is_ok(),
        "halt_ledgers at exactly MAX_CIRCUIT_BREAKER_HALT_LEDGERS must be valid"
    );
}

// ===== AR.H L2: Cross-Source BPS Zero =====
//
// SafeOracleConfig::validate() rejects max_cross_source_bps == 0 when
// secondary is configured. Zero requires impossible perfect equality.

#[test]
fn test_validate_zero_cross_source_bps_rejected_when_secondary_configured() {
    let env = Env::default();
    let config = SafeOracleConfig {
        secondary_oracle: Some(Address::generate(&env)),
        max_cross_source_bps: 0,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidCrossSourceBps),
        "zero max_cross_source_bps with secondary configured = always-fires CrossSourceMismatch — must be rejected"
    );
}

#[test]
fn test_validate_zero_cross_source_bps_skipped_when_no_secondary() {
    // Conditional regression: if secondary is None, max_cross_source_bps is dormant.
    let config = SafeOracleConfig {
        secondary_oracle: None,
        max_cross_source_bps: 0, // dormant
        ..SafeOracleConfig::default()
    };
    assert!(
        config.validate().is_ok(),
        "max_cross_source_bps validation skipped when secondary_oracle is None"
    );
}

// ============================================================
// Phase 7.2 — previous_max_staleness_seconds validation
// ============================================================

/// `previous_max_staleness_seconds == 0` silently disables the
/// previous-price freshness check (every previous price would be
/// classified `StaleData`, blocking every borrow). Validator must reject.
#[test]
fn test_validate_zero_previous_staleness_rejected() {
    let config = SafeOracleConfig {
        previous_max_staleness_seconds: 0,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidPreviousStalenessSeconds),
        "zero previous_max_staleness_seconds blocks every borrow — must be rejected"
    );
}

/// `previous_max_staleness_seconds > 86_400` (24h) accepts unsafe
/// staleness for the deviation reference. Validator must reject — same
/// upper bound as `max_staleness_seconds`.
#[test]
fn test_validate_excessive_previous_staleness_rejected() {
    let config = SafeOracleConfig {
        previous_max_staleness_seconds: 86_401,
        ..SafeOracleConfig::default()
    };
    assert_eq!(
        config.validate(),
        Err(ConfigError::InvalidPreviousStalenessSeconds),
        "previous_max_staleness_seconds > 24h is unsafe — must be rejected"
    );
}
