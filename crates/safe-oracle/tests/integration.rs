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
use soroban_sdk::{testutils::Ledger as _, Symbol};
use test_utils::TestEnv;

/// Happy path: mock-reflector'a iki fiyat enjekte ettikten sonra gerçek
/// cross-contract call ile `lastprice` çağrısı `Ok(PriceData)` dönmeli.
/// Phase 2.3b sonrası `check_deviation` 2 kayıt ister; aralarındaki sapma
/// relaxed_config eşiği altında olduğu için Layer 1 geçer.
#[test]
fn test_lastprice_with_real_reflector_call() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    // 14-decimal precision: $1.00 → $1.01 (1 % change, well under relaxed cap)
    test_env.set_oracle_price(&asset, 100_000_000_000_000, 12000);
    test_env.set_oracle_price(&asset, 101_000_000_000_000, 12345);

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
    assert_eq!(price_data.price, 101_000_000_000_000);
    assert_eq!(price_data.timestamp, 12345);
}

/// Reflector hiç fiyat tutmuyorsa `lastprices` `None` döner;
/// `fetch_reflector_prices` bunu fail-safe `Err(StaleData)`'e map eder.
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

/// 14-decimal helper: dollars → Reflector-scale price (×10^14).
const ONE_DOLLAR: i128 = 100_000_000_000_000;

/// %5 değişim relaxed_config (max=5000 BPS) altında kalır → Ok.
#[test]
fn test_deviation_passes_with_small_change() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    // $1.00 → $1.05 (5 % change = 500 BPS)
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 1000);
    test_env.set_oracle_price(&asset, ONE_DOLLAR + ONE_DOLLAR / 20, 1300);

    let config = TestEnv::relaxed_config(); // max_deviation_bps = 5000
    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address,
        &config,
    );

    assert!(
        result.is_ok(),
        "expected Ok for 5% change, got {:?}",
        result
    );
}

/// %25 değişim strict_config (max=2000 BPS) eşiğini aşıyor → ExcessiveDeviation.
#[test]
fn test_deviation_fails_at_threshold_breach() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    // $100 → $125 (25 % change = 2500 BPS)
    test_env.set_oracle_price(&asset, 100 * ONE_DOLLAR, 1000);
    test_env.set_oracle_price(&asset, 125 * ONE_DOLLAR, 1300);

    let config = TestEnv::strict_config(); // max_deviation_bps = 2000
    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address,
        &config,
    );

    assert_eq!(result, Err(OracleSafetyViolation::ExcessiveDeviation));
}

/// Tam %20 değişim sınır değerinde — `>` kullandığımız için Ok döner
/// (eşit olmak fail-trigger değil; sadece *aşan* sapma reddedilir).
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
    test_env.set_oracle_price(&asset, 100 * ONE_DOLLAR, 1000);
    test_env.set_oracle_price(&asset, 120 * ONE_DOLLAR, 1300);

    let config = TestEnv::strict_config(); // max_deviation_bps = 2000
    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address,
        &config,
    );

    assert!(
        result.is_ok(),
        "expected Ok at exact threshold (2000 BPS == max), got {:?}",
        result
    );
}

/// YieldBlox-sınıfı saldırı simülasyonu: ince SDEX pazarında küçük bir trade
/// ile $1.05 → $106 fiyat şişirme. Strict guardrail bunu reddetmeli; pitch
/// slide'da "bu test geçince proje çalışıyor" demeli.
#[test]
fn test_deviation_yieldblox_attack_simulation() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USTRY"));

    // Baseline: $1.05 — sonra saldırgan SDEX'te ~$5 trade ile $106'ya pumpluyor.
    test_env.set_oracle_price(&asset, ONE_DOLLAR + ONE_DOLLAR / 20, 1000);
    test_env.set_oracle_price(&asset, 106 * ONE_DOLLAR, 1300);

    let config = TestEnv::strict_config();
    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address,
        &config,
    );

    assert_eq!(
        result,
        Err(OracleSafetyViolation::ExcessiveDeviation),
        "YieldBlox-class attack must be blocked by deviation guardrail"
    );
}

/// Tek fiyat varsa deviation karşılaştırması yapılamaz —
/// `fetch_reflector_prices(records=2)` `len < records` görüp StaleData döner.
#[test]
fn test_deviation_fails_when_only_one_price_in_history() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.set_oracle_price(&asset, ONE_DOLLAR, 1000);
    // intentionally only one price — deviation needs two

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

/// Önceki fiyat 0 ise paydaya bölmeden önce manipülasyon sinyali olarak
/// ExcessiveDeviation döneriz (current pozitif, sıfır → herhangi BPS = ∞).
#[test]
fn test_deviation_fails_when_previous_price_is_zero() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "WEIRD"));

    test_env.set_oracle_price(&asset, 0, 1000); // zero baseline (manipulation signal)
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 1300);

    let config = TestEnv::relaxed_config();
    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address,
        &config,
    );

    assert_eq!(result, Err(OracleSafetyViolation::ExcessiveDeviation));
}

/// Fiyat 100 saniye eski, relaxed_config 100_000 saniye toleranslı → Ok.
#[test]
fn test_staleness_passes_when_data_is_fresh() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 5000;
    });

    test_env.set_oracle_price(&asset, ONE_DOLLAR, 4800); // 200s eski
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 4900); // 100s eski (current)

    let config = TestEnv::relaxed_config();
    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address,
        &config,
    );

    assert!(
        result.is_ok(),
        "expected Ok for 100s stale data, got {:?}",
        result
    );
}

/// 4000 saniye eski fiyat strict_config (300s tolerance) altında StaleData döner.
#[test]
fn test_staleness_fails_when_data_too_old() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "ETH"));

    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 5000;
    });

    test_env.set_oracle_price(&asset, ONE_DOLLAR * 100, 800); // 4200s eski
    test_env.set_oracle_price(&asset, ONE_DOLLAR * 100, 1000); // 4000s eski (current)

    let config = TestEnv::strict_config(); // max_staleness_seconds = 300
    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address,
        &config,
    );

    assert_eq!(result, Err(OracleSafetyViolation::StaleData));
}

/// Future timestamp (current.timestamp > now) clock skew veya feed manipulation
/// sinyalidir; defensive future-check StaleData döner.
#[test]
fn test_staleness_fails_with_future_timestamp() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "BTC"));

    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 5000;
    });

    test_env.set_oracle_price(&asset, ONE_DOLLAR, 5500); // 1 önceki (de future)
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 6000); // future (current)

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

/// Tam strict eşiği (300s) üzerinde — `>` kullandığımız için Ok döner
/// (eşit pass; sadece *aşan* yaş reddedilir). check_deviation ile tutarlı.
#[test]
fn test_staleness_passes_at_exact_threshold() {
    let test_env = TestEnv::new();
    let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

    test_env.env.ledger().with_mut(|li| {
        li.timestamp = 5000;
    });

    test_env.set_oracle_price(&asset, ONE_DOLLAR, 4500); // 500s eski
    test_env.set_oracle_price(&asset, ONE_DOLLAR, 4700); // exactly 300s eski

    let config = TestEnv::strict_config(); // max_staleness_seconds = 300
    let result = lastprice(
        &test_env.env,
        &asset,
        &test_env.reflector_address,
        &test_env.lending_address,
        &config,
    );

    assert!(
        result.is_ok(),
        "expected Ok at exact threshold (300s == max), got {:?}",
        result
    );
}
