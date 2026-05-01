#![no_std]

use safe_oracle::Asset;
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeConfig {
    Some((Address, i128)),
    None,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigData {
    pub admin: Address,
    pub history_retention_period: u64,
    pub assets: Vec<Asset>,
    pub base_asset: Asset,
    pub decimals: u32,
    pub resolution: u32,
    pub cache_size: u32,
    pub fee_config: FeeConfig,
}

#[contracttype]
enum DataKey {
    Decimals,
    Resolution,
    Price(Asset),
}

#[contract]
pub struct MockReflector;

#[contractimpl]
impl MockReflector {
    /// Initializes the oracle. Mirrors Reflector's `config(env, ConfigData)`.
    /// MOCK NOTE: Only `decimals` and `resolution` are stored; other ConfigData
    /// fields (admin, assets, base_asset, history_retention_period, cache_size,
    /// fee_config) are accepted for interface fidelity and ignored.
    pub fn config(env: Env, config: ConfigData) {
        env.storage()
            .instance()
            .set(&DataKey::Decimals, &config.decimals);
        env.storage()
            .instance()
            .set(&DataKey::Resolution, &config.resolution);
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Decimals).unwrap()
    }

    pub fn resolution(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Resolution).unwrap()
    }

    pub fn lastprice(env: Env, asset: Asset) -> Option<PriceData> {
        env.storage().persistent().get(&DataKey::Price(asset))
        // TODO: extend_ttl in production
    }

    /// TEST-ONLY: Mock-specific function for injecting prices in tests.
    /// No admin auth check — real Reflector uses multisig for price updates.
    /// Note: Real Reflector enforces `timestamp <= current ledger timestamp`.
    /// This mock does not enforce that invariant — tests can inject future timestamps if needed.
    pub fn set_price(env: Env, asset: Asset, price: i128, timestamp: u64) {
        let data = PriceData { price, timestamp };
        env.storage()
            .persistent()
            .set(&DataKey::Price(asset), &data);
        // TODO: extend_ttl in production
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, vec, Env, Symbol};

    fn setup<'a>() -> (Env, MockReflectorClient<'a>) {
        let env = Env::default();
        let contract_id = env.register(MockReflector, ());
        let client = MockReflectorClient::new(&env, &contract_id);

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
        client.config(&cfg);
        (env, client)
    }

    #[test]
    fn test_set_and_get_price() {
        let (env, client) = setup();
        let asset = Asset::Other(Symbol::new(&env, "XLM"));
        client.set_price(&asset, &1_000_000_000_000, &12345);
        let result = client.lastprice(&asset);
        assert_eq!(
            result,
            Some(PriceData {
                price: 1_000_000_000_000,
                timestamp: 12345,
            })
        );
    }

    #[test]
    fn test_lastprice_returns_none_when_unset() {
        let (env, client) = setup();
        let asset = Asset::Other(Symbol::new(&env, "BTC"));
        assert_eq!(client.lastprice(&asset), None);
    }

    #[test]
    fn test_set_price_overwrites_previous() {
        let (env, client) = setup();
        let asset = Asset::Other(Symbol::new(&env, "ETH"));
        client.set_price(&asset, &100, &1000);
        client.set_price(&asset, &200, &2000);
        let result = client.lastprice(&asset);
        assert_eq!(
            result,
            Some(PriceData {
                price: 200,
                timestamp: 2000,
            })
        );
    }

    #[test]
    fn test_decimals_and_resolution() {
        let (_env, client) = setup();
        assert_eq!(client.decimals(), 14);
        assert_eq!(client.resolution(), 300);
    }
}
