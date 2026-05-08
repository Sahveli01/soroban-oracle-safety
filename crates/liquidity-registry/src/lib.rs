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
    /// Retained for audit-trail continuity; no longer reachable from any
    /// contract entry point after Hardening 6B's CAP-0058
    /// `__constructor` migration (debt #10) — constructors cannot be
    /// invoked twice, so the previous re-init guard was removed.
    #[allow(dead_code)]
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

/// Phase 7.1: TTL extension constants for snapshot persistence.
///
/// Soroban persistent-storage entries expire after their TTL elapses and are
/// archived off-chain (`MIN_PERSISTENT_ENTRY_TTL` ledgers post-write); without
/// `extend_ttl` calls, snapshots silently disappear on testnet/mainnet,
/// breaking integrator reads.
///
/// `SNAPSHOT_TTL_MIN` is the trigger threshold: when an entry's remaining TTL
/// drops below this many ledgers, the next write/read extends it.
/// `SNAPSHOT_TTL_EXTEND` is the post-extension target.
///
/// Sizing assumes oracle-watch attests every ~5 minutes (60 ledgers @ 5 s/ledger).
/// The 24h baseline (17_280 ledgers) leaves ~290 attestation cycles of buffer
/// between extends, so the amortized cost per write is one extend every
/// ~290 attestations.
const SNAPSHOT_TTL_MIN: u32 = 100;
const SNAPSHOT_TTL_EXTEND: u32 = 17_280;

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
    /// Initialize the liquidity registry with an admin address —
    /// CAP-0058 `__constructor`.
    ///
    /// The admin manages the attester whitelist (Phase 3.2).
    ///
    /// # Hardening Phase debt #10 (CAP-0058 migration)
    ///
    /// Replaces the previous `pub fn initialize(...)` two-step deploy +
    /// init flow. Init args are now passed at deploy time
    /// (`env.register(LiquidityRegistry, (admin,))`); the constructor
    /// runs atomically with deploy, eliminating the re-init attack
    /// surface that the previous `if has(Admin) -> AlreadyInitialized`
    /// guard defended against. The `LiquidityRegistryError::AlreadyInitialized`
    /// variant is retained for audit-history continuity but no longer
    /// reachable from this contract.
    pub fn __constructor(env: Env, admin: Address) -> Result<(), LiquidityRegistryError> {
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
    /// Snapshots are stored in `persistent` storage keyed by asset. Phase 7.1
    /// extends TTL on every successful write; see `SNAPSHOT_TTL_*` constants
    /// for sizing rationale.
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
        // Phase 7.1: extend persistent TTL on write — keeps snapshots alive
        // for ~24h baseline. Frequent attestations (every ~5min) trigger
        // re-extends well before the 100-ledger threshold is hit.
        env.storage()
            .persistent()
            .extend_ttl(&snapshot_key, SNAPSHOT_TTL_MIN, SNAPSHOT_TTL_EXTEND);

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
    /// Phase 7.1 defensively extends TTL on every successful read so that even
    /// when attestations slow down, integrator reads keep recent snapshots
    /// alive long enough for ops to react.
    pub fn get_snapshot(env: Env, asset: Address) -> Option<LiquiditySnapshot> {
        let snapshot_key = DataKey::Snapshot(asset);
        let snapshot: Option<LiquiditySnapshot> = env.storage().persistent().get(&snapshot_key);
        if snapshot.is_some() {
            // Phase 7.1: defensive extend on read — if attesters slow down or
            // halt entirely, integrator reads still keep recent snapshots
            // alive long enough for ops to react.
            env.storage().persistent().extend_ttl(
                &snapshot_key,
                SNAPSHOT_TTL_MIN,
                SNAPSHOT_TTL_EXTEND,
            );
        }
        snapshot
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

        // Hardening 6B (#10): CAP-0058 — admin passed at register time,
        // `__constructor` runs atomically. No separate `initialize` call.
        let admin = Address::generate(&env);
        let contract_id = env.register(LiquidityRegistry, (admin.clone(),));
        let client = LiquidityRegistryClient::new(&env, &contract_id);

        (env, client, admin)
    }

    #[test]
    fn test_constructor_sets_admin() {
        // Hardening 6B (#10): CAP-0058 — admin auth is verified during
        // `env.register` (constructor invocation), not in a separate
        // `initialize` call. Granular `mock_auths` (Hardening 4 #8)
        // approves only admin's signature, and only for the constructor
        // call shape, demonstrating the auth gate is real.
        let env = Env::default();
        let admin = Address::generate(&env);
        // Pre-compute the contract address that `register` will assign
        // so we can declare auths bound to it. `register_at` is the
        // SDK-supported way to register at a specific address; here we
        // just use the address space generator and let `register` pick
        // its own (the auth check matches by address, not contract ID).
        let pre_addr = Address::generate(&env);
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pre_addr,
                fn_name: "__constructor",
                args: (admin.clone(),).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        // `register_at` honors the pre-computed address.
        let contract_id = env.register_at(&pre_addr, LiquidityRegistry, (admin.clone(),));
        let _client = LiquidityRegistryClient::new(&env, &contract_id);
        // No assertion needed — `register` would have trapped on auth
        // failure or constructor Err. Reaching this line is the success
        // signal.
    }

    // Pre-Hardening 6B `test_initialize_twice_returns_already_initialized`
    // was deleted: a CAP-0058 `__constructor` cannot be invoked twice on
    // the same contract, so the `AlreadyInitialized` re-init path is
    // unreachable. The error variant is retained at
    // `LiquidityRegistryError::AlreadyInitialized` (#[allow(dead_code)])
    // for audit-history continuity but is no longer exercised by tests.

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
        // Hardening 6B (#10) CAP-0058 update: a registered contract is,
        // by construction, initialized — `__constructor(admin)` runs
        // during `env.register`, and `register` traps if the args are
        // missing. The pre-Hardening 6B premise of "register without
        // init, then call add_attester to observe NotInitialized" is
        // unrepresentable in the new model.
        //
        // The test body is kept as a placeholder that asserts trivial
        // truth, so the function name remains as audit-trail of the
        // historical guard. Remove or repurpose during a later
        // Hardening Closure cleanup if the placeholder is judged
        // misleading.
        let _ = LiquidityRegistryError::NotInitialized;
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
        // Hardening 6B (#10) CAP-0058 update: with `__constructor`, an
        // "uninitialized" contract is no longer representable — register
        // includes init. The test now exercises the equivalent post-init
        // case: a properly registered contract returns `None` for an
        // asset that has never had a snapshot written. (The
        // pre-Hardening 6B premise — `get_snapshot` on an uninitialized
        // contract — was deleted with CAP-0058 migration; this assertion
        // is preserved as a read-on-fresh-contract regression guard.)
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(LiquidityRegistry, (admin,));
        let client = LiquidityRegistryClient::new(&env, &contract_id);
        let asset = Address::generate(&env);

        let result = client.get_snapshot(&asset);
        assert!(
            result.is_none(),
            "fresh contract must return None for an unwritten asset"
        );
    }

    /// Hardening 4 (#8) boundary test, refactored for Hardening 6B
    /// (#10) CAP-0058 model: `__constructor(admin)` calls
    /// `admin.require_auth()` during `env.register`. When `admin`'s
    /// signature is *not* in the mock-auths declaration, the host-side
    /// auth check fails and the registration traps — observable here
    /// as a panic.
    #[test]
    #[should_panic]
    fn test_initialize_rejects_unauthorized_signer() {
        // Hardening 6B (#10) — CAP-0058 update: with `__constructor`,
        // admin's `require_auth()` runs during `env.register`. Without a
        // matching `MockAuth` for `admin`, the host auth check fails and
        // the registration traps. Asserted via `#[should_panic]`.
        let env = Env::default();
        let admin = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        let pre_addr = Address::generate(&env);

        // Only `unauthorized` is declared as a signer. Admin's
        // `require_auth` call inside `__constructor` has no matching
        // MockAuth → host-side auth check fails → register panics.
        env.mock_auths(&[MockAuth {
            address: &unauthorized,
            invoke: &MockAuthInvoke {
                contract: &pre_addr,
                fn_name: "__constructor",
                args: (admin.clone(),).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        let _addr = env.register_at(&pre_addr, LiquidityRegistry, (admin,));
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
