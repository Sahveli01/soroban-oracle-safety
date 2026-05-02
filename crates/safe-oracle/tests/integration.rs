//! Integration tests for `safe_oracle`.
//!
//! These tests use the `test-utils` crate (which itself depends on `safe-oracle`),
//! so they must live in the integration test directory rather than `lib.rs`'s
//! `mod test`. Inline unit tests would force `safe-oracle` to be compiled twice
//! (once as a normal dep of `test-utils`, once as a test target), and Rust would
//! treat the two builds as different crates — every shared type would mismatch.
//! Integration tests in `tests/` see `safe-oracle` as a single normal dependency,
//! which matches `test-utils`' view — types unify, and the cycle disappears.

use safe_oracle::{lastprice, Asset, OracleSafetyViolation};
use soroban_sdk::Symbol;
use test_utils::TestEnv;

/// Happy path: mock-reflector'a fiyat enjekte ettikten sonra gerçek
/// cross-contract call ile `lastprice` çağrısı `Ok(PriceData)` dönmeli.
/// Layer 1 guardrail'leri Phase 2.3-2.5'te yazılana dek hâlâ `Ok(())` —
/// dolayısıyla relaxed_config burada yetiyor.
#[test]
fn test_lastprice_with_real_reflector_call() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, 1_000_000_000_000_000_000, 12345);

    let config = TestEnv::relaxed_config();
    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address, // dummy registry — Phase 4'te gerçek olacak
        &config,
    );

    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    let price_data = result.unwrap();
    assert_eq!(price_data.price, 1_000_000_000_000_000_000);
    assert_eq!(price_data.timestamp, 12345);
}

/// Reflector hiç fiyat tutmuyorsa `lastprice` `None` döner;
/// `fetch_reflector_price` bunu fail-safe `Err(StaleData)`'e map eder.
#[test]
fn test_lastprice_returns_stale_data_when_reflector_has_no_price() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC")); // hiç fiyat set edilmedi
    let config = TestEnv::relaxed_config();

    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address,
        &config,
    );

    assert_eq!(result, Err(OracleSafetyViolation::StaleData));
}
