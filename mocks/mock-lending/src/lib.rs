#![no_std]

use safe_oracle::{stub, Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env};

#[contractevent]
#[derive(Clone, Debug)]
pub struct Borrow {
    #[topic]
    pub caller: Address,
    pub asset: Asset,
    pub amount: i128,
    pub price: i128,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Oracle,
    Registry,
    Config,
    Deposit(Address, Asset),
}

#[contract]
pub struct MockLending;

#[contractimpl]
impl MockLending {
    pub fn initialize(
        env: Env,
        admin: Address,
        oracle: Address,
        liquidity_registry: Address,
        config: SafeOracleConfig,
    ) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage()
            .instance()
            .set(&DataKey::Registry, &liquidity_registry);
        env.storage().instance().set(&DataKey::Config, &config);
        // TODO: extend_ttl in production
    }

    pub fn deposit(env: Env, caller: Address, asset: Asset, amount: i128) {
        caller.require_auth();
        let key = DataKey::Deposit(caller, asset);
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(current + amount));
        // TODO: extend_ttl in production
    }

    pub fn borrow(
        env: Env,
        caller: Address,
        asset: Asset,
        amount: i128,
    ) -> Result<(), OracleSafetyViolation> {
        caller.require_auth();

        let oracle: Address = env.storage().instance().get(&DataKey::Oracle).unwrap();
        let registry: Address = env.storage().instance().get(&DataKey::Registry).unwrap();
        let config: SafeOracleConfig = env.storage().instance().get(&DataKey::Config).unwrap();

        // Phase 2'de gerçek `safe_oracle::lastprice` ile değiştirilecek — imza aynı kalır.
        let price = stub::lastprice(&env, &asset, &oracle, &registry, &config)?;

        Borrow {
            caller,
            asset,
            amount,
            price,
        }
        .publish(&env);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, testutils::Events as _, Symbol};

    fn fresh_env() -> (Env, Address, MockLendingClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(MockLending, ());
        let client = MockLendingClient::new(&env, &contract_id);
        (env, contract_id, client)
    }

    fn init(env: &Env, client: &MockLendingClient<'_>) -> (Address, Address, Address) {
        let admin = Address::generate(env);
        let oracle = Address::generate(env);
        let registry = Address::generate(env);
        let config = SafeOracleConfig::default();
        client.initialize(&admin, &oracle, &registry, &config);
        (admin, oracle, registry)
    }

    #[test]
    fn test_initialize_sets_storage() {
        let (env, contract_id, client) = fresh_env();
        let (admin, oracle, registry) = init(&env, &client);

        env.as_contract(&contract_id, || {
            let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
            let stored_oracle: Address = env.storage().instance().get(&DataKey::Oracle).unwrap();
            let stored_registry: Address =
                env.storage().instance().get(&DataKey::Registry).unwrap();
            let _stored_config: SafeOracleConfig =
                env.storage().instance().get(&DataKey::Config).unwrap();

            assert_eq!(stored_admin, admin);
            assert_eq!(stored_oracle, oracle);
            assert_eq!(stored_registry, registry);
        });
    }

    #[test]
    fn test_borrow_succeeds_with_stub() {
        let (env, _contract_id, client) = fresh_env();
        init(&env, &client);

        let caller = Address::generate(&env);
        let asset = Asset::Other(Symbol::new(&env, "USDC"));

        client.borrow(&caller, &asset, &500);
    }

    #[test]
    fn test_borrow_emits_event() {
        let (env, _contract_id, client) = fresh_env();
        init(&env, &client);

        let caller = Address::generate(&env);
        let asset = Asset::Other(Symbol::new(&env, "XLM"));

        client.borrow(&caller, &asset, &1000);

        let events = env.events().all();
        assert_eq!(events.events().len(), 1);
    }

    #[test]
    fn test_deposit_records_amount() {
        let (env, contract_id, client) = fresh_env();

        let caller = Address::generate(&env);
        let asset = Asset::Other(Symbol::new(&env, "USDC"));

        client.deposit(&caller, &asset, &100);
        client.deposit(&caller, &asset, &50);

        env.as_contract(&contract_id, || {
            let total: i128 = env
                .storage()
                .persistent()
                .get(&DataKey::Deposit(caller.clone(), asset.clone()))
                .unwrap();
            assert_eq!(total, 150);
        });
    }
}
