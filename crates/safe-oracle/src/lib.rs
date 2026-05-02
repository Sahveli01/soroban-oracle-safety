#![no_std]

use soroban_sdk::{contracterror, contracttype, Address, Env, Symbol, Vec};

mod reflector_client;
pub use reflector_client::ReflectorClient;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OracleSafetyViolation {
    ExcessiveDeviation = 1,
    StaleData = 2,
    CrossSourceMismatch = 3,
    InsufficientLiquidity = 4,
    ThinSampling = 5,
    CircuitBreakerOpen = 6,
    StaleSnapshot = 7,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SafeOracleConfig {
    pub max_deviation_bps: u32,
    pub max_staleness_seconds: u32,
    pub max_cross_source_bps: u32,
    pub min_liquidity_usd: i128,
    pub min_trade_count_1h: u32,
    pub secondary_oracle: Option<Address>,
    pub circuit_breaker_enabled: bool,
    pub circuit_breaker_halt_ledgers: u32,
}

impl Default for SafeOracleConfig {
    fn default() -> Self {
        Self {
            max_deviation_bps: 2000,
            max_staleness_seconds: 300,
            max_cross_source_bps: 500,
            min_liquidity_usd: 100_000_000_000,
            min_trade_count_1h: 5,
            secondary_oracle: None,
            circuit_breaker_enabled: false,
            circuit_breaker_halt_ledgers: 720,
        }
    }
}

/// Validates oracle output against five layered guardrails before returning a price.
///
/// This function is the public entry point of the `safe_oracle` library. Lending
/// protocols call this instead of `reflector.lastprice()` directly. Each guardrail
/// is a deterministic check that returns `Err` on violation, propagating to the
/// calling contract via `?`.
///
/// # Guardrails
/// - Layer 1 (Reflector-only): deviation, staleness, cross-source
/// - Layer 2 (LiquidityRegistry-required): liquidity threshold, thin sampling
/// - Optional: circuit breaker (Phase 5)
///
/// # Phase 2 Status
/// Skeleton — Layer 1 guardrails are scaffolded as stubs returning `Ok(())`.
/// Real guardrail logic arrives in prompts 2.2 (deviation), 2.3 (staleness),
/// 2.4 (multi-source). Layer 2 lands in Phase 4 alongside `LiquidityRegistry`.
pub fn lastprice(
    env: &Env,
    asset: &Asset,
    reflector: &Address,
    liquidity_registry: &Address,
    config: &SafeOracleConfig,
) -> Result<PriceData, OracleSafetyViolation> {
    // 1. Fetch the most recent price (records=1 — `check_deviation` will fetch
    //    the second-most-recent on its own). Single source of truth: every
    //    Reflector read goes through `fetch_reflector_prices`.
    let prices = fetch_reflector_prices(env, reflector, asset, 1)?;
    let current = prices.get(0).ok_or(OracleSafetyViolation::StaleData)?;

    // 2. Layer 1 guardrails (Reflector-only data)
    check_deviation(env, reflector, asset, &current, config)?;
    check_staleness(env, &current, config)?;
    check_cross_source(env, asset, &current, config)?;

    // 3. Layer 2 guardrails (require LiquidityRegistry)
    check_liquidity(env, liquidity_registry, asset, config)?;
    check_thin_sampling(env, liquidity_registry, asset, config)?;

    // 4. Circuit breaker check — Phase 5'te implement edilecek
    // check_circuit_breaker(env, asset)?;

    Ok(current)
}

/// Fetches the most recent `records` prices from Reflector via cross-contract call.
///
/// Returns prices ordered newest-first. Single source of truth for every
/// Reflector read: `lastprice` calls with `records=1`, `check_deviation` with
/// `records=2`. Reflector returns `None` when the asset has no recorded prices,
/// and a shorter `Vec` when history is thinner than `records`; both cases map
/// to `Err(StaleData)` here — fail-safe default that downstream guardrails can
/// rely on.
fn fetch_reflector_prices(
    env: &Env,
    reflector: &Address,
    asset: &Asset,
    records: u32,
) -> Result<Vec<PriceData>, OracleSafetyViolation> {
    let client = ReflectorClient::new(env, reflector);
    let prices = client
        .lastprices(asset, &records)
        .ok_or(OracleSafetyViolation::StaleData)?;

    if prices.len() < records {
        return Err(OracleSafetyViolation::StaleData);
    }

    Ok(prices)
}

/// Layer 1, Guardrail 1 — Maximum Deviation.
///
/// Compares the current price against the previous price recorded by Reflector
/// (one resolution-window earlier — typically ~5 min) and rejects updates whose
/// BPS deviation exceeds `config.max_deviation_bps`. This is the primary defense
/// against YieldBlox-class SDEX manipulation: an attacker who shifts the spot
/// price by buying/selling on a thin market produces a delta that this guardrail
/// flags as `ExcessiveDeviation`.
///
/// # Defensive logic
/// - `current.price <= 0` → `ExcessiveDeviation`. Reflector should never return a
///   non-positive price, but a corrupted or malicious feed is the threat model.
/// - Newest/oldest are determined by `timestamp`, not vec index — the mock
///   currently returns newest-first, but we don't make production code rely on it.
/// - Sanity-check that `current` matches the newest from the 2-record fetch.
///   Storage cannot mutate within a single transaction, so a mismatch implies
///   a feed bug → fail safe with `StaleData`.
/// - `previous.price <= 0` → `ExcessiveDeviation`. Same reasoning as current.
/// - `checked_mul(10_000)` catches the rare overflow where `abs_diff * 10_000`
///   would exceed `i128::MAX`; treating overflow as deviation is the safe default.
fn check_deviation(
    env: &Env,
    reflector: &Address,
    asset: &Asset,
    current: &PriceData,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    if current.price <= 0 {
        return Err(OracleSafetyViolation::ExcessiveDeviation);
    }

    let prices = fetch_reflector_prices(env, reflector, asset, 2)?;
    let p0 = prices.get(0).ok_or(OracleSafetyViolation::StaleData)?;
    let p1 = prices.get(1).ok_or(OracleSafetyViolation::StaleData)?;

    let (newest, oldest) = if p0.timestamp >= p1.timestamp {
        (p0, p1)
    } else {
        (p1, p0)
    };

    if current.timestamp != newest.timestamp || current.price != newest.price {
        return Err(OracleSafetyViolation::StaleData);
    }

    let previous = oldest;
    if previous.price <= 0 {
        return Err(OracleSafetyViolation::ExcessiveDeviation);
    }

    let abs_diff = (current.price - previous.price).abs();
    let scaled = abs_diff
        .checked_mul(10_000)
        .ok_or(OracleSafetyViolation::ExcessiveDeviation)?;
    let deviation_bps = scaled / previous.price;

    if deviation_bps > config.max_deviation_bps as i128 {
        return Err(OracleSafetyViolation::ExcessiveDeviation);
    }

    Ok(())
}

/// Phase 2.4'te implement edilecek (Layer 1, Guardrail 3 — Staleness Check).
///
/// Mantık (Phase 2.4):
/// - `env.ledger().timestamp()` ile mevcut Unix saniyesini al
/// - `current.timestamp` ile farkı hesapla (her ikisi de Unix saniye)
/// - Fark `config.max_staleness_seconds`'i aşıyorsa `Err(StaleData)`
fn check_staleness(
    _env: &Env,
    _current: &PriceData,
    _config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    Ok(())
}

/// Phase 2.4'te implement edilecek (Layer 1, Guardrail 4 — Multi-Source Cross-Check).
///
/// Mantık (Phase 2.4):
/// - `config.secondary_oracle` `None` ise skip
/// - `Some(addr)` ise ikincil oracle'dan fiyat çek
/// - İki fiyat arasındaki BPS sapma `config.max_cross_source_bps` üstündeyse
///   `Err(CrossSourceMismatch)`
fn check_cross_source(
    _env: &Env,
    _asset: &Asset,
    _current: &PriceData,
    _config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    Ok(())
}

/// Phase 4'te implement edilecek (Layer 2, Guardrail 2 — Minimum Liquidity).
///
/// `LiquidityRegistry` kontratından son 30 dakika USD hacmini okur ve
/// `config.min_liquidity_usd` altındaysa `Err(InsufficientLiquidity)` döner.
fn check_liquidity(
    _env: &Env,
    _liquidity_registry: &Address,
    _asset: &Asset,
    _config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    Ok(())
}

/// Phase 4'te implement edilecek (Layer 2, Guardrail 5 — Thin Sampling).
///
/// `LiquidityRegistry` kontratından son 1 saatin unique trade sayısını okur ve
/// `config.min_trade_count_1h` altındaysa `Err(ThinSampling)` döner.
fn check_thin_sampling(
    _env: &Env,
    _liquidity_registry: &Address,
    _asset: &Asset,
    _config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    Ok(())
}

/// STUB module — Phase 2'de gerçek implementasyonla değiştirilecek.
/// Public interface (`stub::lastprice` imzası) Phase 2'deki gerçek
/// `lastprice` ile birebir aynı kalmalı; tüketici kontratlar (mock-lending)
/// Phase 2'de tek satır değiştirmeden geçmeli.
pub mod stub {
    use super::{Address, Asset, OracleSafetyViolation, PriceData, SafeOracleConfig};
    use soroban_sdk::Env;

    /// STUB — Phase 2'de gerçek implementasyonla değiştirilecek.
    /// Şimdilik her zaman Ok(dummy_price_data) döner.
    /// Gerçek imza Phase 2'de bire bir aynı kalmalı.
    pub fn lastprice(
        _env: &Env,
        _asset: &Asset,
        _oracle: &Address,
        _registry: &Address,
        _config: &SafeOracleConfig,
    ) -> Result<PriceData, OracleSafetyViolation> {
        Ok(PriceData {
            price: 1_000_000_000_000_000_000, // dummy 1.0 with 18 decimals
            timestamp: 0,                     // dummy timestamp
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let cfg = SafeOracleConfig::default();
        assert_eq!(cfg.max_deviation_bps, 2000);
        assert_eq!(cfg.max_staleness_seconds, 300);
        assert_eq!(cfg.max_cross_source_bps, 500);
        assert_eq!(cfg.min_liquidity_usd, 100_000_000_000);
        assert_eq!(cfg.min_trade_count_1h, 5);
        assert!(cfg.secondary_oracle.is_none());
        assert!(!cfg.circuit_breaker_enabled);
        assert_eq!(cfg.circuit_breaker_halt_ledgers, 720);
    }

    #[test]
    fn test_error_variants_have_correct_discriminants() {
        assert_eq!(OracleSafetyViolation::ExcessiveDeviation as u32, 1);
        assert_eq!(OracleSafetyViolation::StaleData as u32, 2);
        assert_eq!(OracleSafetyViolation::CrossSourceMismatch as u32, 3);
        assert_eq!(OracleSafetyViolation::InsufficientLiquidity as u32, 4);
        assert_eq!(OracleSafetyViolation::ThinSampling as u32, 5);
        assert_eq!(OracleSafetyViolation::CircuitBreakerOpen as u32, 6);
        assert_eq!(OracleSafetyViolation::StaleSnapshot as u32, 7);
    }

    /// Compile-time guarantee: gerçek `lastprice` ile `stub::lastprice`
    /// birebir aynı imzaya sahip. Phase 2 sonunda mock-lending tek satır
    /// değişikliğiyle (stub::lastprice → safe_oracle::lastprice) geçecek.
    #[test]
    fn test_lastprice_signature_matches_stub() {
        let _real: fn(
            &Env,
            &Asset,
            &Address,
            &Address,
            &SafeOracleConfig,
        ) -> Result<PriceData, OracleSafetyViolation> = lastprice;
        let _stub: fn(
            &Env,
            &Asset,
            &Address,
            &Address,
            &SafeOracleConfig,
        ) -> Result<PriceData, OracleSafetyViolation> = stub::lastprice;
    }
}
