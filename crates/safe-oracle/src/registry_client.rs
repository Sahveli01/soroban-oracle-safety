use soroban_sdk::{contractclient, contracttype, Address, Env};

/// Cross-contract client trait for `LiquidityRegistry`.
///
/// The auto-generated `LiquidityRegistryClient` is consumed by Phase 4's
/// `check_liquidity` and `check_thin_sampling` to read SDEX trade
/// attestations. Defining the binding here (rather than depending on the
/// `liquidity-registry` crate directly) keeps `safe-oracle` decoupled at link
/// time and matches the `ReflectorClient` pattern already in use for the
/// oracle binding.
///
/// Only `get_snapshot` is mirrored: `safe-oracle` is a read-side consumer of
/// the registry — write-side methods (`initialize`, `add_attester`,
/// `remove_attester`, `write_snapshot`) belong to admin/attester tooling and
/// have no place in the guardrail call path.
// The trait exists solely so `#[contractclient]` can synthesize the client
// struct; nothing calls it directly.
#[allow(dead_code)]
#[contractclient(name = "LiquidityRegistryClient")]
pub trait LiquidityRegistry {
    fn get_snapshot(env: Env, asset: Address) -> Option<LiquiditySnapshot>;
}

/// Mirror of `liquidity-registry::LiquiditySnapshot`.
///
/// Defined independently here to avoid making `safe-oracle` depend on the
/// `liquidity-registry` crate. Soroban's contracttype serialization is
/// structural, so a snapshot written by the contract round-trips into this
/// type as long as the field order, names, and types stay aligned. Any change
/// to the registry's snapshot shape must be mirrored here in lockstep — Phase
/// 4 tests will catch a drift via integration round-trip.
///
/// **Precision:** All USD-denominated fields use **7-decimal precision**
/// (Stellar stroop convention). 1 USD = 10_000_000 (10^7). This matches
/// `SafeOracleConfig::min_liquidity_usd` for direct comparison without scaling.
/// Reflector uses 14-decimal precision for *prices*, but liquidity volumes are
/// dollar-denominated and follow the project-wide 7-decimal convention.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquiditySnapshot {
    /// Asset this snapshot describes.
    pub asset: Address,
    /// SDEX trading volume during the last 30 minutes, denominated in USD with
    /// 7-decimal precision. Example: $50,000 = `500_000_000_000`.
    pub volume_30m_usd: i128,
    /// Count of unique trades on SDEX during the last 1 hour. Used by
    /// `safe_oracle::check_thin_sampling` (Layer 2 Guardrail #5) to reject
    /// markets where price discovery is too thin to trust.
    pub unique_trades_1h: u32,
    /// Snapshot creation time, in Unix seconds (matching ledger timestamp).
    /// Consumers compare against `env.ledger().timestamp()` and their own
    /// `max_snapshot_age_seconds` threshold to enforce freshness.
    pub timestamp: u64,
    /// Address that wrote this snapshot. Must be in the attester whitelist;
    /// equality with the caller is enforced in `write_snapshot`.
    pub attester: Address,
}
