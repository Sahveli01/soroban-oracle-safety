//! Layer 1 combined-behavior integration tests.
//!
//! Bu dosya `safe_oracle::lastprice`'in tüm Layer 1 guardrail zincirinin
//! birlikte çalışmasını ve execution order'ı doğrular. Bireysel guardrail
//! testleri (deviation, staleness, cross-source — her biri 6/4/6 senaryo)
//! `tests/integration.rs`'de; bu dosya farklı: Layer 1'in kombine
//! davranışına ve hata önceliğine odaklanır.

use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::Symbol;
use test_utils::TestEnv;

/// 14-decimal helper: dollars → Reflector-scale price (×10^14).
const ONE_DOLLAR: i128 = 100_000_000_000_000;

/// Tüm Layer 1 guardrail'leri (deviation, staleness, cross-source) geçiyor → Ok.
/// Returned price newest entry'yi yansıtmalı.
#[test]
fn test_layer1_happy_path_all_guardrails_pass() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_950);
    test_env.set_secondary_oracle_price(&asset, ONE_DOLLAR, 99_950);

    let mut config = TestEnv::relaxed_config();
    config.secondary_oracle = Some(test_env.secondary_reflector_address.clone());

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "expected Ok when all guardrails pass, got {:?}",
        result
    );
    let price = result.unwrap();
    assert_eq!(price.price, ONE_DOLLAR);
    assert_eq!(price.timestamp, 99_950);
}

/// Hem deviation hem staleness fail durumunda deviation önce çalıştığı için
/// `ExcessiveDeviation` döner. Bu test execution order garantisini bozsa
/// failed assertion ile yakalar.
#[test]
fn test_layer1_execution_order_deviation_before_staleness() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    // ts=50_000/50_300 → ~50_000s elapsed (strict max_staleness=300 → stale)
    // prices: $100 → $200 = 10_000 BPS deviation (strict max=2000 → excessive)
    test_env.set_oracle_price(&asset, 100 * ONE_DOLLAR, 50_000);
    test_env.set_oracle_price(&asset, 200 * ONE_DOLLAR, 50_300);

    let config = TestEnv::strict_config();

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "deviation check should run before staleness — expected ExcessiveDeviation"
    );
}

/// Deviation pass (küçük), staleness fail (eski) → `StaleData`.
/// Pipeline'ın deviation'dan sonra staleness'e gerçekten geldiğini doğrular.
#[test]
fn test_layer1_execution_order_staleness_after_deviation_pass() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    // ts=50_000/50_300 → ~49_700s elapsed (strict max_staleness=300 → stale)
    // %1 deviation = 100 BPS (strict max=2000 → pass)
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 50_000);
    test_env.set_oracle_price(&asset, ONE_DOLLAR + ONE_DOLLAR / 100, 50_300);

    let config = TestEnv::strict_config();

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::StaleData),
        "deviation passed but staleness failed — expected StaleData"
    );
}

/// Deviation + staleness pass, cross-source fail → `CrossSourceMismatch`.
/// Pipeline'ın staleness'tan sonra cross-source'a geldiğini doğrular.
#[test]
fn test_layer1_execution_order_cross_source_after_staleness_pass() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    // Fresh (50s elapsed ≤ 300), 0 BPS deviation (≤ 2000)
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_900);
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_950);
    // Secondary $1.10 = 1000 BPS cross-source (strict max=500 → mismatch)
    test_env.set_secondary_oracle_price(&asset, ONE_DOLLAR * 110 / 100, 99_950);

    let mut config = TestEnv::strict_config();
    config.secondary_oracle = Some(test_env.secondary_reflector_address.clone());

    let result = test_env.lastprice(&asset, &config);

    assert_eq!(
        result,
        Err(OracleSafetyViolation::CrossSourceMismatch),
        "deviation + staleness passed but cross-source failed — expected CrossSourceMismatch"
    );
}

/// `SafeOracleConfig::default()` ile production-realistic senaryo:
/// %1 deviation, 150s eski, secondary yok → tüm guardrail'ler geçer.
/// Default değerlerin reasonable workflow'u kabul ettiğini doğrular.
#[test]
fn test_layer1_with_default_config_passes_normal_scenario() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    // baseline=100_000; ts=99_750/99_850 → 250/150s elapsed (default max=300 → ok)
    // %1 deviation = 100 BPS (default max=2000 → ok)
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 99_750);
    test_env.set_oracle_price(&asset, ONE_DOLLAR + ONE_DOLLAR / 100, 99_850);

    // Default: secondary_oracle = None → cross-source skip
    let config = SafeOracleConfig::default();

    let result = test_env.lastprice(&asset, &config);

    assert!(
        result.is_ok(),
        "default config should pass normal scenario, got {:?}",
        result
    );
}
