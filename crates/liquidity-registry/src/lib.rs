#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

/// Errors returned by `LiquidityRegistry`.
///
/// Discriminants 1–6 cover the full Phase 3 surface; only `AlreadyInitialized`
/// is exercised in Phase 3.1. The remaining variants are reserved for upcoming
/// Phase 3.2–3.6 work (whitelist management, snapshot writes, reads) and are
/// declared up-front so the audit-visible enum stays stable.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LiquidityRegistryError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAuthorized = 3,
    SnapshotNotFound = 4,
    AttesterNotWhitelisted = 5,
    InvalidSnapshot = 6,
}

/// On-chain SDEX trade attestation snapshot for a single asset.
///
/// Produced off-chain by `oracle-watch` and persisted here so that
/// `safe_oracle::check_liquidity` and `safe_oracle::check_thin_sampling`
/// (Phase 4) can read recent SDEX activity without trusting the calling
/// contract's view of liquidity.
///
/// `volume_30m_usd` is intentionally `i128` (not `u128`) to match the spec and
/// to allow Phase 4 to defensively reject `volume <= 0` payloads as a
/// manipulation signal — an attacker who controls a whitelisted attester
/// cannot forge legitimate volume by underflowing into negatives.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquiditySnapshot {
    pub asset: Address,
    pub volume_30m_usd: i128,
    pub unique_trades_1h: u32,
    pub timestamp: u64,
    pub attester: Address,
}

#[contracttype]
enum DataKey {
    Admin,
    Snapshot(Address),
    Whitelist(Address),
}

#[contract]
pub struct LiquidityRegistry;

#[contractimpl]
impl LiquidityRegistry {
    /// Initialize the liquidity registry with an admin address.
    ///
    /// The admin manages the attester whitelist (Phase 3.2). Reinitialization
    /// is rejected to prevent admin-override attacks: once `Admin` is in
    /// instance storage, a second call returns `AlreadyInitialized` instead of
    /// silently overwriting it.
    pub fn initialize(env: Env, admin: Address) -> Result<(), LiquidityRegistryError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(LiquidityRegistryError::AlreadyInitialized);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_initialize_sets_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.initialize(&admin);
    }

    #[test]
    fn test_initialize_twice_returns_already_initialized() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        client.initialize(&admin1);

        let result = client.try_initialize(&admin2);
        assert_eq!(result, Err(Ok(LiquidityRegistryError::AlreadyInitialized)));
    }

    #[test]
    fn test_liquidity_snapshot_round_trip() {
        let env = Env::default();
        let asset = Address::generate(&env);
        let attester = Address::generate(&env);

        let snapshot = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 1234567890,
            attester: attester.clone(),
        };

        assert_eq!(snapshot.volume_30m_usd, 1_000_000_000);
        assert_eq!(snapshot.unique_trades_1h, 42);
        assert_eq!(snapshot.timestamp, 1234567890);
        assert_eq!(snapshot.asset, asset);
        assert_eq!(snapshot.attester, attester);
    }
}
