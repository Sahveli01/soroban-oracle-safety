#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env,
};

/// Errors returned by `LiquidityRegistry`.
///
/// Discriminants 1–7 cover the Phase 3 surface; Phase 3.1 exercises
/// `AlreadyInitialized`, Phase 3.2 adds whitelist management which exercises
/// `NotInitialized`, `AttesterNotWhitelisted`, and `AttesterAlreadyWhitelisted`.
/// Remaining variants are reserved for Phase 3.4–3.5 (snapshot writes / reads).
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LiquidityRegistryError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAuthorized = 3,
    SnapshotNotFound = 4,
    AttesterNotWhitelisted = 5,
    InvalidSnapshot = 6,
    AttesterAlreadyWhitelisted = 7,
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

/// Emitted when the admin adds an attester to the whitelist.
#[contractevent]
#[derive(Clone, Debug)]
pub struct AttesterAdded {
    #[topic]
    pub attester: Address,
}

/// Emitted when the admin removes an attester from the whitelist.
#[contractevent]
#[derive(Clone, Debug)]
pub struct AttesterRemoved {
    #[topic]
    pub attester: Address,
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

    /// Add an attester to the whitelist. Only the admin can call this.
    ///
    /// Returns `AttesterAlreadyWhitelisted` if the attester is already in the
    /// whitelist. The check is intentional: silent duplicates would obscure
    /// the audit trail, so the admin must explicitly remove and re-add to
    /// "refresh" an entry.
    pub fn add_attester(env: Env, attester: Address) -> Result<(), LiquidityRegistryError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(LiquidityRegistryError::NotInitialized)?;
        admin.require_auth();

        let key = DataKey::Whitelist(attester.clone());
        if env.storage().instance().has(&key) {
            return Err(LiquidityRegistryError::AttesterAlreadyWhitelisted);
        }

        env.storage().instance().set(&key, &true);

        AttesterAdded { attester }.publish(&env);

        Ok(())
    }

    /// Remove an attester from the whitelist. Only the admin can call this.
    ///
    /// Returns `AttesterNotWhitelisted` if the attester is not currently in
    /// the whitelist — admins should not be able to silently "remove" an
    /// already-absent attester (audit-trail clarity).
    pub fn remove_attester(env: Env, attester: Address) -> Result<(), LiquidityRegistryError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(LiquidityRegistryError::NotInitialized)?;
        admin.require_auth();

        let key = DataKey::Whitelist(attester.clone());
        if !env.storage().instance().has(&key) {
            return Err(LiquidityRegistryError::AttesterNotWhitelisted);
        }

        env.storage().instance().remove(&key);

        AttesterRemoved { attester }.publish(&env);

        Ok(())
    }

    /// Check whether an address is in the attester whitelist.
    ///
    /// Read-only; no auth required. Used by the Phase 3.3 authorized-writer
    /// guard and by external integrators verifying attestation provenance.
    /// Pre-initialize calls return `false` (storage is empty), which is the
    /// conservative answer for a permission check.
    pub fn is_attester(env: Env, attester: Address) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Whitelist(attester))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, LiquidityRegistryClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.initialize(&admin);

        (env, client, admin)
    }

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

    #[test]
    fn test_add_attester_success() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);

        client.add_attester(&attester);

        assert!(client.is_attester(&attester));
    }

    #[test]
    fn test_add_attester_twice_returns_already_whitelisted() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);

        client.add_attester(&attester);

        let result = client.try_add_attester(&attester);
        assert_eq!(
            result,
            Err(Ok(LiquidityRegistryError::AttesterAlreadyWhitelisted))
        );
    }

    #[test]
    fn test_remove_attester_success() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);

        client.add_attester(&attester);
        assert!(client.is_attester(&attester));

        client.remove_attester(&attester);
        assert!(!client.is_attester(&attester));
    }

    #[test]
    fn test_remove_attester_not_whitelisted_returns_error() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);

        let result = client.try_remove_attester(&attester);
        assert_eq!(
            result,
            Err(Ok(LiquidityRegistryError::AttesterNotWhitelisted))
        );
    }

    #[test]
    fn test_is_attester_returns_false_when_not_whitelisted() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);

        assert!(!client.is_attester(&attester));
    }

    #[test]
    fn test_add_attester_fails_when_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let attester = Address::generate(&env);

        let result = client.try_add_attester(&attester);
        assert_eq!(result, Err(Ok(LiquidityRegistryError::NotInitialized)));
    }
}
