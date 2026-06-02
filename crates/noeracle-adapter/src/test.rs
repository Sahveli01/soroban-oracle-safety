//! Unit tests for `noeracle-adapter`.
//!
//! A minimal in-test Noeracle mock (`get_price_pers` keyed by `BytesN<8>` tag)
//! stands in for the real testnet contract so the adapter's bridging logic —
//! tag mapping, 7→14 rescale, ring-buffer synthesis — is exercised in isolation.

use super::*;
use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, BytesN, Env, Symbol};

#[contract]
pub struct NoeracleMock;

#[contracttype]
enum MockKey {
    Price(BytesN<8>),
}

#[contractimpl]
impl NoeracleMock {
    pub fn seed(env: Env, asset: BytesN<8>, price: i128, round_id: u64, timestamp: u64) {
        env.storage().persistent().set(
            &MockKey::Price(asset),
            &PriceEntry {
                price,
                round_id,
                timestamp,
            },
        );
    }

    pub fn get_price_pers(env: Env, asset: BytesN<8>) -> Option<PriceEntry> {
        env.storage().persistent().get(&MockKey::Price(asset))
    }
}

fn setup<'a>() -> (Env, NoeracleAdapterClient<'a>, NoeracleMockClient<'a>) {
    let env = Env::default();
    let noeracle = env.register(NoeracleMock, ());
    let adapter = env.register(NoeracleAdapter, ());
    let aclient = NoeracleAdapterClient::new(&env, &adapter);
    let nclient = NoeracleMockClient::new(&env, &noeracle);
    aclient.init(&noeracle);
    (env, aclient, nclient)
}

fn other(env: &Env, s: &str) -> Asset {
    Asset::Other(Symbol::new(env, s))
}

#[test]
fn test_tag_for_known_symbols() {
    let (env, a, _n) = setup();
    // Verified live against the testnet Noeracle contract.
    assert_eq!(
        a.tag_for(&other(&env, "BTC")),
        Some(BytesN::from_array(&env, b"BTCUSD\0\0"))
    );
    assert_eq!(
        a.tag_for(&other(&env, "XLM")),
        Some(BytesN::from_array(&env, b"XLMUSD\0\0"))
    );
    // USDC is the tricky one: 4-char base → "USDCUSD" (7 chars) + one NUL pad.
    assert_eq!(
        a.tag_for(&other(&env, "USDC")),
        Some(BytesN::from_array(&env, b"USDCUSD\0"))
    );
    assert_eq!(
        a.tag_for(&other(&env, "ETH")),
        Some(BytesN::from_array(&env, b"ETHUSD\0\0"))
    );
}

#[test]
fn test_tag_for_unknown_and_stellar_is_none() {
    let (env, a, _n) = setup();
    assert_eq!(a.tag_for(&other(&env, "DOGE")), None);
    assert_eq!(a.tag_for(&Asset::Stellar(Address::generate(&env))), None);
}

#[test]
fn test_lastprice_rescales_7_to_14() {
    let (env, a, n) = setup();
    // BTC at $69,535.21 in Noeracle's 1e7 scale.
    let tag = BytesN::from_array(&env, b"BTCUSD\0\0");
    n.seed(&tag, &695_352_100_000, &3_560_802_279, &1_780_401_139);

    let pd = a.lastprice(&other(&env, "BTC")).unwrap();
    // 1e7 → 1e14: ×10^7.
    assert_eq!(pd.price, 695_352_100_000 * 10_000_000);
    assert_eq!(pd.timestamp, 1_780_401_139);
}

#[test]
fn test_lastprice_none_when_noeracle_empty() {
    let (env, a, _n) = setup();
    assert_eq!(a.lastprice(&other(&env, "BTC")), None);
}

#[test]
fn test_lastprice_none_for_unmapped_asset() {
    let (env, a, _n) = setup();
    assert_eq!(a.lastprice(&other(&env, "DOGE")), None);
}

#[test]
fn test_decimals_and_resolution() {
    let (_env, a, _n) = setup();
    assert_eq!(a.decimals(), 14);
    assert_eq!(a.resolution(), 300);
}

#[test]
fn test_prices_none_when_buffer_under_records() {
    let (env, a, n) = setup();
    let tag = BytesN::from_array(&env, b"BTCUSD\0\0");
    n.seed(&tag, &695_352_100_000, &1, &5000);
    // prices() refreshes (pushes 1), buffer len 1 < 2 → None (→ StaleData upstream).
    assert_eq!(a.prices(&other(&env, "BTC"), &2), None);
}

#[test]
fn test_prices_returns_newest_first_when_seeded() {
    let (env, a, _n) = setup();
    // Seed two distinct timestamps directly (14-dp scale) via the keeper helper.
    let btc = other(&env, "BTC");
    a.record_price(&btc, &100, &4000);
    a.record_price(&btc, &200, &5000);
    // Noeracle empty → refresh adds nothing; buffer = [ts5000, ts4000].
    let v = a.prices(&btc, &2).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v.get(0).unwrap().timestamp, 5000); // newest first
    assert_eq!(v.get(0).unwrap().price, 200);
    assert_eq!(v.get(1).unwrap().timestamp, 4000);
}

#[test]
fn test_prices_refreshes_from_noeracle() {
    let (env, a, n) = setup();
    let btc = other(&env, "BTC");
    // One older entry already in the buffer (14-dp scale).
    a.record_price(&btc, &690_000_000_000_000_000, &4000);
    // Noeracle holds a newer price (1e7 scale).
    let tag = BytesN::from_array(&env, b"BTCUSD\0\0");
    n.seed(&tag, &695_352_100_000, &2, &5000);

    let v = a.prices(&btc, &2).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v.get(0).unwrap().timestamp, 5000); // refreshed from Noeracle
    assert_eq!(v.get(0).unwrap().price, 695_352_100_000 * 10_000_000);
    assert_eq!(v.get(1).unwrap().timestamp, 4000);
}

#[test]
fn test_ring_buffer_dedup_by_timestamp() {
    let (env, a, _n) = setup();
    let btc = other(&env, "BTC");
    a.record_price(&btc, &100, &5000);
    a.record_price(&btc, &999, &5000); // same ts → ignored
                                       // Only one distinct ts → prices(.,2) is None.
    assert_eq!(a.prices(&btc, &2), None);
}

#[test]
fn test_ring_buffer_caps_at_max() {
    let (env, a, _n) = setup();
    let btc = other(&env, "BTC");
    for i in 1..=12u64 {
        a.record_price(&btc, &(i as i128 * 100), &(i * 1000));
    }
    // Capacity is 10: a request for 10 succeeds, 11 does not.
    assert_eq!(a.prices(&btc, &10).map(|v| v.len()), Some(10));
    assert_eq!(a.prices(&btc, &11), None);
    // Newest retained, oldest two (ts 1000, 2000) evicted.
    let v = a.prices(&btc, &10).unwrap();
    assert_eq!(v.get(0).unwrap().timestamp, 12000);
    assert_eq!(v.get(9).unwrap().timestamp, 3000);
}
