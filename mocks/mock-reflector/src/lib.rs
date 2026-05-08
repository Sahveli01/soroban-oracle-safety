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
    PriceHistory(Asset),
}

/// Phase 7.1: TTL extension constants for price persistence.
///
/// Mirrors the production-style sizing applied in `liquidity-registry`
/// (see that crate's `SNAPSHOT_TTL_*` doc-comment for the rationale).
/// The mock's `set_price` is called by tests at arbitrary cadence, so
/// reusing the same 24h baseline keeps mock and real contracts behaviorally
/// equivalent under deploy-style harnesses.
const PRICE_TTL_MIN: u32 = 100;
const PRICE_TTL_EXTEND: u32 = 17_280;

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
        let price_key = DataKey::Price(asset);
        let price: Option<PriceData> = env.storage().persistent().get(&price_key);
        if price.is_some() {
            // Phase 7.1: defensive extend on read — keeps the entry alive
            // even when set_price is infrequent (test fixture mode).
            env.storage()
                .persistent()
                .extend_ttl(&price_key, PRICE_TTL_MIN, PRICE_TTL_EXTEND);
        }
        price
    }

    /// Returns up to `records` most recent prices for `asset`, newest first.
    /// Mirrors Reflector mainnet's `lastprices(asset, records)` shape.
    pub fn lastprices(env: Env, asset: Asset, records: u32) -> Option<Vec<PriceData>> {
        let history_key = DataKey::PriceHistory(asset);
        let history: Option<Vec<PriceData>> = env.storage().persistent().get(&history_key);
        if history.is_some() {
            // Phase 7.1: defensive extend on read for the history vector.
            env.storage()
                .persistent()
                .extend_ttl(&history_key, PRICE_TTL_MIN, PRICE_TTL_EXTEND);
        }

        match history {
            None => None,
            Some(prices) => {
                let len = prices.len();
                if len == 0 {
                    return None;
                }
                let take = records.min(len);
                let mut result = Vec::new(&env);
                for i in 0..take {
                    let idx = len - 1 - i;
                    result.push_back(prices.get(idx).unwrap());
                }
                Some(result)
            }
        }
    }

    /// TEST-ONLY: Mock-specific function for injecting prices in tests.
    /// No admin auth check — real Reflector uses multisig for price updates.
    /// Note: Real Reflector enforces `timestamp <= current ledger timestamp`.
    /// This mock does not enforce that invariant — tests can inject future timestamps if needed.
    pub fn set_price(env: Env, asset: Asset, price: i128, timestamp: u64) {
        let data = PriceData { price, timestamp };
        let price_key = DataKey::Price(asset.clone());
        env.storage().persistent().set(&price_key, &data);

        let history_key = DataKey::PriceHistory(asset);
        let mut history: Vec<PriceData> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(&env));
        history.push_back(data);
        env.storage().persistent().set(&history_key, &history);
        // Phase 7.1: extend TTL on both Price and PriceHistory entries.
        env.storage()
            .persistent()
            .extend_ttl(&price_key, PRICE_TTL_MIN, PRICE_TTL_EXTEND);
        env.storage()
            .persistent()
            .extend_ttl(&history_key, PRICE_TTL_MIN, PRICE_TTL_EXTEND);
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

    #[test]
    fn test_lastprices_returns_none_when_no_history() {
        let (env, client) = setup();
        let asset = Asset::Other(Symbol::new(&env, "BTC"));

        let result = client.lastprices(&asset, &2);
        assert!(result.is_none());
    }

    #[test]
    fn test_lastprices_returns_history_in_reverse_order() {
        let (env, client) = setup();
        let asset = Asset::Other(Symbol::new(&env, "XLM"));

        client.set_price(&asset, &100, &1000);
        client.set_price(&asset, &200, &2000);
        client.set_price(&asset, &300, &3000);

        let result = client.lastprices(&asset, &2).unwrap();
        assert_eq!(result.len(), 2);

        assert_eq!(result.get(0).unwrap().price, 300);
        assert_eq!(result.get(0).unwrap().timestamp, 3000);
        assert_eq!(result.get(1).unwrap().price, 200);
        assert_eq!(result.get(1).unwrap().timestamp, 2000);
    }

    #[test]
    fn test_lastprices_respects_records_limit() {
        let (env, client) = setup();
        let asset = Asset::Other(Symbol::new(&env, "ETH"));

        for i in 1u32..=5 {
            client.set_price(&asset, &(i128::from(i) * 100), &(u64::from(i) * 1000));
        }

        let result = client.lastprices(&asset, &3).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result.get(0).unwrap().timestamp, 5000);
        assert_eq!(result.get(2).unwrap().timestamp, 3000);
    }
}
