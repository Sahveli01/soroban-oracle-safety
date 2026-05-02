#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env,
};

/// Errors returned by `LiquidityRegistry`.
///
/// Discriminants 1–8 cover the Phase 3 write surface. Phase 3.3 exercises
/// `NotInitialized`, `AttesterNotWhitelisted`, `InvalidSnapshot`, and
/// `StaleSnapshot` via `write_snapshot`. `SnapshotNotFound` is reserved for
/// the Phase 3.5 read path. The enum is extended in place rather than split
/// into a new error type to keep one audit-visible error surface.
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
    StaleSnapshot = 8,
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

/// Emitted when a whitelisted attester writes a snapshot.
///
/// `asset` and `attester` are topics so off-chain consumers can subscribe to a
/// single asset's attestation stream or audit a specific attester's writes
/// without scanning the full event log.
#[contractevent]
#[derive(Clone, Debug)]
pub struct SnapshotWritten {
    #[topic]
    pub asset: Address,
    #[topic]
    pub attester: Address,
    pub volume_30m_usd: i128,
    pub unique_trades_1h: u32,
    pub timestamp: u64,
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

    /// Write a liquidity snapshot for an asset. Only whitelisted attesters can call.
    ///
    /// Validation order is deliberate:
    /// 1. `NotInitialized` — admin storage missing means no whitelist exists,
    ///    so no caller can possibly be authorized; fail before touching auth.
    /// 2. `attester.require_auth()` — Soroban-level signature check.
    /// 3. Whitelist membership — `AttesterNotWhitelisted` if the signer isn't
    ///    authorized to attest.
    /// 4. Payload integrity — `volume_30m_usd > 0` (defensive: a malicious or
    ///    buggy attester cannot land a negative/zero volume that would later
    ///    underflow `check_liquidity`'s comparisons), and the `attester` field
    ///    on the snapshot must equal the calling attester (prevents one
    ///    whitelisted attester from forging another's signed attestation).
    /// 5. Replay protection — strict greater-than on the previous timestamp
    ///    rejects both stale resubmissions and equal-timestamp double-writes.
    ///
    /// Snapshots are stored in `persistent` storage keyed by asset; production
    /// deployments must call `extend_ttl` here (Phase 8 deployment work).
    pub fn write_snapshot(
        env: Env,
        attester: Address,
        snapshot: LiquiditySnapshot,
    ) -> Result<(), LiquidityRegistryError> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(LiquidityRegistryError::NotInitialized);
        }

        attester.require_auth();

        let whitelist_key = DataKey::Whitelist(attester.clone());
        if !env.storage().instance().has(&whitelist_key) {
            return Err(LiquidityRegistryError::AttesterNotWhitelisted);
        }

        if snapshot.volume_30m_usd <= 0 {
            return Err(LiquidityRegistryError::InvalidSnapshot);
        }
        if snapshot.attester != attester {
            return Err(LiquidityRegistryError::InvalidSnapshot);
        }

        let snapshot_key = DataKey::Snapshot(snapshot.asset.clone());
        if let Some(existing) = env
            .storage()
            .persistent()
            .get::<DataKey, LiquiditySnapshot>(&snapshot_key)
        {
            if snapshot.timestamp <= existing.timestamp {
                return Err(LiquidityRegistryError::StaleSnapshot);
            }
        }

        env.storage().persistent().set(&snapshot_key, &snapshot);
        // TODO: extend_ttl in production (Phase 8 deployment).

        SnapshotWritten {
            asset: snapshot.asset,
            attester,
            volume_30m_usd: snapshot.volume_30m_usd,
            unique_trades_1h: snapshot.unique_trades_1h,
            timestamp: snapshot.timestamp,
        }
        .publish(&env);

        Ok(())
    }

    /// Read the most recent snapshot for an asset.
    ///
    /// Returns `None` when no snapshot has been written for this asset,
    /// including the pre-initialize case (storage is empty either way). This
    /// is the conservative answer for a read query — callers decide what
    /// "no snapshot" means in their domain (Phase 4's `check_liquidity` treats
    /// it as `InsufficientLiquidity`).
    ///
    /// **Freshness is intentionally not checked here.** This function returns
    /// the raw stored value; consumers compare `snapshot.timestamp` against
    /// `env.ledger().timestamp()` to enforce their own staleness threshold
    /// (e.g., `safe_oracle::SafeOracleConfig::max_staleness_seconds`). Keeping
    /// the registry policy-agnostic lets multiple integrators share one
    /// attestation feed with different freshness requirements.
    ///
    /// Production deployment must call `extend_ttl` on read paths to prevent
    /// silent expiration of fresh snapshots between attestations (Phase 8
    /// deployment work).
    pub fn get_snapshot(env: Env, asset: Address) -> Option<LiquiditySnapshot> {
        env.storage()
            .persistent()
            .get::<DataKey, LiquiditySnapshot>(&DataKey::Snapshot(asset))
        // TODO: extend_ttl on read in production (Phase 8 deployment).
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

    #[test]
    fn test_write_snapshot_success_first_write() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);
        let snapshot = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };

        client.write_snapshot(&attester, &snapshot);
    }

    #[test]
    fn test_write_snapshot_success_replaces_older() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);

        let old_snapshot = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };
        client.write_snapshot(&attester, &old_snapshot);

        let new_snapshot = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 2_000_000_000,
            unique_trades_1h: 50,
            timestamp: 2_000_000,
            attester: attester.clone(),
        };
        client.write_snapshot(&attester, &new_snapshot);
    }

    #[test]
    fn test_write_snapshot_fails_when_attester_not_whitelisted() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);

        let asset = Address::generate(&env);
        let snapshot = LiquiditySnapshot {
            asset,
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };

        let result = client.try_write_snapshot(&attester, &snapshot);
        assert_eq!(
            result,
            Err(Ok(LiquidityRegistryError::AttesterNotWhitelisted))
        );
    }

    #[test]
    fn test_write_snapshot_fails_with_negative_volume() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);
        let snapshot = LiquiditySnapshot {
            asset,
            volume_30m_usd: -1,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };

        let result = client.try_write_snapshot(&attester, &snapshot);
        assert_eq!(result, Err(Ok(LiquidityRegistryError::InvalidSnapshot)));
    }

    #[test]
    fn test_write_snapshot_fails_with_zero_volume() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);
        let snapshot = LiquiditySnapshot {
            asset,
            volume_30m_usd: 0,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };

        let result = client.try_write_snapshot(&attester, &snapshot);
        assert_eq!(result, Err(Ok(LiquidityRegistryError::InvalidSnapshot)));
    }

    #[test]
    fn test_write_snapshot_fails_with_mismatched_attester_field() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        let other = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);
        let snapshot = LiquiditySnapshot {
            asset,
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: other,
        };

        let result = client.try_write_snapshot(&attester, &snapshot);
        assert_eq!(result, Err(Ok(LiquidityRegistryError::InvalidSnapshot)));
    }

    #[test]
    fn test_write_snapshot_fails_with_stale_timestamp() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);

        let snapshot1 = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 2_000_000,
            attester: attester.clone(),
        };
        client.write_snapshot(&attester, &snapshot1);

        let snapshot2 = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 2_000_000_000,
            unique_trades_1h: 50,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };

        let result = client.try_write_snapshot(&attester, &snapshot2);
        assert_eq!(result, Err(Ok(LiquidityRegistryError::StaleSnapshot)));
    }

    #[test]
    fn test_write_snapshot_fails_with_equal_timestamp() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);

        let snapshot1 = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };
        client.write_snapshot(&attester, &snapshot1);

        let snapshot2 = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 5_000_000_000,
            unique_trades_1h: 99,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };

        let result = client.try_write_snapshot(&attester, &snapshot2);
        assert_eq!(
            result,
            Err(Ok(LiquidityRegistryError::StaleSnapshot)),
            "equal timestamp must be rejected for replay protection"
        );
    }

    #[test]
    fn test_get_snapshot_returns_none_when_no_snapshot() {
        let (env, client, _admin) = setup();
        let asset = Address::generate(&env);

        let result = client.get_snapshot(&asset);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_snapshot_returns_written_snapshot() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);
        let snapshot = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };
        client.write_snapshot(&attester, &snapshot);

        let read_back = client.get_snapshot(&asset).unwrap();
        assert_eq!(read_back.asset, snapshot.asset);
        assert_eq!(read_back.volume_30m_usd, snapshot.volume_30m_usd);
        assert_eq!(read_back.unique_trades_1h, snapshot.unique_trades_1h);
        assert_eq!(read_back.timestamp, snapshot.timestamp);
        assert_eq!(read_back.attester, snapshot.attester);
    }

    #[test]
    fn test_get_snapshot_returns_latest_after_overwrite() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);

        let snapshot1 = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };
        client.write_snapshot(&attester, &snapshot1);

        let snapshot2 = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 2_000_000_000,
            unique_trades_1h: 50,
            timestamp: 2_000_000,
            attester: attester.clone(),
        };
        client.write_snapshot(&attester, &snapshot2);

        let read_back = client.get_snapshot(&asset).unwrap();
        assert_eq!(read_back.timestamp, 2_000_000);
        assert_eq!(read_back.volume_30m_usd, 2_000_000_000);
        assert_eq!(read_back.unique_trades_1h, 50);
    }

    #[test]
    fn test_get_snapshot_isolates_by_asset() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset_a = Address::generate(&env);
        let asset_b = Address::generate(&env);

        let snapshot_a = LiquiditySnapshot {
            asset: asset_a.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 10,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };
        let snapshot_b = LiquiditySnapshot {
            asset: asset_b.clone(),
            volume_30m_usd: 5_000_000_000,
            unique_trades_1h: 100,
            timestamp: 1_000_000,
            attester: attester.clone(),
        };

        client.write_snapshot(&attester, &snapshot_a);
        client.write_snapshot(&attester, &snapshot_b);

        let read_a = client.get_snapshot(&asset_a).unwrap();
        let read_b = client.get_snapshot(&asset_b).unwrap();

        assert_eq!(read_a.volume_30m_usd, 1_000_000_000);
        assert_eq!(read_b.volume_30m_usd, 5_000_000_000);
        assert_ne!(read_a.asset, read_b.asset);
    }

    #[test]
    fn test_get_snapshot_returns_none_when_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let asset = Address::generate(&env);

        let result = client.get_snapshot(&asset);
        assert!(
            result.is_none(),
            "uninitialized contract must return None for read"
        );
    }
}
