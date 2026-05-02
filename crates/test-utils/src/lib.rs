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

use liquidity_registry::{LiquidityRegistry, LiquidityRegistryClient, LiquiditySnapshot};
use mock_lending::{MockLending, MockLendingClient};
use mock_reflector::{ConfigData, FeeConfig, MockReflector, MockReflectorClient};
use safe_oracle::{Asset, SafeOracleConfig};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    vec, Address, Env, Symbol,
};

/// Test environment that bundles Env + registered mock contracts + helpers.
pub struct TestEnv<'a> {
    pub env: Env,
    pub reflector_address: Address,
    pub reflector_client: MockReflectorClient<'a>,
    /// Secondary Reflector instance for cross-source guardrail tests
    /// (Phase 2.5). A separate `MockReflector` registration with its own
    /// storage; clients opt in by setting `config.secondary_oracle =
    /// Some(secondary_reflector_address)`.
    pub secondary_reflector_address: Address,
    pub secondary_reflector_client: MockReflectorClient<'a>,
    pub lending_address: Address,
    pub lending_client: MockLendingClient<'a>,
    /// Real `LiquidityRegistry` registration. Phase 4's `check_liquidity` and
    /// `check_thin_sampling` will read from this instance; Phase 3.5 wires it
    /// into `TestEnv` so test setups don't reimplement registry boilerplate.
    pub registry: Address,
    pub registry_client: LiquidityRegistryClient<'a>,
    /// Admin of the `LiquidityRegistry` (separate from the mock-reflector and
    /// mock-lending admins). Tests that mutate the whitelist authenticate as
    /// this address; `mock_all_auths` makes the auth itself a no-op.
    pub admin: Address,
    /// A whitelisted attester ready to call `write_snapshot`. Convenience for
    /// the common case where a test just needs *some* authorized writer.
    pub attester: Address,
}

impl<'a> TestEnv<'a> {
    /// Creates a fresh test environment with mock contracts registered.
    /// Mock Reflector is initialized with decimals=14, resolution=300.
    /// Mock Lending is NOT initialized — caller must call lending_client.initialize() if needed.
    /// No default prices are set — each test injects its own price scenarios.
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Baseline ledger timestamp so staleness checks have a defined "now"
        // (Soroban's default is 0, which would flag every Phase 2 fixture as
        // a future-dated price). Individual tests can override per-call via
        // env.ledger().with_mut().
        env.ledger().with_mut(|li| {
            li.timestamp = 100_000;
        });

        // Primary mock Reflector + initialize with default config.
        let reflector_address = env.register(MockReflector, ());
        let reflector_client = MockReflectorClient::new(&env, &reflector_address);
        let reflector_admin = Address::generate(&env);
        let base_asset = Asset::Other(Symbol::new(&env, "USD"));
        let cfg = ConfigData {
            admin: reflector_admin.clone(),
            history_retention_period: 0,
            assets: vec![&env],
            base_asset: base_asset.clone(),
            decimals: 14,
            resolution: 300,
            cache_size: 10,
            fee_config: FeeConfig::None,
        };
        reflector_client.config(&cfg);

        // Secondary mock Reflector — separate registration, identical config.
        // Used opt-in by tests that exercise the cross-source guardrail.
        let secondary_reflector_address = env.register(MockReflector, ());
        let secondary_reflector_client =
            MockReflectorClient::new(&env, &secondary_reflector_address);
        let secondary_cfg = ConfigData {
            admin: reflector_admin,
            history_retention_period: 0,
            assets: vec![&env],
            base_asset,
            decimals: 14,
            resolution: 300,
            cache_size: 10,
            fee_config: FeeConfig::None,
        };
        secondary_reflector_client.config(&secondary_cfg);

        // Register mock Lending and initialize it so tests get a Phase 2.7
        // wired contract by default — `safe_oracle::lastprice` is invoked
        // through the real path (no stub). Registry placeholder is the
        // lending address itself; `check_liquidity` and `check_thin_sampling`
        // are still stubs returning `Ok(())`, so the placeholder is never
        // dereferenced. Phase 4 swaps in a real `LiquidityRegistry`.
        let lending_address = env.register(MockLending, ());
        let lending_client = MockLendingClient::new(&env, &lending_address);
        let lending_admin = Address::generate(&env);
        lending_client.initialize(
            &lending_admin,
            &reflector_address,
            &lending_address,
            &SafeOracleConfig::default(),
        );

        // Register `LiquidityRegistry` and prime it with an admin and a single
        // whitelisted attester. Phase 4 guardrails read from this instance, so
        // wiring it up here lets test authors call `write_snapshot` without
        // per-test setup. Mock-lending intentionally still keeps its own
        // address as the registry placeholder — Phase 4 swaps it in once the
        // read-side guardrails consume it.
        let registry = env.register(LiquidityRegistry, ());
        let registry_client = LiquidityRegistryClient::new(&env, &registry);
        let admin = Address::generate(&env);
        let attester = Address::generate(&env);
        registry_client.initialize(&admin);
        registry_client.add_attester(&attester);

        Self {
            env,
            reflector_address,
            reflector_client,
            secondary_reflector_address,
            secondary_reflector_client,
            lending_address,
            lending_client,
            registry,
            registry_client,
            admin,
            attester,
        }
    }

    /// Sets a price in the primary mock Reflector for a given asset.
    pub fn set_oracle_price(&self, asset: &Asset, price: i128, timestamp: u64) {
        self.reflector_client.set_price(asset, &price, &timestamp);
    }

    /// Sets a price in the secondary mock Reflector for cross-source tests.
    pub fn set_secondary_oracle_price(&self, asset: &Asset, price: i128, timestamp: u64) {
        self.secondary_reflector_client
            .set_price(asset, &price, &timestamp);
    }

    /// Write a snapshot through the default whitelisted attester. Convenience
    /// wrapper around `LiquidityRegistry::write_snapshot` for Phase 4 tests
    /// that just need *some* attestation present for an asset; the explicit
    /// timestamp lets callers exercise replay-protection edge cases.
    pub fn write_snapshot(
        &self,
        asset: &Address,
        volume_usd: i128,
        trades_1h: u32,
        timestamp: u64,
    ) {
        let snapshot = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: volume_usd,
            unique_trades_1h: trades_1h,
            timestamp,
            attester: self.attester.clone(),
        };
        self.registry_client
            .write_snapshot(&self.attester, &snapshot);
    }

    /// Write a snapshot stamped at the current ledger time. Use this when the
    /// test doesn't care about timestamp positioning relative to staleness
    /// thresholds — `write_snapshot` is the right call when it does.
    pub fn write_snapshot_now(&self, asset: &Address, volume_usd: i128, trades_1h: u32) {
        let now = self.env.ledger().timestamp();
        self.write_snapshot(asset, volume_usd, trades_1h, now);
    }

    /// Returns a test-friendly config with relaxed thresholds.
    /// Production defaults (SafeOracleConfig::default()) are too strict for many tests
    /// (e.g. min_liquidity_usd=$10,000 requires liquidity injection in every test).
    /// This helper provides a config where guardrails are easy to satisfy.
    pub fn relaxed_config() -> SafeOracleConfig {
        SafeOracleConfig {
            max_deviation_bps: 5000,
            max_staleness_seconds: 100_000,
            max_cross_source_bps: 2000,
            max_snapshot_age_seconds: 100_000,
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
        let _ = test_env.secondary_reflector_address;
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

    /// Primary and secondary Reflectors must be distinct on-chain instances —
    /// they share the contract type but have independent storage.
    #[test]
    fn test_new_creates_env_with_secondary_reflector() {
        let test_env = TestEnv::new();
        assert_ne!(
            test_env.reflector_address,
            test_env.secondary_reflector_address
        );
    }

    /// `set_secondary_oracle_price` must write to the secondary instance only,
    /// leaving the primary untouched.
    #[test]
    fn test_set_secondary_oracle_price_updates_secondary_reflector() {
        let test_env = TestEnv::new();
        let asset = Asset::Other(Symbol::new(&test_env.env, "USDC"));

        test_env.set_secondary_oracle_price(&asset, 100_000_000_000_000, 1000);

        let secondary_result = test_env.secondary_reflector_client.lastprice(&asset);
        assert!(secondary_result.is_some());
        assert_eq!(secondary_result.unwrap().price, 100_000_000_000_000);

        // Primary intentionally left empty — confirms storage isolation.
        let primary_result = test_env.reflector_client.lastprice(&asset);
        assert!(primary_result.is_none());
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
