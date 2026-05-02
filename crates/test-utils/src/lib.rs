//! Test utilities for the soroban-oracle-safety project.
//!
//! Used by Phase 2-8 test suites. NOT a production crate — only used in tests.
//!
//! Usage in another crate's tests:
//! ```ignore
//! use test_utils::TestEnv;
//! let env = TestEnv::new();
//! env.set_oracle_price(&asset, 100_000_000, 12345);
//! ```

use mock_lending::{MockLending, MockLendingClient};
use mock_reflector::{ConfigData, FeeConfig, MockReflector, MockReflectorClient};
use safe_oracle::{Asset, SafeOracleConfig};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, Symbol};

/// Test environment that bundles Env + registered mock contracts + helpers.
pub struct TestEnv<'a> {
    pub env: Env,
    pub reflector_address: Address,
    pub reflector_client: MockReflectorClient<'a>,
    pub lending_address: Address,
    pub lending_client: MockLendingClient<'a>,
    // NOTE: liquidity_registry will be added in Phase 3 when the contract is implemented.
}

impl<'a> TestEnv<'a> {
    /// Creates a fresh test environment with mock contracts registered.
    /// Mock Reflector is initialized with decimals=14, resolution=300.
    /// Mock Lending is NOT initialized — caller must call lending_client.initialize() if needed.
    /// No default prices are set — each test injects its own price scenarios.
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Register mock Reflector + initialize with default config
        let reflector_address = env.register(MockReflector, ());
        let reflector_client = MockReflectorClient::new(&env, &reflector_address);
        let admin = Address::generate(&env);
        let base_asset = Asset::Other(Symbol::new(&env, "USD"));
        let cfg = ConfigData {
            admin,
            history_retention_period: 0,
            assets: vec![&env],
            base_asset,
            decimals: 14,
            resolution: 300,
            cache_size: 10,
            fee_config: FeeConfig::None,
        };
        reflector_client.config(&cfg);

        // Register mock Lending (not initialized — caller's responsibility)
        let lending_address = env.register(MockLending, ());
        let lending_client = MockLendingClient::new(&env, &lending_address);

        Self {
            env,
            reflector_address,
            reflector_client,
            lending_address,
            lending_client,
        }
    }

    /// Sets a price in the mock Reflector for a given asset.
    pub fn set_oracle_price(&self, asset: &Asset, price: i128, timestamp: u64) {
        self.reflector_client.set_price(asset, &price, &timestamp);
    }

    /// Returns a test-friendly config with relaxed thresholds.
    /// Production defaults (SafeOracleConfig::default()) are too strict for many tests
    /// (e.g. min_liquidity_usd=$10,000 requires liquidity injection in every test).
    /// This helper provides a config where guardrails are easy to satisfy.
    pub fn relaxed_config() -> SafeOracleConfig {
        SafeOracleConfig {
            max_deviation_bps: 5000,
            max_staleness_seconds: 5000,
            max_cross_source_bps: 2000,
            min_liquidity_usd: 1,
            min_trade_count_1h: 1,
            secondary_oracle: None,
            circuit_breaker_enabled: false,
            circuit_breaker_halt_ledgers: 720,
        }
    }

    /// Returns the production default config (passes through to SafeOracleConfig::default()).
    pub fn strict_config() -> SafeOracleConfig {
        SafeOracleConfig::default()
    }
}

impl<'a> Default for TestEnv<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new_creates_env_with_registered_contracts() {
        let test_env = TestEnv::new();
        let _ = test_env.reflector_address;
        let _ = test_env.lending_address;
    }

    #[test]
    fn test_set_oracle_price_updates_reflector() {
        let test_env = TestEnv::new();
        let asset = Asset::Other(Symbol::new(&test_env.env, "XLM"));
        test_env.set_oracle_price(&asset, 1_000_000, 12345);

        let result = test_env.reflector_client.lastprice(&asset);
        assert!(result.is_some());
        let price_data = result.unwrap();
        assert_eq!(price_data.price, 1_000_000);
        assert_eq!(price_data.timestamp, 12345);
    }

    #[test]
    fn test_relaxed_config_is_more_permissive_than_strict() {
        let relaxed = TestEnv::relaxed_config();
        let strict = TestEnv::strict_config();

        assert!(relaxed.max_deviation_bps > strict.max_deviation_bps);
        assert!(relaxed.max_staleness_seconds > strict.max_staleness_seconds);
        assert!(relaxed.min_liquidity_usd < strict.min_liquidity_usd);
        assert!(relaxed.min_trade_count_1h <= strict.min_trade_count_1h);
    }
}
