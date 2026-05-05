#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, IntoVal,
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
    /// Snapshot rejected because `timestamp > ledger_now +
    /// MAX_TIMESTAMP_SKEW_SECONDS`. Hardening Phase debt #5: a compromised
    /// attester writing `timestamp = u64::MAX` would otherwise permanently
    /// block all future snapshots (every later write is then stale relative
    /// to the poisoned `existing.timestamp`). The skew tolerance allows
    /// honest attester clock drift.
    FutureTimestamp = 9,
}

/// Maximum allowed clock skew between an attester's wall-clock time and
/// the on-chain ledger time when accepting a snapshot. A snapshot whose
/// `timestamp` exceeds `ledger_now + MAX_TIMESTAMP_SKEW_SECONDS` is
/// rejected as `FutureTimestamp` (Hardening Phase debt #5).
///
/// 300 seconds matches Reflector's mainnet resolution and the safe-oracle
/// `max_staleness_seconds` default — the same "this is how out of sync
/// honest feeds can plausibly be" budget applies to attesters.
const MAX_TIMESTAMP_SKEW_SECONDS: u64 = 300;

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
    /// 5. **Future-timestamp upper bound (Hardening Phase debt #5)** —
    ///    `snapshot.timestamp > ledger_now + MAX_TIMESTAMP_SKEW_SECONDS`
    ///    rejected as `FutureTimestamp`. Without this bound a compromised
    ///    attester could write `timestamp = u64::MAX`, permanently locking
    ///    out every subsequent write (each later one would compare against
    ///    the poisoned `existing.timestamp` and fail step 6 with
    ///    `StaleSnapshot`). The 300s skew matches honest attester clock
    ///    drift and Reflector's mainnet resolution.
    /// 6. Replay protection — strict greater-than on the previous timestamp
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

        // Hardening Phase debt #11: granular auth — attester's signature
        // is bound to the exact `snapshot` payload. A captured signature
        // for `{ volume: $5M, trades: 100 }` cannot be replayed for a
        // crafted `{ volume: $5, trades: 1 }` snapshot. Generic
        // `require_auth()` would approve any args under the same
        // signature; that is too coarse for write_snapshot, which is the
        // primary attack surface against Layer 2 attestations.
        attester.require_auth_for_args((snapshot.clone(),).into_val(&env));

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

        // Hardening Phase debt #5: reject far-future timestamps.
        // `saturating_add` clamps at `u64::MAX` so a `now` very close to
        // the upper bound cannot cause a subtle wraparound.
        let now = env.ledger().timestamp();
        let max_acceptable = now.saturating_add(MAX_TIMESTAMP_SKEW_SECONDS);
        if snapshot.timestamp > max_acceptable {
            return Err(LiquidityRegistryError::FutureTimestamp);
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
    use soroban_sdk::testutils::{Address as _, Ledger as _, MockAuth, MockAuthInvoke};

    fn setup() -> (Env, LiquidityRegistryClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();

        // Bump the ledger baseline well above the hard-coded snapshot
        // timestamps used by the rest of this test module (1_000_000 and
        // 2_000_000). Hardening Phase debt #5 rejects
        // `snapshot.timestamp > ledger_now + 300`; without this bump,
        // every fixture in this module would fail with `FutureTimestamp`
        // because the default `Env::default()` ledger time is 0.
        // 100_000_000 is comfortably past every test fixture, so all
        // existing timestamps fall in the "past" relative to ledger now —
        // they exercise replay protection (their original intent) without
        // triggering the new future-timestamp guard.
        env.ledger().with_mut(|li| {
            li.timestamp = 100_000_000;
        });

        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.initialize(&admin);

        (env, client, admin)
    }

    #[test]
    fn test_initialize_sets_admin() {
        let env = Env::default();
        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        // Hardening Phase debt #8: granular auth — only admin's signature
        // is approved, and only for the specific `initialize(admin)`
        // invocation. Generic `mock_all_auths()` would have approved any
        // address signing any call; this declaration pins admin auth as
        // the only one needed.
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(),).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        client.initialize(&admin);
    }

    #[test]
    fn test_initialize_twice_returns_already_initialized() {
        let env = Env::default();
        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        // Hardening Phase debt #8: granular auth — admin1's signature
        // covers the first `initialize` call. The second call short-
        // circuits at `AlreadyInitialized` *before* reaching
        // `require_auth()`, so admin2 deliberately has no entry here —
        // pinning that the rejection happens on the storage check, not
        // because admin2's signature was missing.
        env.mock_auths(&[MockAuth {
            address: &admin1,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin1.clone(),).into_val(&env),
                sub_invokes: &[],
            },
        }]);

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
        // No `mock_auths` needed: `add_attester` returns `NotInitialized`
        // before reaching the `admin.require_auth()` call. Hardening 4
        // (#8) cleanup — removed the dead `mock_all_auths` call so the
        // test reads as "auth never enters the picture for this path".

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

    /// Two whitelisted attesters writing to the same asset in succession: the
    /// later (newer-timestamp) write must overwrite the earlier one, and the
    /// stored `attester` field must reflect the *latest* writer. Replay
    /// protection compares against the asset's prior snapshot regardless of
    /// who wrote it, so timestamp ordering is global per asset, not per
    /// attester. This complements the single-attester replay tests above.
    #[test]
    fn test_write_snapshot_multi_attester_succession() {
        let (env, client, _admin) = setup();

        let attester_a = Address::generate(&env);
        let attester_b = Address::generate(&env);
        client.add_attester(&attester_a);
        client.add_attester(&attester_b);

        let asset = Address::generate(&env);

        let snapshot_a = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: 1_000_000,
            attester: attester_a.clone(),
        };
        client.write_snapshot(&attester_a, &snapshot_a);

        let snapshot_b = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 2_000_000_000,
            unique_trades_1h: 50,
            timestamp: 2_000_000,
            attester: attester_b.clone(),
        };
        client.write_snapshot(&attester_b, &snapshot_b);

        let read_back = client.get_snapshot(&asset).unwrap();
        assert_eq!(
            read_back.attester, attester_b,
            "latest writer's attester should be in snapshot"
        );
        assert_eq!(read_back.timestamp, 2_000_000);
        assert_eq!(read_back.volume_30m_usd, 2_000_000_000);
    }

    #[test]
    fn test_get_snapshot_returns_none_when_not_initialized() {
        let env = Env::default();
        // No `mock_auths` needed: `get_snapshot` is read-only and never
        // invokes `require_auth()`. Hardening 4 (#8) cleanup — removed
        // the dead `mock_all_auths` call so the test reads as
        // "auth never enters the picture for read paths".

        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let asset = Address::generate(&env);

        let result = client.get_snapshot(&asset);
        assert!(
            result.is_none(),
            "uninitialized contract must return None for read"
        );
    }

    /// Hardening 4 (#8) boundary test: `initialize(admin)` calls
    /// `admin.require_auth()`. When `admin`'s signature is *not* in the
    /// mock-auths declaration, the host treats the auth check as missing
    /// and the invocation traps — observable as `Err(_)` from `try_*`.
    /// Pins that the auth gate is real, not a cosmetic call.
    #[test]
    fn test_initialize_rejects_unauthorized_signer() {
        let env = Env::default();
        let contract_id = env.register(LiquidityRegistry, ());
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let unauthorized = Address::generate(&env);

        // Only `unauthorized` is declared as a signer. Admin's
        // require_auth() call has no matching MockAuth and therefore
        // fails the host-side auth check.
        env.mock_auths(&[MockAuth {
            address: &unauthorized,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(),).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        let result = client.try_initialize(&admin);
        assert!(
            result.is_err(),
            "init must fail when admin's signature is not provided: {:?}",
            result
        );
    }

    // ===== Hardening Phase debt #5: future-timestamp DoS protection =====

    /// `snapshot.timestamp > now + MAX_TIMESTAMP_SKEW_SECONDS` must be
    /// rejected with `FutureTimestamp`. Without this guard a compromised
    /// attester could pin every future write into `StaleSnapshot` by
    /// landing a single far-future timestamp first.
    #[test]
    fn test_write_snapshot_rejects_far_future_timestamp() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);
        // setup() pinned `ledger_now` at 100_000_000; skew is 300, so
        // anything past `100_000_300` must be rejected. Pick a value
        // 10_000s past the skew window.
        let far_future = env.ledger().timestamp() + 10_000;
        let snapshot = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: far_future,
            attester: attester.clone(),
        };

        let result = client.try_write_snapshot(&attester, &snapshot);
        assert_eq!(
            result,
            Err(Ok(LiquidityRegistryError::FutureTimestamp)),
            "far-future timestamp must be rejected (Hardening debt #5)"
        );
    }

    /// Direct DoS attack: attester writes `timestamp = u64::MAX`. The
    /// `saturating_add` inside the guard clamps cleanly so the comparison
    /// fires regardless of how close `now` is to `u64::MAX`.
    #[test]
    fn test_write_snapshot_rejects_u64_max_timestamp() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);
        let snapshot = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: u64::MAX,
            attester: attester.clone(),
        };

        let result = client.try_write_snapshot(&attester, &snapshot);
        assert_eq!(
            result,
            Err(Ok(LiquidityRegistryError::FutureTimestamp)),
            "u64::MAX timestamp must be rejected — direct DoS vector"
        );
    }

    /// Regression guard: clock-drift within `MAX_TIMESTAMP_SKEW_SECONDS`
    /// is accepted. Honest attesters running slightly ahead of the
    /// validator-consensus clock must not be locked out.
    #[test]
    fn test_write_snapshot_accepts_within_skew_tolerance() {
        let (env, client, _admin) = setup();
        let attester = Address::generate(&env);
        client.add_attester(&attester);

        let asset = Address::generate(&env);
        // 100s in the future — well within the 300s skew tolerance.
        let slightly_future = env.ledger().timestamp() + 100;
        let snapshot = LiquiditySnapshot {
            asset: asset.clone(),
            volume_30m_usd: 1_000_000_000,
            unique_trades_1h: 42,
            timestamp: slightly_future,
            attester: attester.clone(),
        };

        // No `try_*`: a panic here would surface the guard misfiring.
        client.write_snapshot(&attester, &snapshot);

        let read_back = client.get_snapshot(&asset).unwrap();
        assert_eq!(read_back.timestamp, slightly_future);
    }
}
