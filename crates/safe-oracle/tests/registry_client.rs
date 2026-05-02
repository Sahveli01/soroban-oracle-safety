//! Integration tests for the `LiquidityRegistry` cross-contract binding.
//!
//! Phase 3.6 wires `LiquidityRegistryClient` into `safe-oracle`'s public API
//! so Phase 4's `check_liquidity` and `check_thin_sampling` can read snapshot
//! data without taking a link-time dependency on `liquidity-registry`. These
//! tests exercise the round trip end-to-end through `TestEnv`'s registry
//! plumbing — the auto-generated client in `liquidity-registry` and the
//! mirror trait in `safe-oracle::registry_client` target the same on-chain
//! contract, so a successful round trip here also proves the safe-oracle-side
//! binding will deserialize correctly when Phase 4 invokes it.

use soroban_sdk::{testutils::Address as _, Address};
use test_utils::TestEnv;

/// Reading an unwritten asset returns `None` — the `Option` shape that
/// `check_liquidity` will pattern-match against to decide between
/// `InsufficientLiquidity` (no snapshot) and a real liquidity comparison.
#[test]
fn test_registry_client_returns_none_for_missing_snapshot() {
    let test_env = TestEnv::new();
    let asset = Address::generate(&test_env.env);

    let result = test_env.registry_client.get_snapshot(&asset);

    assert!(result.is_none(), "missing snapshot should return None");
}

/// A snapshot written through the default whitelisted attester round-trips
/// back through `get_snapshot` with field-level equality. This is the
/// guarantee Phase 4's Layer 2 guardrails rely on — both `volume_30m_usd`
/// (for `check_liquidity`) and `unique_trades_1h` (for `check_thin_sampling`)
/// must survive the cross-contract serialization unchanged.
#[test]
fn test_registry_client_returns_written_snapshot() {
    let test_env = TestEnv::new();
    let asset = Address::generate(&test_env.env);

    test_env.write_snapshot_now(&asset, 1_000_000_000, 42);

    let snapshot = test_env
        .registry_client
        .get_snapshot(&asset)
        .expect("snapshot must be present after write");

    assert_eq!(snapshot.asset, asset);
    assert_eq!(snapshot.volume_30m_usd, 1_000_000_000);
    assert_eq!(snapshot.unique_trades_1h, 42);
    assert_eq!(snapshot.attester, test_env.attester);
}
