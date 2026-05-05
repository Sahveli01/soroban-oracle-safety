//! Circuit breaker state machine for `safe_oracle` (Phase 5.1).
//!
//! Provides a per-asset halt mechanism. When a guardrail violation is
//! detected by `lastprice()` (Phase 5.2 integration), the breaker is opened
//! for `halt_duration_ledgers`; subsequent calls return `CircuitBreakerOpen`
//! immediately without re-running guardrails. After the halt window expires,
//! the breaker auto-closes on the next `check_circuit_breaker` call and
//! normal flow resumes.
//!
//! # Spec
//!
//! See spec §4 — Circuit Breaker Mode and Circuit Breaker State Location.
//! The auto-halt + governance-override + per-asset isolation semantics
//! implemented here all derive directly from the §4 specification; the
//! storage-location decision below (calling-contract instance storage,
//! keyed by asset) is the §4 "State Location" subsection, exact.
//!
//! # Storage location
//!
//! State is stored in the **calling contract's** instance storage under a
//! `CBStorageKey` discriminated by asset. Implications:
//!
//! - Each integrator (lending protocol) maintains its own breaker state.
//! - Two integrators using `safe_oracle` for the same asset have independent
//!   breakers; manipulation against one pool does not halt unrelated pools.
//! - The breaker's storage lifecycle is bound to the integrator's contract.
//!
//! # Authorization — read this before integrating
//!
//! `safe_oracle` does **not** enforce authorization on `open_circuit_breaker`
//! or `close_circuit_breaker`. The library has no admin concept by design
//! (it is stateless infrastructure consumed by many integrators with
//! different governance models). Callers MUST enforce authorization at
//! their own layer:
//!
//! - `open_circuit_breaker`: Intended for internal use by `lastprice()` after
//!   a guardrail violation. External callers may invoke it for manual halt,
//!   but doing so without a `require_auth()` gate exposes the integrator to
//!   griefing — anyone could halt borrowing on any asset at will.
//!
//! - `close_circuit_breaker`: Intended for governance / admin override.
//!   Calling contract MUST verify authorization before invoking. Without
//!   that gate, anyone could close an active breaker and re-enable lending
//!   during an in-progress oracle attack — defeating the breaker's purpose.

use soroban_sdk::{contracttype, Address, Env, Symbol};

use crate::{Asset, OracleSafetyViolation};

/// State of the circuit breaker for a single asset.
///
/// `Open(halt_until_ledger)` carries the ledger sequence at which the halt
/// expires. We store the absolute target rather than the duration so
/// `check_circuit_breaker` can decide auto-recovery with a single
/// comparison against the current ledger, no time-arithmetic at read time.
///
/// Tuple variant rather than struct variant: `#[contracttype]` enums in
/// soroban-sdk 25.x do not support named fields.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitBreakerState {
    /// Normal operation: `lastprice()` runs the full guardrail chain.
    Closed,

    /// Halt active: `lastprice()` returns `CircuitBreakerOpen` until the
    /// ledger sequence reaches the carried value. After that, the next
    /// `check_circuit_breaker` call auto-closes.
    Open(u32),
}

/// Storage keys for breaker state, partitioned by asset variant.
///
/// `Asset::Stellar(Address)` and `Asset::Other(Symbol)` use distinct enum
/// variants so the two key spaces never collide — an `Address` whose bytes
/// happened to match a `Symbol` representation would otherwise share state,
/// and the type system catches the mistake at the storage boundary.
#[contracttype]
#[derive(Clone, Debug)]
pub(crate) enum CBStorageKey {
    /// State for an on-chain Stellar asset, keyed by its contract address.
    StellarAsset(Address),

    /// State for an off-chain Reflector-tracked asset, keyed by symbol.
    OtherAsset(Symbol),
}

/// Map an `Asset` to the corresponding storage key. Single conversion site
/// so a future asset variant only needs to be wired here.
fn storage_key(asset: &Asset) -> CBStorageKey {
    match asset {
        Asset::Stellar(addr) => CBStorageKey::StellarAsset(addr.clone()),
        Asset::Other(sym) => CBStorageKey::OtherAsset(sym.clone()),
    }
}

/// Check the breaker state for an asset.
///
/// # Behavior
///
/// - `Closed` (or no entry yet): returns `Ok(())`.
/// - `Open { halt_until_ledger }` with current sequence `>= halt_until_ledger`:
///   auto-recovers — state transitions to `Closed` and `Ok(())` is returned.
/// - `Open` with current sequence `< halt_until_ledger`: returns
///   `Err(CircuitBreakerOpen)`.
///
/// Auto-recovery means integrators do not need a separate "reset" call after
/// the halt window expires; the next `lastprice()` invocation detects expiry
/// and resumes normal flow.
///
/// # Storage interaction
///
/// On auto-recovery this function **writes** to instance storage (the state
/// transition from `Open` to `Closed`). The write is intentional: the next
/// call should observe `Closed` directly without re-evaluating expiry.
pub fn check_circuit_breaker(env: &Env, asset: &Asset) -> Result<(), OracleSafetyViolation> {
    let key = storage_key(asset);
    let state: CircuitBreakerState = env
        .storage()
        .instance()
        .get(&key)
        .unwrap_or(CircuitBreakerState::Closed);

    match state {
        CircuitBreakerState::Closed => Ok(()),
        CircuitBreakerState::Open(halt_until_ledger) => {
            if env.ledger().sequence() >= halt_until_ledger {
                env.storage()
                    .instance()
                    .set(&key, &CircuitBreakerState::Closed);
                Ok(())
            } else {
                Err(OracleSafetyViolation::CircuitBreakerOpen)
            }
        }
    }
}

/// Open the circuit breaker for an asset, halting `lastprice()` calls until
/// `env.ledger().sequence() + halt_duration_ledgers`.
///
/// # Spec
///
/// See spec §4 — Circuit Breaker Mode (auto-halt path). Invoked by
/// `lastprice` after a guardrail violation when
/// `config.circuit_breaker_enabled = true`; integrators may also invoke it
/// directly for an off-chain-monitor-driven manual halt, behind their own
/// auth gate (see Authorization below).
///
/// # Authorization
///
/// `safe_oracle` does **not** enforce auth here. Intended for internal use by
/// `lastprice()` (Phase 5.2) on guardrail violation. External callers can
/// invoke it for manual halt, but only behind a `require_auth()` gate in
/// their own contract — otherwise anyone could grief the integrator by
/// halting borrowing on any asset.
///
/// # Idempotency
///
/// If the breaker is already `Open`, this **overwrites** the existing
/// `halt_until_ledger` with the new value. A fresh violation should extend
/// the halt window forward, not preserve a shorter prior window.
///
/// # Overflow
///
/// `saturating_add` clamps `halt_until_ledger` at `u32::MAX` rather than
/// panicking. A maliciously large `halt_duration_ledgers` therefore degrades
/// to "permanent halt until u32::MAX" rather than aborting the transaction —
/// the latter would let an attacker who can influence `halt_duration_ledgers`
/// brick `lastprice()` outright.
///
/// # Parameters
///
/// - `halt_duration_ledgers`: ledgers from now until auto-recovery. The
///   default in `SafeOracleConfig::circuit_breaker_halt_ledgers` is 720
///   (~1 hour at Stellar's ~5-second close time).
pub fn open_circuit_breaker(env: &Env, asset: &Asset, halt_duration_ledgers: u32) {
    let key = storage_key(asset);
    let halt_until_ledger = env
        .ledger()
        .sequence()
        .saturating_add(halt_duration_ledgers);

    env.storage()
        .instance()
        .set(&key, &CircuitBreakerState::Open(halt_until_ledger));
}

/// Manually close the circuit breaker for an asset (governance override).
///
/// # Spec
///
/// See spec §4 — Circuit Breaker Mode (governance manual override path).
/// Spec §4 specifies that halts can be cleared by DAO/governance action
/// after off-chain verification; this is the primitive integrators wrap
/// behind an auth-gated wrapper to deliver that capability.
///
/// # CRITICAL — Authorization
///
/// **`safe_oracle` does NOT enforce authorization on this function.** The
/// calling contract MUST verify admin/governance auth before invoking.
/// Recommended pattern:
///
/// ```rust,ignore
/// pub fn governance_close_breaker(env: Env, admin: Address, asset: Asset) {
///     admin.require_auth();
///     let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
///     assert_eq!(admin, stored_admin, "only admin may close the breaker");
///     safe_oracle::circuit_breaker::close_circuit_breaker(&env, &asset);
/// }
/// ```
///
/// Without that gate, **any caller can close an active breaker and re-enable
/// lending during an in-progress oracle attack** — exactly the situation the
/// breaker exists to prevent. Treat this function as privileged.
///
/// # Use cases
///
/// - DAO/multisig governance clearing a breaker after off-chain verification.
/// - Ops emergency response to a known false positive.
/// - Test utilities resetting state between scenarios.
///
/// # Behavior
///
/// Sets state to `Closed` regardless of current state. Idempotent — closing
/// an already-closed breaker is a no-op semantically (still writes `Closed`).
pub fn close_circuit_breaker(env: &Env, asset: &Asset) {
    let key = storage_key(asset);
    env.storage()
        .instance()
        .set(&key, &CircuitBreakerState::Closed);
}
