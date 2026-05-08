#![no_std]

use safe_oracle::{Asset, ConfigError, OracleSafetyViolation, SafeOracleConfig};
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, IntoVal,
};

/// Phase 7.1: TTL extension constants for mock-lending storage.
///
/// `INSTANCE_TTL_*` sizes the renewal for `instance` storage (admin, oracle,
/// registry, validated config). Instance entries change rarely, so the
/// strategy is "extend to the maximum on every initialize call" — gas paid
/// once at deploy buys ~31 days of guaranteed liveness.
///
/// `PERSISTENT_TTL_*` sizes the renewal for `persistent` storage (per-user
/// deposit balances). Persistent entries change every deposit, so the
/// strategy is "extend on each write" with a 24h baseline; users who deposit
/// then go silent still see their balance preserved across the extension
/// horizon (a few hundred deposits between extends in steady state).
const INSTANCE_TTL_MIN: u32 = 1_000;
const INSTANCE_TTL_EXTEND: u32 = 535_679;
const PERSISTENT_TTL_MIN: u32 = 100;
const PERSISTENT_TTL_EXTEND: u32 = 17_280;

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
    /// Mirror of `OracleSafetyViolation::ExternalContractFailure` (Hardening
    /// Phase debt #4). Surfaces when Reflector or `LiquidityRegistry`
    /// trapped during a cross-contract invocation; the lending integrator's
    /// caller sees the same granular reason safe-oracle reported.
    ExternalContractFailure = 8,
    NotInitialized = 100,
    /// Retained for audit-trail continuity; no longer reachable from
    /// any contract entry point after Hardening 6B's CAP-0058
    /// `__constructor` migration (debt #10) — constructors cannot be
    /// invoked twice, so the previous re-init guard was removed.
    #[allow(dead_code)]
    AlreadyInitialized = 101,
    /// Returned by `initialize` when `SafeOracleConfig::validate()` rejects
    /// the supplied config (Hardening Phase debt #2). Surfaces every
    /// `safe_oracle::ConfigError` variant under one lending-side
    /// discriminant — the granular reason is in the audit log emitted by
    /// the safe-oracle library.
    InvalidConfig = 102,
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
            OracleSafetyViolation::ExternalContractFailure => {
                MockLendingError::ExternalContractFailure
            }
        }
    }
}

/// All `safe_oracle::ConfigError` variants collapse to
/// [`MockLendingError::InvalidConfig`]. The lending-side surface only needs
/// to know that init was rejected; the precise validation reason
/// (`InvalidDeviationBps`, `InvalidStalenessSeconds`, etc.) belongs in the
/// safe-oracle audit log, not in the lending integrator's error enum.
impl From<ConfigError> for MockLendingError {
    fn from(_: ConfigError) -> Self {
        MockLendingError::InvalidConfig
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
            BorrowOutcome::Failed(8) => Err(MockLendingError::ExternalContractFailure),
            BorrowOutcome::Failed(100) => Err(MockLendingError::NotInitialized),
            BorrowOutcome::Failed(101) => Err(MockLendingError::AlreadyInitialized),
            BorrowOutcome::Failed(102) => Err(MockLendingError::InvalidConfig),
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
    /// Initialize the mock lending contract — CAP-0058 `__constructor`.
    ///
    /// # Hardening Phase debt #10 (CAP-0058 migration)
    ///
    /// Replaces the previous `pub fn initialize(...)` two-step deploy +
    /// init flow with the atomic CAP-0058 constructor pattern. Init args
    /// are now passed at deploy time (`env.register(MockLending, (admin,
    /// oracle, registry, config))`); the constructor runs as part of the
    /// same host invocation, eliminating the deploy/init race window
    /// where an attacker could sandwich operations between the two.
    ///
    /// The previous `if has(Admin) -> AlreadyInitialized` re-init guard is
    /// gone — a CAP-0058 constructor cannot be invoked twice on the same
    /// contract. The `MockLendingError::AlreadyInitialized` variant is
    /// retained for audit-history continuity but is no longer reachable
    /// from this contract.
    ///
    /// # Config validation (Hardening Phase debt #2)
    ///
    /// `config.validate()` still runs before any storage write. With the
    /// constructor model an `InvalidConfig` Err traps the registration
    /// itself (the contract is never created), so misconfigured deploys
    /// fail visibly at the host level rather than landing storage.
    pub fn __constructor(
        env: Env,
        admin: Address,
        oracle: Address,
        liquidity_registry: Address,
        config: SafeOracleConfig,
    ) -> Result<(), MockLendingError> {
        admin.require_auth();

        // Hardening debt #2: reject misconfigured deploys before persisting
        // anything. Validation is opt-in at the library layer; the lending
        // integrator opts in here on behalf of its caller.
        config.validate().map_err(MockLendingError::from)?;

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage()
            .instance()
            .set(&DataKey::Registry, &liquidity_registry);
        env.storage().instance().set(&DataKey::Config, &config);
        // Phase 7.1: extend instance TTL — config rarely changes, so push
        // to the maximum (~31 days) on each constructor invocation.
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_MIN, INSTANCE_TTL_EXTEND);

        Ok(())
    }

    pub fn deposit(env: Env, caller: Address, asset: Asset, amount: i128) {
        caller.require_auth();
        let key = DataKey::Deposit(caller, asset);
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(current + amount));
        // Phase 7.1: extend persistent TTL on every deposit — keeps user
        // balances alive between deposits (24h baseline).
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL_MIN, PERSISTENT_TTL_EXTEND);
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
        // Hardening Phase debt #11: granular auth — `caller`'s signature is
        // bound to the exact `(asset, amount)` pair this borrow is for.
        // A captured signature for "borrow 1000 USDC" cannot be replayed
        // to borrow 10_000 USDC, or to borrow against a different asset.
        // Generic `require_auth()` would approve any args under the same
        // signature; that is too coarse for high-value paths like borrow.
        caller.require_auth_for_args((asset.clone(), amount).into_val(&env));

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

    // ===== Test-only circuit-breaker primitive surface =====
    //
    // Hardening Phase debts #18 + #20 (merged): Phase 5.5 had two harness
    // contracts in `test-utils` — `OracleHost` (drove `lastprice`) and
    // `TestHost` (drove the breaker primitives). They had separate
    // `instance()` storage, so a manual `close_circuit_breaker` call routed
    // through `TestHost` could not reset the auto-halt state that
    // `lastprice` (via `borrow`) had committed to `OracleHost`'s storage.
    // The Phase 5.5 `test_manual_close_resets_open_breaker_state` worked
    // around this with a ledger-advance + auto-recovery hack; the test
    // documented manual-close semantics it could not actually exercise.
    //
    // Hardening 5 unifies the two harnesses on `MockLending` itself: the
    // breaker writes that `borrow()` triggers via `safe_oracle::lastprice`
    // land in `MockLending`'s `instance()` storage (verified empirically
    // since Phase 5.4 v2). Exposing the primitives as test-only methods
    // here means tests share that storage — manual-close after auto-halt
    // becomes a real test, not a workaround.
    //
    // The methods are gated behind `#[cfg(any(test, feature = "testutils"))]`
    // so they are present in:
    //   - `cargo test` builds for this crate's own inline tests
    //   - any consumer crate that pulls `mock-lending` with the `testutils`
    //     feature flag (the test-utils crate does this — see its
    //     `Cargo.toml`)
    // and absent from production WASM builds (the `cdylib` artifact
    // compiled by `stellar contract build` ships without them).
    //
    // Empirical PoC (Pre-5 Discovery) verified two prerequisites:
    //   1. Soroban's `#[contractimpl]` accepts method-level `#[cfg(any(...))]`
    //      attributes — the auto-generated client picks up only the
    //      currently-compiled methods.
    //   2. The `borrow` path's auto-halt write is observable from a
    //      cfg-gated `run_check` on the same contract — `Err(Ok(
    //      CircuitBreakerOpen))` returned in the PoC, confirming shared
    //      `instance()` storage.

    /// Test-only: invoke `safe_oracle::circuit_breaker::check_circuit_breaker`
    /// in `MockLending`'s contract context. Reads the same `instance()`
    /// storage that `lastprice`'s auto-halt writes commit to.
    ///
    /// # Production safety — downstream integrator warning (AR.H L3)
    ///
    /// **These cfg-gated methods (`run_check`, `run_open`, `run_close`)
    /// bypass admin authorization by design.** They exist to support
    /// unified-storage testing of the auto-halt + manual-close path
    /// (Hardening Phase debt #18+#20). The cfg gate ensures they are
    /// excluded from production WASM builds:
    ///
    /// ```toml
    /// [features]
    /// testutils = []  # NEVER enable in production builds
    /// ```
    ///
    /// **A downstream integrator who forks this contract and enables
    /// `testutils` for their test pipeline must verify that their mainnet
    /// build configuration explicitly excludes the feature.** Shipping a
    /// cdylib with `testutils` enabled would expose `run_check` /
    /// `run_open` / `run_close` as un-authorized contract entry points —
    /// effectively ceding admin override of the circuit breaker to any
    /// caller.
    ///
    /// Phase 7 may relocate these primitives to a separate sibling crate
    /// (e.g., `mock-lending-testutils`) to eliminate the cfg-flag footgun
    /// at the build-system level.
    #[cfg(any(test, feature = "testutils"))]
    pub fn run_check(env: Env, asset: Asset) -> Result<(), OracleSafetyViolation> {
        safe_oracle::circuit_breaker::check_circuit_breaker(&env, &asset)
    }

    /// Test-only: invoke `safe_oracle::circuit_breaker::open_circuit_breaker`
    /// in `MockLending`'s contract context. Mirrors what `lastprice`'s
    /// auto-halt does, exposed here so tests can drive the open/close
    /// state machine without going through a guardrail violation.
    ///
    /// See `run_check` doc-comment for the AR.H L3 production-safety
    /// warning that applies to all three cfg-gated primitives.
    #[cfg(any(test, feature = "testutils"))]
    pub fn run_open(env: Env, asset: Asset, duration: u32) {
        safe_oracle::circuit_breaker::open_circuit_breaker(&env, &asset, duration);
    }

    /// Test-only: invoke `safe_oracle::circuit_breaker::close_circuit_breaker`
    /// in `MockLending`'s contract context. Resets state set by either
    /// `run_open` or by `lastprice`'s auto-halt — the unification this
    /// whole block exists to enable.
    ///
    /// See `run_check` doc-comment for the AR.H L3 production-safety
    /// warning that applies to all three cfg-gated primitives.
    #[cfg(any(test, feature = "testutils"))]
    pub fn run_close(env: Env, asset: Asset) {
        safe_oracle::circuit_breaker::close_circuit_breaker(&env, &asset);
    }
}
