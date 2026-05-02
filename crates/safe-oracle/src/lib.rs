#![no_std]

use soroban_sdk::{contracterror, contracttype, Address, Symbol};

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
    pub max_staleness_ledgers: u32,
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
            max_staleness_ledgers: 60,
            max_cross_source_bps: 500,
            min_liquidity_usd: 100_000_000_000,
            min_trade_count_1h: 5,
            secondary_oracle: None,
            circuit_breaker_enabled: false,
            circuit_breaker_halt_ledgers: 720,
        }
    }
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
        assert_eq!(cfg.max_staleness_ledgers, 60);
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
}
