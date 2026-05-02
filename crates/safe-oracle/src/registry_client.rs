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
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquiditySnapshot {
    pub asset: Address,
    pub volume_30m_usd: i128,
    pub unique_trades_1h: u32,
    pub timestamp: u64,
    pub attester: Address,
}
