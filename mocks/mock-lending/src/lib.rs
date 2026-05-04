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

/// Result type for `borrow()` that allows safe_oracle's auto-halt to commit
/// even on guardrail violations.
///
/// # Why a custom enum instead of `Result<(), MockLendingError>`?
///
/// Soroban contract methods returning `Result::Err` cause **all storage
/// writes in the same invocation to roll back**, including writes inside
/// `safe_oracle::circuit_breaker::open_circuit_breaker()`. By returning
/// `BorrowOutcome::Failed(d)` (wrapped in `Ok` at the Soroban boundary), the
/// breaker write commits while still conveying the violation to the caller.
///
/// Pre-5.2.D empirically eliminated 8 alternative mechanisms (cross-contract
/// sub-invocation, self-invocation, storage-type variants, scheduled calls,
/// out-of-band events). Caller Ok-API is the only viable path.
///
/// # Migration from Phase 1-4 `Result<(), MockLendingError>`
///
/// ```ignore
/// // Before:
/// lending_client.borrow(&user, &asset, &amount)?;
///
/// // After:
/// lending_client.borrow(&user, &asset, &amount).into_result()?;
/// ```
///
/// # Audit notes
///
/// - The `u32` discriminant is a soroban-sdk 25.x workaround: the
///   `Arbitrary` derive on `#[contracttype]` enums under the test feature
///   does not compose with `#[contracterror]` payloads. Phase 6 debt #17
///   tracks restoring the typed `MockLendingError` payload when SDK
///   supports it. This pattern mirrors `safe_oracle::PriceResult::Err(u32)`.
/// - Discriminants 1..=7 mirror `MockLendingError` 1:1 (oracle violations),
///   100..=101 are init-related, 200 is collateral-related.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BorrowOutcome {
    /// Borrow successful, all guardrails passed.
    Ok,

    /// Borrow rejected; the `u32` is the `MockLendingError` discriminant.
    Failed(u32),
}

impl BorrowOutcome {
    pub fn is_ok(&self) -> bool {
        matches!(self, BorrowOutcome::Ok)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, BorrowOutcome::Failed(_))
    }

    /// Convert to standard Rust `Result` for ergonomic `?` operator usage.
    /// Re-hydrates the `u32` discriminant into the typed `MockLendingError`.
    /// Unknown discriminants panic — they cannot occur on a result produced
    /// by `borrow()` (which only emits 1..=7, 100..=101, 200), but the
    /// explicit panic guards against forged values reaching the shim.
    pub fn into_result(self) -> Result<(), MockLendingError> {
        match self {
            BorrowOutcome::Ok => Ok(()),
            BorrowOutcome::Failed(1) => Err(MockLendingError::ExcessiveDeviation),
            BorrowOutcome::Failed(2) => Err(MockLendingError::StaleData),
            BorrowOutcome::Failed(3) => Err(MockLendingError::CrossSourceMismatch),
            BorrowOutcome::Failed(4) => Err(MockLendingError::InsufficientLiquidity),
            BorrowOutcome::Failed(5) => Err(MockLendingError::ThinSampling),
            BorrowOutcome::Failed(6) => Err(MockLendingError::CircuitBreakerOpen),
            BorrowOutcome::Failed(7) => Err(MockLendingError::StaleSnapshot),
            BorrowOutcome::Failed(100) => Err(MockLendingError::NotInitialized),
            BorrowOutcome::Failed(101) => Err(MockLendingError::AlreadyInitialized),
            BorrowOutcome::Failed(200) => Err(MockLendingError::InsufficientCollateral),
            BorrowOutcome::Failed(d) => panic!(
                "BorrowOutcome::Failed discriminant {} not mapped — \
                 likely missing variant in MockLendingError or out-of-range value",
                d
            ),
        }
    }
}

impl From<Result<(), MockLendingError>> for BorrowOutcome {
    fn from(r: Result<(), MockLendingError>) -> Self {
        match r {
            Ok(()) => BorrowOutcome::Ok,
            Err(e) => BorrowOutcome::Failed(e as u32),
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

    /// Borrow against deposited collateral.
    ///
    /// # Phase 5.4 v2 — Ok-only API
    ///
    /// Returns `BorrowOutcome` (Ok variant always at the Soroban boundary)
    /// instead of `Result<(), MockLendingError>` so that `safe_oracle`'s
    /// auto-halt write inside `lastprice()` commits cleanly. Soroban rolls
    /// back all storage writes when a contract method returns `Result::Err`,
    /// which is why Phase 5.2 v1 (lib-level Result::Err return) and
    /// Phase 5.4 v1 (caller-level Result::Err return) both failed to
    /// persist breaker state. See `BorrowOutcome` for the full rationale.
    ///
    /// Inner closure pattern lets the body keep `?`-operator ergonomics for
    /// the storage gets and `lastprice()` call; `BorrowOutcome::from(...)`
    /// converts the inner `Result` to the Ok-only outer return at the very
    /// last step, after every potential write has already happened.
    pub fn borrow(env: Env, caller: Address, asset: Asset, amount: i128) -> BorrowOutcome {
        caller.require_auth();

        let inner = || -> Result<(), MockLendingError> {
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

            // Transparent passthrough preserves which guardrail tripped.
            let price_data =
                safe_oracle::lastprice(&env, &asset, &oracle, &registry, &config).into_result()?;

            Borrow {
                caller: caller.clone(),
                asset: asset.clone(),
                amount,
                price: price_data.price,
            }
            .publish(&env);

            Ok(())
        };

        BorrowOutcome::from(inner())
    }
}
