#![no_std]

use safe_oracle::{Asset, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env,
};

#[contractevent]
#[derive(Clone, Debug)]
pub struct Borrow {
    #[topic]
    pub caller: Address,
    pub asset: Asset,
    pub amount: i128,
    pub price: i128,
}

/// Error type returned by `MockLending::borrow`.
///
/// Discriminants 1–7 mirror `safe_oracle::OracleSafetyViolation` exactly so
/// the `borrow` flow can transparently propagate which guardrail tripped —
/// audit logs and client-side error handling preserve guardrail granularity
/// rather than collapsing every oracle failure into a single bucket.
/// Discriminants 100+ are mock-lending-specific (no oracle equivalent).
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MockLendingError {
    ExcessiveDeviation = 1,
    StaleData = 2,
    CrossSourceMismatch = 3,
    InsufficientLiquidity = 4,
    ThinSampling = 5,
    CircuitBreakerOpen = 6,
    StaleSnapshot = 7,
    NotInitialized = 100,
    AlreadyInitialized = 101,
    InsufficientCollateral = 200,
}

impl From<OracleSafetyViolation> for MockLendingError {
    fn from(v: OracleSafetyViolation) -> Self {
        match v {
            OracleSafetyViolation::ExcessiveDeviation => MockLendingError::ExcessiveDeviation,
            OracleSafetyViolation::StaleData => MockLendingError::StaleData,
            OracleSafetyViolation::CrossSourceMismatch => MockLendingError::CrossSourceMismatch,
            OracleSafetyViolation::InsufficientLiquidity => MockLendingError::InsufficientLiquidity,
            OracleSafetyViolation::ThinSampling => MockLendingError::ThinSampling,
            OracleSafetyViolation::CircuitBreakerOpen => MockLendingError::CircuitBreakerOpen,
            OracleSafetyViolation::StaleSnapshot => MockLendingError::StaleSnapshot,
        }
    }
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
    /// Initialize the mock lending contract.
    ///
    /// Reinitialization is rejected to prevent admin-override attacks: once
    /// `Admin` is in instance storage, a second call returns
    /// `AlreadyInitialized` instead of silently overwriting the oracle,
    /// registry, or config addresses. Pattern mirrors `LiquidityRegistry`
    /// (Phase 3.1) and is mandatory for all `initialize()` functions in this
    /// project (see CLAUDE.md).
    pub fn initialize(
        env: Env,
        admin: Address,
        oracle: Address,
        liquidity_registry: Address,
        config: SafeOracleConfig,
    ) -> Result<(), MockLendingError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(MockLendingError::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage()
            .instance()
            .set(&DataKey::Registry, &liquidity_registry);
        env.storage().instance().set(&DataKey::Config, &config);
        // TODO: extend_ttl in production

        Ok(())
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
    ) -> Result<(), MockLendingError> {
        caller.require_auth();

        let oracle: Address = env
            .storage()
            .instance()
            .get(&DataKey::Oracle)
            .ok_or(MockLendingError::NotInitialized)?;
        let registry: Address = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .ok_or(MockLendingError::NotInitialized)?;
        let config: SafeOracleConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(MockLendingError::NotInitialized)?;

        // Layer 1 guardrails — transparent passthrough preserves which
        // guardrail tripped (caller can match on `MockLendingError`).
        let price_data =
            safe_oracle::lastprice(&env, &asset, &oracle, &registry, &config).into_result()?;

        Borrow {
            caller,
            asset,
            amount,
            price: price_data.price,
        }
        .publish(&env);

        Ok(())
    }
}
