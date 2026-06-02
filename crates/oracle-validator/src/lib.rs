#![no_std]
//! # oracle-validator
//!
//! A thin, **oracle-agnostic** read-only contract that runs `safe-oracle`'s
//! guardrail chain against any SEP-40 / Reflector-shaped oracle and returns a
//! structured [`ValidationResult`].
//!
//! The oracle address is a parameter, so the same validator serves both:
//!   - **Reflector** directly (`lastprice`/`prices`/`decimals` native), and
//!   - **Noeracle** via the `noeracle-adapter` (same surface, synthesized
//!     ring-buffer history).
//!
//! `validate` is intended to be called read-only via `simulateTransaction`
//! (no signature, no fee): it performs no storage writes of its own. It runs
//! with `SafeOracleConfig::default()` — Layer 1 only (`max_deviation_bps =
//! 2000`, `max_staleness_seconds = 300`, `layer2_enabled = false`,
//! `circuit_breaker_enabled = false`), so the `liquidity_registry` argument is
//! never queried and the oracle address is passed for it as a harmless
//! placeholder.
//!
//! `safe-oracle` v0.4.0 is a path dependency used as a library only — its
//! production code is not modified.

use safe_oracle::{lastprice, Asset, PriceResult, SafeOracleConfig};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

/// Outcome of a validation run.
///
/// `safe-oracle` short-circuits on the first failing guardrail, so:
///   - `approved = true`  → every Layer 1 guardrail passed; `price`/`timestamp`
///     hold the validated value, `violation = 0`.
///   - `approved = false` → `violation` is the `OracleSafetyViolation`
///     discriminant (1..=10) of the guardrail that rejected; the guardrails
///     evaluated before it passed. `price`/`timestamp` are 0.
///
/// Discriminants (see `safe_oracle::OracleSafetyViolation`):
/// 1 ExcessiveDeviation · 2 StaleData · 3 CrossSourceMismatch ·
/// 4 InsufficientLiquidity · 5 ThinSampling · 6 CircuitBreakerOpen ·
/// 7 StaleSnapshot · 8 ExternalContractFailure · 9 DecimalsMismatch ·
/// 10 UnexpectedDecimals.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationResult {
    pub approved: bool,
    pub price: i128,
    pub timestamp: u64,
    pub violation: u32,
}

#[contract]
pub struct OracleValidator;

#[contractimpl]
impl OracleValidator {
    /// Validate the latest price for `asset` from `oracle` through safe-oracle's
    /// Layer 1 guardrails. Read-only; safe to call via `simulateTransaction`.
    pub fn validate(env: Env, oracle: Address, asset: Asset) -> ValidationResult {
        let config = SafeOracleConfig::default();
        // `oracle` is passed twice: once as the price feed, once as the
        // (unused) liquidity_registry — default config has layer2_enabled =
        // false, so the registry is never queried.
        match lastprice(&env, &asset, &oracle, &oracle, &config) {
            PriceResult::Ok(pd) => ValidationResult {
                approved: true,
                price: pd.price,
                timestamp: pd.timestamp,
                violation: 0,
            },
            PriceResult::Err(discriminant) => ValidationResult {
                approved: false,
                price: 0,
                timestamp: 0,
                violation: discriminant,
            },
        }
    }
}

#[cfg(test)]
mod test;
