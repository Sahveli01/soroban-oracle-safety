#![no_std]

use soroban_sdk::{contracterror, contracttype, Address, Env, Symbol, Vec};

pub mod circuit_breaker;
mod reflector_client;
mod registry_client;
pub use reflector_client::ReflectorClient;
pub use registry_client::{LiquidityRegistryClient, LiquiditySnapshot};

/// Reasons a guardrail has rejected a price; the `Err` payload of every
/// safe_oracle public API.
///
/// Discriminants are stable u32 values (1..=7) so they can be carried as the
/// `u32` inside [`PriceResult::Err`] and re-hydrated through
/// [`PriceResult::into_result`]. Integrators surfacing oracle violations to
/// their own callers typically mirror these discriminants 1:1 in their own
/// error enum (see `mock_lending::MockLendingError` for the canonical
/// reference) so audit logs preserve which guardrail tripped.
///
/// # Spec
///
/// See spec §4 — Error Enum. The seven variants here implement the spec's
/// required violation taxonomy. Phases 1–5 wired the variants in order:
/// 1–3 (Layer 1) in Phase 2, 4–5 (Layer 2) in Phase 4, 6 (circuit breaker)
/// in Phase 5, and 7 (stale snapshot) introduced alongside the freshness
/// check in Phase 4.
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
    /// An external contract (Reflector primary feed or `LiquidityRegistry`)
    /// failed unexpectedly — host-level trap, contract upgrade
    /// incompatibility, storage corruption, or any other invocation error
    /// surfaced through Soroban's `try_*` client variants. Hardening Phase
    /// debt #4 added this variant so cross-contract failures arrive as
    /// regular guardrail violations rather than propagating to the caller
    /// (which would prevent auto-halt from committing — same Phase 5.2 v1
    /// root cause).
    ///
    /// Secondary-feed failures intentionally do NOT surface as this variant
    /// — `check_cross_source` skips silently on secondary trap, consistent
    /// with `None` and "secondary returned `None`" semantics.
    ExternalContractFailure = 8,
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

/// Result type for `lastprice()` that allows auto-halt to commit even on
/// guardrail violations.
///
/// # Why a custom enum instead of `Result<PriceData, OracleSafetyViolation>`?
///
/// Soroban contract methods that return `Result::Err` cause **all storage
/// writes in the same invocation to roll back**, including writes inside
/// `open_circuit_breaker()`. The original Phase 5.2 design hit this and was
/// reverted (commit `e98ed48`). By returning `Ok(PriceResult::Err(...))`
/// from the contract method, the breaker write commits while still
/// conveying the violation to the caller.
///
/// # Why `Err(u32)` and not `Err(OracleSafetyViolation)`?
///
/// `OracleSafetyViolation` is a `#[contracterror]` type. soroban-sdk 25.x
/// has two distinct constraints that block embedding it inside the
/// `#[contracttype]` enum below — both empirically verified, the second
/// not surfaced until Hardening 6C's PoC:
///
/// 1. **`SorobanArbitrary` bound (Pre-5.4 finding).** Under the test
///    feature `soroban-sdk` derives an `Arbitrary` prototype for every
///    `#[contracttype]`. The derive recursively requires every variant's
///    payload to implement `SorobanArbitrary`, which `#[contracterror]`
///    types do not — build fails with "trait bound
///    `OracleSafetyViolation: SorobanArbitrary` is not satisfied."
///    Manual `SorobanArbitrary` impl on the error type is conceptually
///    possible (the trait is `pub`, three trait bounds to satisfy).
///
/// 2. **`ScVec: TryFrom<(ScSymbol, &OracleSafetyViolation)>` bound
///    (Hardening 6C finding, deferred).** Independent of the
///    `Arbitrary` derive, the `#[contracttype]` macro's XDR encoding
///    expects each variant payload to be convertible into the tuple
///    shape `(ScSymbol, &T)` ⟶ `ScVec`. `#[contracterror]` types
///    implement `IntoVal<Env, Val>` but not this specific tuple-to-XDR
///    path. A manual impl is blocked by Rust's orphan rule — both
///    `ScVec` and `(ScSymbol, &T)` are foreign, so neither side of the
///    `TryFrom` can host the impl from this crate. Closing this would
///    require either (a) a `soroban-sdk` change exposing the conversion
///    or (b) reshaping `OracleSafetyViolation` away from
///    `#[contracterror]` (which would lose the stable u32 discriminants
///    that integrators consume).
///
/// Carrying the violation as its `u32` discriminant sidesteps both
/// constraints. The values here MUST stay aligned with
/// `OracleSafetyViolation = 1..=8`. The `into_result()` shim re-hydrates
/// the typed variant for callers that want it. Hardening Phase debt #17
/// remains deferred for future SDK releases that resolve constraint (2).
///
/// # Migration from the Phase 1-4 `Result<PriceData, OracleSafetyViolation>`
///
/// Callers that used `?` continue to do so via the `into_result()` shim:
///
/// ```ignore
/// // Before (Phase 1-4):
/// let price = safe_oracle::lastprice(&env, &asset, ...)?;
///
/// // After (Phase 5.2 v2):
/// let price = safe_oracle::lastprice(&env, &asset, ...).into_result()?;
/// ```
///
/// `From<Result<PriceData, OracleSafetyViolation>>` is also implemented so
/// internal helpers that produce `Result` (e.g., `lastprice_inner`) convert
/// at the API boundary without per-callsite match plumbing.
///
/// # Audit notes
///
/// - `PriceResult::Err(d)` is semantically identical to a guardrail
///   failure. A lending protocol MUST NOT proceed with `PriceResult::Err`
///   the same way it would not proceed with `Err` in Phase 1-4.
/// - The `Ok` wrapping at the Soroban boundary is a storage-commit
///   mechanism only; the public-facing semantics ("violation = no price")
///   are unchanged.
/// - Tuple variants (not named-field) match the soroban-sdk 25.x
///   `#[contracttype]` enum constraint observed in Phase 5.1.
///
/// # Spec
///
/// See spec §4 — Function Signature and Stub Contract. `PriceResult`
/// preserves the spec's `lastprice → Ok(price) | Err(violation)` semantic
/// at the public API level (via [`PriceResult::into_result`]) while letting
/// auto-halt writes inside `lastprice` commit at the Soroban boundary.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PriceResult {
    /// Validated price data, all guardrails passed.
    Ok(PriceData),

    /// Guardrail violation; price MUST NOT be used. The `u32` is the
    /// `OracleSafetyViolation` discriminant (1..=7); see `into_result()`
    /// for the typed re-hydration.
    Err(u32),
}

impl PriceResult {
    /// Returns `true` if the result is `Ok`.
    pub fn is_ok(&self) -> bool {
        matches!(self, PriceResult::Ok(_))
    }

    /// Returns `true` if the result is `Err`.
    pub fn is_err(&self) -> bool {
        matches!(self, PriceResult::Err(_))
    }

    /// Convert to standard Rust `Result` for ergonomic `?` operator usage.
    ///
    /// Recommended migration path for Phase 1-4 callers: replace
    /// `lastprice(...)?` with `lastprice(...).into_result()?`.
    ///
    /// Re-hydrates the `u32` discriminant into the typed
    /// `OracleSafetyViolation`. Unknown discriminants panic — they cannot
    /// occur on a result produced by `lastprice()`, which only emits
    /// values from the canonical `1..=7` range, but the explicit panic
    /// guards against forged values reaching the shim.
    pub fn into_result(self) -> Result<PriceData, OracleSafetyViolation> {
        match self {
            PriceResult::Ok(p) => Ok(p),
            PriceResult::Err(1) => Err(OracleSafetyViolation::ExcessiveDeviation),
            PriceResult::Err(2) => Err(OracleSafetyViolation::StaleData),
            PriceResult::Err(3) => Err(OracleSafetyViolation::CrossSourceMismatch),
            PriceResult::Err(4) => Err(OracleSafetyViolation::InsufficientLiquidity),
            PriceResult::Err(5) => Err(OracleSafetyViolation::ThinSampling),
            PriceResult::Err(6) => Err(OracleSafetyViolation::CircuitBreakerOpen),
            PriceResult::Err(7) => Err(OracleSafetyViolation::StaleSnapshot),
            PriceResult::Err(8) => Err(OracleSafetyViolation::ExternalContractFailure),
            PriceResult::Err(d) => panic!(
                "PriceResult::Err discriminant {} is outside the OracleSafetyViolation range (1..=8)",
                d
            ),
        }
    }
}

impl From<Result<PriceData, OracleSafetyViolation>> for PriceResult {
    fn from(r: Result<PriceData, OracleSafetyViolation>) -> Self {
        match r {
            Ok(p) => PriceResult::Ok(p),
            Err(e) => PriceResult::Err(e as u32),
        }
    }
}

/// Configuration for the safe_oracle library — the per-pool tuning surface.
///
/// Holds the thresholds and toggles consumed by [`lastprice`] and the
/// [`circuit_breaker`] module. Integrators construct it once at init time
/// and pass it to every `lastprice` call; the library is stateless, so the
/// integrator owns where this lives in storage.
///
/// # Spec
///
/// See spec §4 — Config Struct. The defaults returned by
/// [`SafeOracleConfig::default`] match the spec's recommended values
/// (`max_deviation_bps=2000`, `max_staleness_seconds=300`,
/// `max_cross_source_bps=500`, `min_liquidity_usd=$10,000` at 7-decimal
/// precision, `min_trade_count_1h=5`); integrators requiring tighter or
/// looser thresholds override per-field.
#[contracttype]
#[derive(Clone, Debug)]
pub struct SafeOracleConfig {
    pub max_deviation_bps: u32,
    pub max_staleness_seconds: u32,
    pub max_cross_source_bps: u32,
    /// Maximum age (in seconds) of a `LiquidityRegistry` snapshot still
    /// considered fresh. Phase 4's `check_liquidity` rejects snapshots older
    /// than this against `env.ledger().timestamp()`; the field is wired here
    /// in Phase 3.6 so config-construction sites do not need to change again
    /// when the Layer 2 logic lands.
    pub max_snapshot_age_seconds: u64,
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
            max_staleness_seconds: 300,
            max_cross_source_bps: 500,
            max_snapshot_age_seconds: 300,
            min_liquidity_usd: 100_000_000_000,
            min_trade_count_1h: 5,
            secondary_oracle: None,
            circuit_breaker_enabled: false,
            circuit_breaker_halt_ledgers: 720,
        }
    }
}

/// Errors returned by [`SafeOracleConfig::validate`] when a config field
/// has an out-of-range value that would silently disable a guardrail or
/// produce nonsensical behavior at runtime.
///
/// # Spec
///
/// See spec §4 — Config Struct. Validation prevents silent guardrail
/// disabling caused by misconfiguration (e.g., `max_deviation_bps = 0`
/// allows infinite deviation, effectively disabling the deviation check
/// without any visible signal).
///
/// # Audit notes
///
/// - Validation is **opt-in** — callers must invoke `config.validate()`.
///   The library does not enforce validation in `lastprice()` to avoid
///   per-call gas cost. Production integrators should validate at init
///   time (recommended pattern: `MockLending::initialize` calls validate).
///
/// - All errors are recoverable at init time; runtime config changes are
///   not supported (config is immutable after deploy per spec §4).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// `max_deviation_bps` is 0 (allows infinite deviation, disabling the
    /// check) or > 10_000 (100% — values above this are nonsensical for a
    /// relative-deviation threshold).
    InvalidDeviationBps,

    /// `max_staleness_seconds` is 0 (rejects every recorded price as
    /// stale) or > 86_400 (24h — stale data older than a day is unsafe
    /// regardless of how lenient the integrator wants to be).
    InvalidStalenessSeconds,

    /// `min_liquidity_usd` is `<= 0` — negative values are semantically
    /// nonsense (liquidity is non-negative by definition), and `0` silently
    /// disables the Layer 2 liquidity check (every snapshot's
    /// `volume_30m_usd > 0` trivially passes the threshold). Same defensive
    /// principle as `InvalidTradeCountThreshold`: a zero guardrail is no
    /// guardrail.
    ///
    /// # AR.H M1 closure
    ///
    /// This variant's runtime rule was tightened from `< 0` to `<= 0` after
    /// AR.H surfaced the silent-disable case as the single residual asymmetry
    /// from Hardening Closure (Debt #22).
    InvalidLiquidityThreshold,

    /// `secondary_oracle` is `Some(_)` but `max_cross_source_bps > 10_000`.
    /// The cross-source guardrail is configured but its threshold is
    /// nonsensical. When `secondary_oracle = None` the value of
    /// `max_cross_source_bps` is irrelevant (cross-source is skipped
    /// entirely), so this check is conditional.
    InvalidCrossSourceBps,

    /// `circuit_breaker_enabled = true` but `circuit_breaker_halt_ledgers
    /// == 0`. A degenerate halt window: the breaker would fire and
    /// immediately auto-recover on the same call, providing no actual
    /// halt. When the breaker is disabled, the halt-ledgers field is
    /// dormant, so this check is conditional.
    InvalidHaltLedgers,

    /// `min_trade_count_1h` is 0 — disables thin-sampling check entirely
    /// (every snapshot's `unique_trades_1h >= 0` always true). Same defensive
    /// principle as `InvalidDeviationBps`: a "guardrail of zero" is no
    /// guardrail at all. Integrators wanting to disable Layer 2 thin-sampling
    /// should leave the guardrail's threshold meaningful and route around it
    /// via Layer 1 / circuit breaker controls.
    ///
    /// # Hardening 3A follow-up (Debt #22)
    ///
    /// This variant closes the gap intentionally left open in Hardening 3A,
    /// where the prompt's "5 variant" boundary kept the pattern consistent
    /// with the other guardrails but left two silent-disable cases
    /// undetected. Hardening Closure brings parity.
    InvalidTradeCountThreshold,

    /// `max_snapshot_age_seconds` is 0 (rejects all snapshots) or > 86_400
    /// (24h — staler than this is unsafe regardless of integrator intent).
    /// Mirrors `InvalidStalenessSeconds` boundary logic for the Layer 2 path.
    ///
    /// # Hardening 3A follow-up (Debt #22)
    ///
    /// Same closure rationale as `InvalidTradeCountThreshold`. The Hardening
    /// 3A boundary kept the new-variant count at 5; the Layer 2 snapshot age
    /// validation remained an audit-trail gap until this patch.
    InvalidSnapshotAge,
}

impl SafeOracleConfig {
    /// Validates the config and returns an error if any field has an
    /// out-of-range value. Recommended call site: at integrator
    /// initialization, before storing the config in instance storage.
    ///
    /// # Spec
    ///
    /// See spec §4 — Config Struct. Validation is opt-in (the library
    /// does not enforce it on every `lastprice` call) so integrators pay
    /// the check exactly once per config change.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::InvalidDeviationBps`] — `max_deviation_bps == 0`
    ///   or `> 10_000`.
    /// - [`ConfigError::InvalidStalenessSeconds`] — `max_staleness_seconds
    ///   == 0` or `> 86_400`.
    /// - [`ConfigError::InvalidLiquidityThreshold`] — `min_liquidity_usd
    ///   < 0`.
    /// - [`ConfigError::InvalidCrossSourceBps`] — secondary configured
    ///   but `max_cross_source_bps > 10_000`.
    /// - [`ConfigError::InvalidHaltLedgers`] — `circuit_breaker_enabled`
    ///   and `circuit_breaker_halt_ledgers == 0`.
    /// - [`ConfigError::InvalidTradeCountThreshold`] — `min_trade_count_1h
    ///   == 0` (Hardening Closure / Debt #22).
    /// - [`ConfigError::InvalidSnapshotAge`] — `max_snapshot_age_seconds
    ///   == 0` or `> 86_400` (Hardening Closure / Debt #22).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = SafeOracleConfig::default();
    /// config.validate().expect("default config is valid by construction");
    /// ```
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.max_deviation_bps == 0 || self.max_deviation_bps > 10_000 {
            return Err(ConfigError::InvalidDeviationBps);
        }

        if self.max_staleness_seconds == 0 || self.max_staleness_seconds > 86_400 {
            return Err(ConfigError::InvalidStalenessSeconds);
        }

        // AR.H M1 fix: also reject == 0 to prevent silent-disable.
        // With min_liquidity_usd == 0, the runtime check
        // `snapshot.volume_30m_usd < 0` is unreachable because write_snapshot
        // rejects volume_30m_usd <= 0 — every attestation passes the threshold,
        // silently disabling the Layer 2 liquidity guardrail (the YieldBlox
        // vector). Mirrors the silent-disable defenses Hardening 3A established
        // for the deviation/staleness/halt-ledgers fields and Hardening Closure
        // (Debt #22) extended to min_trade_count_1h and max_snapshot_age_seconds.
        if self.min_liquidity_usd <= 0 {
            return Err(ConfigError::InvalidLiquidityThreshold);
        }

        if self.secondary_oracle.is_some() && self.max_cross_source_bps > 10_000 {
            return Err(ConfigError::InvalidCrossSourceBps);
        }

        if self.circuit_breaker_enabled && self.circuit_breaker_halt_ledgers == 0 {
            return Err(ConfigError::InvalidHaltLedgers);
        }

        // Hardening Closure (Debt #22): Layer 2 thin-sampling guard.
        // min_trade_count_1h == 0 silently disables the check.
        if self.min_trade_count_1h == 0 {
            return Err(ConfigError::InvalidTradeCountThreshold);
        }

        // Hardening Closure (Debt #22): Layer 2 snapshot age guard.
        // 0 rejects all snapshots; > 86_400 (24h) accepts unsafe staleness.
        if self.max_snapshot_age_seconds == 0 || self.max_snapshot_age_seconds > 86_400 {
            return Err(ConfigError::InvalidSnapshotAge);
        }

        Ok(())
    }
}

/// Validates oracle output against five layered guardrails before returning a
/// price, wrapped by the circuit breaker (Phase 5.2 v2).
///
/// Public entry point of the `safe_oracle` library. Lending protocols call
/// this instead of `reflector.lastprice()` directly.
///
/// # Spec
///
/// See spec §4 — `safe_oracle` Library API. This is the canonical entry
/// point defined in "Function Signature and Stub Contract"; the integration
/// example in §4 shows the one-line migration from `reflector.lastprice(asset)`
/// to this call.
///
/// # Why `PriceResult` instead of `Result`?
///
/// Soroban contract methods that return `Result::Err` roll back all storage
/// writes in the same invocation. The original Phase 5.2 design (commit
/// `6ef65b7`, reverted in `e98ed48`) hit this and could not commit
/// `open_circuit_breaker()` writes — auto-halt never persisted. Wrapping
/// violations in `PriceResult::Err` (returned through the `Ok` boundary at
/// the Soroban level) lets the breaker write commit cleanly.
///
/// See `PriceResult` for full migration guidance and `into_result()` shim.
///
/// # Guardrails
/// - Layer 1 (Reflector-only): deviation, staleness, cross-source
/// - Layer 2 (LiquidityRegistry-required): liquidity threshold, thin sampling
/// - Wrapper: circuit breaker (Phase 5)
///
/// # Circuit breaker integration
///
/// 1. Pre-flight: `check_circuit_breaker(env, asset)` runs first. If the
///    breaker is `Open` and the halt window has not expired, returns
///    `PriceResult::Err(CircuitBreakerOpen)` immediately — no Reflector or
///    LiquidityRegistry calls are made, so a halted asset costs near-zero
///    gas to reject. Auto-recovery on expiry is handled inside
///    `check_circuit_breaker`.
///
/// 2. Auto-halt: if `config.circuit_breaker_enabled == true` (default
///    `false`) and any guardrail violates, the breaker is opened for
///    `config.circuit_breaker_halt_ledgers` ledgers (default 720, ~1 hour
///    at 5-second close time). The violation is then returned as
///    `PriceResult::Err(<violation>)`.
///
/// The breaker is opt-in. With the default config, this function preserves
/// the exact Phase 1-4 contract: guardrail violations propagate as
/// `PriceResult::Err` without persisting any breaker state.
pub fn lastprice(
    env: &Env,
    asset: &Asset,
    reflector: &Address,
    liquidity_registry: &Address,
    config: &SafeOracleConfig,
) -> PriceResult {
    // Pre-flight breaker check. Open + not yet expired → short-circuit
    // before any cross-contract call. Auto-recovery (state transition
    // Open → Closed when ledger advanced past halt window) is handled
    // inside check_circuit_breaker.
    if let Err(e) = circuit_breaker::check_circuit_breaker(env, asset) {
        return PriceResult::Err(e as u32);
    }

    let result = lastprice_inner(env, asset, reflector, liquidity_registry, config);

    // Auto-halt on guardrail violation. Only trips when the integrator
    // opted in — default `circuit_breaker_enabled = false` keeps Phase 1-4
    // behavior (no breaker side effects).
    //
    // CRITICAL: this write commits because the contract method returns Ok
    // at the Soroban boundary (PriceResult::Err is wrapped in Ok). Phase
    // 5.2 v1 used Result::Err here and the write rolled back; that is the
    // bug this version exists to fix. Empirical evidence in the Pre-5.2.C
    // discovery diagnostic (no-commit transient state).
    if result.is_err() && config.circuit_breaker_enabled {
        circuit_breaker::open_circuit_breaker(env, asset, config.circuit_breaker_halt_ledgers);
    }

    PriceResult::from(result)
}

/// Internal: full 5-guardrail chain without circuit breaker concerns.
///
/// Split from `lastprice` so the breaker stays a pure wrapper concern
/// (pre-flight check + post-failure halt) and the guardrail chain itself
/// remains the unchanged Phase 4.2 implementation. Returns `Result` rather
/// than `PriceResult` because the wrapper composes the two with a single
/// `PriceResult::from(result)` at the boundary — the `?` operator on
/// `Result` keeps the inner code idiomatic.
fn lastprice_inner(
    env: &Env,
    asset: &Asset,
    reflector: &Address,
    liquidity_registry: &Address,
    config: &SafeOracleConfig,
) -> Result<PriceData, OracleSafetyViolation> {
    // 1. Fetch newest + previous prices in a single cross-contract call.
    //
    // Hardening Phase debt #14: pre-6A this path issued two reads —
    // `records=1` here for `current`, then `records=2` again inside
    // `check_deviation` for the previous price. The records=2 fetch
    // already returns both, so the records=1 call was redundant; folding
    // it eliminates one Reflector round-trip per Layer 1 evaluation.
    // Actual gas savings will be measured during testnet deployment
    // (debt #13, deferred to Phase 8).
    //
    // `fetch_reflector_prices` enforces `prices.len() >= records`, so
    // missing-history scenarios (0 or 1 stored price) surface as
    // `StaleData` from the helper itself — `prices.get(1)` here is
    // always populated when this path executes.
    let prices = fetch_reflector_prices(env, reflector, asset, 2)?;
    let p0 = prices.get(0).ok_or(OracleSafetyViolation::StaleData)?;
    let p1 = prices.get(1).ok_or(OracleSafetyViolation::StaleData)?;

    // Newest/oldest by `timestamp`, not vec index — the mock currently
    // returns newest-first, but production code does not depend on that
    // ordering convention.
    let (current, previous) = if p0.timestamp >= p1.timestamp {
        (p0, p1)
    } else {
        (p1, p0)
    };

    // 2. Layer 1 guardrails (Reflector-only data).
    // `check_deviation_from_pair` is pure validation — both prices are
    // already in hand, no further cross-contract calls.
    check_deviation_from_pair(&current, &previous, config)?;
    check_staleness(env, &current, config)?;
    check_cross_source(env, asset, &current, config)?;

    // 3. Layer 2 guardrails (require LiquidityRegistry).
    // Single cross-contract call shared by both threshold checks; helper
    // returns None for Asset::Other so both guardrails skip together.
    if let Some(snapshot) = get_validated_snapshot(env, liquidity_registry, asset, config)? {
        check_liquidity(&snapshot, config)?;
        check_thin_sampling(&snapshot, config)?;
    }

    Ok(current)
}

/// Fetches the most recent `records` prices from Reflector via cross-contract call.
///
/// Returns prices ordered newest-first. Single source of truth for every
/// Reflector read: `lastprice_inner` calls this once with `records=2`
/// (Hardening 6A debt #14 collapsed the previous two-call pattern into
/// one). Reflector returns `None` when the asset has no recorded prices,
/// and a shorter `Vec` when history is thinner than `records`; both cases
/// map to `Err(StaleData)` here — fail-safe default that downstream
/// guardrails can rely on.
fn fetch_reflector_prices(
    env: &Env,
    reflector: &Address,
    asset: &Asset,
    records: u32,
) -> Result<Vec<PriceData>, OracleSafetyViolation> {
    let client = ReflectorClient::new(env, reflector);
    // Hardening Phase debt #4: graceful handling of Reflector contract
    // trap. `try_lastprices` wraps the cross-contract invocation so a
    // Reflector panic (upgrade incompatibility, storage corruption, host
    // trap) lands in the `Err(Ok(_))` arm rather than propagating to the
    // caller. Without this guard a primary-feed crash would prevent the
    // auto-halt write from committing — same root-cause family as the
    // Phase 5.2 v1 revert.
    //
    // Empirical PoC (pre-3C): a panicking contract method invoked through
    // `try_<method>` lands in `Err(Ok(_))` with a framework
    // representation of the trap. The wildcard arm below catches that
    // plus all other non-success shapes (XDR conversion, host error).
    let prices = match client.try_lastprices(asset, &records) {
        Ok(Ok(Some(p))) => p,
        Ok(Ok(None)) => return Err(OracleSafetyViolation::StaleData),
        _ => return Err(OracleSafetyViolation::ExternalContractFailure),
    };

    if prices.len() < records {
        return Err(OracleSafetyViolation::StaleData);
    }

    Ok(prices)
}

/// Layer 1, Guardrail 1 — Maximum Deviation (pure validation).
///
/// Compares the newest price against its predecessor recorded by Reflector
/// (one resolution-window earlier — typically ~5 min) and rejects updates
/// whose BPS deviation exceeds `config.max_deviation_bps`. This is the
/// primary defense against YieldBlox-class SDEX manipulation: an attacker
/// who shifts the spot price by buying/selling on a thin market produces a
/// delta that this guardrail flags as `ExcessiveDeviation`.
///
/// # Hardening Phase debt #14
///
/// Pre-6A this lived as `check_deviation`, which made its own
/// `fetch_reflector_prices(records=2)` call independent of the records=1
/// fetch in `lastprice_inner` — two cross-contract reads on every Layer 1
/// evaluation. The records=2 fetch is now done once at the entry point
/// and both prices passed in here as references; this helper became pure
/// validation (no env/reflector/asset parameters).
///
/// The pre-6A sanity check `current.timestamp != newest.timestamp` was a
/// defense against the (impossible-in-single-tx) scenario where storage
/// mutated between the two cross-contract reads. With one read, that
/// scenario cannot arise; the check is removed as dead code.
///
/// # Defensive logic
/// - `current.price <= 0` or `previous.price <= 0` → `ExcessiveDeviation`.
///   Reflector should never return a non-positive price, but a corrupted
///   or malicious feed is the threat model.
/// - `checked_mul(10_000)` catches the rare overflow where
///   `abs_diff * 10_000` would exceed `i128::MAX`; treating overflow as
///   deviation is the safe default.
fn check_deviation_from_pair(
    current: &PriceData,
    previous: &PriceData,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    if current.price <= 0 || previous.price <= 0 {
        return Err(OracleSafetyViolation::ExcessiveDeviation);
    }

    let abs_diff = (current.price - previous.price).abs();
    let scaled = abs_diff
        .checked_mul(10_000)
        .ok_or(OracleSafetyViolation::ExcessiveDeviation)?;
    let deviation_bps = scaled / previous.price;

    if deviation_bps > config.max_deviation_bps as i128 {
        return Err(OracleSafetyViolation::ExcessiveDeviation);
    }

    Ok(())
}

/// Layer 1, Guardrail 3 — Staleness Check.
///
/// Compares the Reflector price's `timestamp` against the current ledger time
/// (`env.ledger().timestamp()` — both Unix seconds, no conversion). Rejects
/// prices older than `config.max_staleness_seconds`. This blocks the
/// stale-feed attack class: an oracle that has not refreshed (because the
/// off-chain feed is down or paused) cannot be used to value collateral.
///
/// # Defensive logic
/// - `current.timestamp > now` → `StaleData`. A future-dated price implies
///   clock skew or feed manipulation; treat as untrusted.
/// - `elapsed > max_staleness_seconds` → `StaleData`. Hard cutoff; `>` is
///   used (not `>=`) so the boundary value is accepted — consistent with
///   `check_deviation`'s threshold semantics.
/// - `now - current.timestamp` cannot underflow: the future-check above
///   guarantees `current.timestamp <= now`.
fn check_staleness(
    env: &Env,
    current: &PriceData,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    let now = env.ledger().timestamp();

    if current.timestamp > now {
        return Err(OracleSafetyViolation::StaleData);
    }

    let elapsed = now - current.timestamp;
    if elapsed > config.max_staleness_seconds as u64 {
        return Err(OracleSafetyViolation::StaleData);
    }

    Ok(())
}

/// Layer 1, Guardrail 4 — Multi-Source Cross-Check.
///
/// When `config.secondary_oracle` is `Some(addr)`, fetches the secondary
/// oracle's price for the same asset and rejects the trade if the two sources
/// disagree by more than `config.max_cross_source_bps`. Reflector CEX feeds
/// can be cross-checked against DEX feeds (or DIA) so that an attack that
/// shifts only one feed is caught by the other. Opt-in: `None` skips entirely.
///
/// # Skip vs. fail semantics
/// - `secondary_oracle = None` → `Ok(())`. Single-source operation is allowed.
/// - Secondary returns `None` (no recorded price) → `Ok(())`. "No evidence" is
///   not the same as "evidence of mismatch"; we don't penalize an asset just
///   because the secondary feed has not seen it yet.
/// - Secondary returns a non-positive price → `CrossSourceMismatch`. A live
///   feed reporting zero/negative is a manipulation signal, not a data gap.
/// - Secondary returns a stale price (older than `config.max_staleness_seconds`,
///   the same threshold the primary's freshness check uses) → `Ok(())`. A
///   stale value is "no fresh evidence"; comparing primary against an old
///   secondary would generate false-positive halts whenever the secondary
///   updates lag behind primary. Hardening 3B debt #3 added this skip;
///   pre-3B behavior collapsed stale secondary into the BPS comparison
///   below.
/// - BPS deviation beyond threshold → `CrossSourceMismatch`.
///
/// Primary is the BPS reference (`|primary - secondary| * 10_000 / primary`)
/// because primary is the value the lending contract actually consumes.
fn check_cross_source(
    env: &Env,
    asset: &Asset,
    current: &PriceData,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    let secondary = match &config.secondary_oracle {
        Some(addr) => addr,
        None => return Ok(()),
    };

    if current.price <= 0 {
        return Err(OracleSafetyViolation::CrossSourceMismatch);
    }

    let client = ReflectorClient::new(env, secondary);
    // Hardening Phase debt #4: graceful handling of secondary Reflector
    // trap. Secondary failure short-circuits to `Ok(())` (silent skip) —
    // same semantics as `secondary_oracle = None` and "secondary returned
    // `None`". The cross-source check is opt-in defense-in-depth; a
    // broken secondary feed must not freeze borrowing on an
    // otherwise-healthy primary. Primary failure is handled separately in
    // `fetch_reflector_prices` and surfaces as `ExternalContractFailure`.
    let secondary_price = match client.try_lastprice(asset) {
        Ok(Ok(Some(p))) => p,
        _ => return Ok(()),
    };

    if secondary_price.price <= 0 {
        return Err(OracleSafetyViolation::CrossSourceMismatch);
    }

    // Hardening Phase debt #3: skip when the secondary feed is stale. A
    // stale value is not fresh evidence of disagreement; treating it as a
    // mismatch would generate false-positive halts whenever the secondary
    // updates lag behind primary. Uses the same `max_staleness_seconds`
    // threshold as primary's `check_staleness` — the integrator's freshness
    // expectation is uniform across both feeds.
    //
    // `saturating_sub` handles future-dated secondary timestamps (clock
    // skew) without panicking: future values yield `secondary_age = 0`,
    // which falls through to the BPS comparison rather than hitting the
    // skip path. The BPS check itself is the safety net for that anomaly.
    let now = env.ledger().timestamp();
    let secondary_age = now.saturating_sub(secondary_price.timestamp);
    if secondary_age > config.max_staleness_seconds as u64 {
        return Ok(());
    }

    let abs_diff = (current.price - secondary_price.price).abs();
    let scaled = abs_diff
        .checked_mul(10_000)
        .ok_or(OracleSafetyViolation::CrossSourceMismatch)?;
    let deviation_bps = scaled / current.price;

    if deviation_bps > config.max_cross_source_bps as i128 {
        return Err(OracleSafetyViolation::CrossSourceMismatch);
    }

    Ok(())
}

/// Fetch and validate a `LiquiditySnapshot` for a given asset.
///
/// Encapsulates the snapshot fetch + freshness check shared by
/// `check_liquidity` and `check_thin_sampling`. Called once per `lastprice`
/// invocation so that both Layer 2 guardrails are served by a single
/// cross-contract call to `LiquidityRegistry::get_snapshot` — the round-trip
/// dominates Layer 2 cost, and Phase 4.1's per-guardrail fetch was paying it
/// twice.
///
/// # Returns
/// - `Ok(Some(snapshot))` — `Asset::Stellar` with a fresh, attested snapshot.
/// - `Ok(None)` — `Asset::Other` (off-chain asset). Cross-source (Layer 1)
///   is the relevant defense for these; both Layer 2 guardrails skip when
///   the helper returns `None`.
/// - `Err(InsufficientLiquidity)` — `Asset::Stellar` with no snapshot in the
///   registry. Fail-safe: "no evidence of liquidity" is treated as evidence
///   of absence so a forgotten attester pipeline cannot silently bypass the
///   guardrail (spec §3, Layer 2).
/// - `Err(StaleSnapshot)` — snapshot older than `config.max_snapshot_age_seconds`.
///   Freshness is enforced consumer-side (here) rather than in the registry,
///   keeping the registry policy-agnostic so different integrators can use
///   different thresholds against one shared attestation feed.
///
/// # Future-dated snapshots
/// If `snapshot.timestamp > now` (possible from clock drift between attesters),
/// the snapshot is accepted as fresh — `now - snapshot.timestamp` is gated on
/// `now > snapshot.timestamp` so the subtraction can never underflow.
fn get_validated_snapshot(
    env: &Env,
    liquidity_registry: &Address,
    asset: &Asset,
    config: &SafeOracleConfig,
) -> Result<Option<LiquiditySnapshot>, OracleSafetyViolation> {
    let asset_address = match asset {
        Asset::Stellar(addr) => addr.clone(),
        Asset::Other(_) => return Ok(None),
    };

    let registry_client = LiquidityRegistryClient::new(env, liquidity_registry);
    // Hardening Phase debt #4: graceful handling of `LiquidityRegistry`
    // contract trap. A registry failure (upgrade incompatibility, storage
    // corruption) becomes `ExternalContractFailure` rather than
    // propagating; integrators with `circuit_breaker_enabled = true` then
    // auto-halt on the failure, treating it as "no fresh evidence" the
    // same way Reflector failures are treated.
    let snapshot = match registry_client.try_get_snapshot(&asset_address) {
        Ok(Ok(Some(s))) => s,
        Ok(Ok(None)) => return Err(OracleSafetyViolation::InsufficientLiquidity),
        _ => return Err(OracleSafetyViolation::ExternalContractFailure),
    };

    let now = env.ledger().timestamp();
    if now > snapshot.timestamp {
        let age = now - snapshot.timestamp;
        if age > config.max_snapshot_age_seconds {
            return Err(OracleSafetyViolation::StaleSnapshot);
        }
    }

    Ok(Some(snapshot))
}

/// Layer 2, Guardrail 4 — Minimum SDEX Liquidity (Phase 4.1).
///
/// Threshold check on a snapshot already fetched + freshness-validated by
/// `get_validated_snapshot`. Rejects when the asset's 30-minute SDEX volume
/// is below `config.min_liquidity_usd`.
///
/// Structural defense against YieldBlox-class attacks: an attacker who can
/// move price with a $5 trade has — by definition — drained the order book
/// to near-zero, and this check blocks borrowing against such an unstable
/// feed even when Reflector reports a clean-looking price.
///
/// **Precision:** `volume_30m_usd` and `min_liquidity_usd` both use 7-decimal
/// USD (Stellar stroop convention) — direct `<` comparison without scaling.
/// See `LiquiditySnapshot` doc for the full precision convention. See
/// `get_validated_snapshot` for the skip and fail-safe semantics that produce
/// the snapshot reaching this function.
fn check_liquidity(
    snapshot: &LiquiditySnapshot,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    if snapshot.volume_30m_usd < config.min_liquidity_usd {
        return Err(OracleSafetyViolation::InsufficientLiquidity);
    }
    Ok(())
}

/// Layer 2, Guardrail 5 — Thin Sampling Detection (Phase 4.2).
///
/// Threshold check on a snapshot already fetched + freshness-validated by
/// `get_validated_snapshot`. Rejects when fewer than `config.min_trade_count_1h`
/// unique trades occurred in the past hour.
///
/// Defense against price manipulation in markets where trade frequency is
/// too low for VWAP/TWAP feeds to produce trustworthy prices. Even when
/// 30-minute volume passes `check_liquidity`, a market with only 1–2 trades
/// per hour is structurally vulnerable to single-trade manipulation — the
/// YieldBlox attacker had effectively one trade in the relevant pricing
/// window, and this guardrail catches that shape independently of the
/// volume threshold.
///
/// `unique_trades_1h` semantics (one trade per `source_account` per ledger,
/// $10 minimum sybil floor) are defined by `oracle-watch`; see spec §5
/// "Trade Counting Definition". See `get_validated_snapshot` for the skip and
/// fail-safe semantics that produce the snapshot reaching this function.
fn check_thin_sampling(
    snapshot: &LiquiditySnapshot,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation> {
    if snapshot.unique_trades_1h < config.min_trade_count_1h {
        return Err(OracleSafetyViolation::ThinSampling);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let cfg = SafeOracleConfig::default();
        assert_eq!(cfg.max_deviation_bps, 2000);
        assert_eq!(cfg.max_staleness_seconds, 300);
        assert_eq!(cfg.max_cross_source_bps, 500);
        assert_eq!(cfg.max_snapshot_age_seconds, 300);
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
        // AR.H L5 fix: ExternalContractFailure = 8 (Hardening 3C) regression guard.
        // The discriminant is correctly used in PriceResult::into_result and the
        // mock-lending mirror; the gap was solely in this regression test.
        assert_eq!(OracleSafetyViolation::ExternalContractFailure as u32, 8);
    }
}
