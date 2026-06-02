#![no_std]
//! # noeracle-adapter
//!
//! A SEP-40 / Reflector-shaped **adapter contract** that wraps the
//! [Noeracle](https://noeracle.org) pull oracle so that `safe-oracle`'s
//! `ReflectorClient` can validate Noeracle prices unchanged.
//!
//! `safe-oracle` (v0.4.0) consumes four methods on its oracle:
//!   - `lastprice(asset: Asset) -> Option<PriceData>`
//!   - `prices(asset: Asset, records: u32) -> Option<Vec<PriceData>>`
//!   - `decimals() -> u32`   (must equal `REFLECTOR_DECIMALS_EXPECTED` = 14)
//!   - `resolution() -> u32`
//!
//! Noeracle exposes only:
//!   - `get_price_pers(asset: BytesN<8>) -> Option<PriceEntry { price, round_id, timestamp }>`
//!
//! This adapter bridges the four gaps between the two:
//!   1. **Method name** — Noeracle has no `prices`/`lastprice`; we synthesize them.
//!   2. **Storage class** — we read `get_price_pers` (persistent, populated),
//!      not `get_price` (temporary, frequently empty on testnet).
//!   3. **Asset identity** — `Asset::Other(Symbol)` is mapped to Noeracle's
//!      UTF-8 `BytesN<8>` pair tag (e.g. `BTC` → `b"BTCUSD\0\0"`).
//!   4. **Decimals** — Noeracle scales by 1e7 (7 dp); we rescale by 1e7 to the
//!      1e14 (14 dp) convention `safe-oracle` enforces, and report `decimals = 14`.
//!
//! ## ADAPTER-SYNTHESIZED HISTORY (honesty note)
//!
//! Noeracle stores only the **latest** price per asset on-chain — a single
//! slot, no native history, no ring buffer, no round-indexed map. To enable
//! `safe-oracle`'s **deviation guardrail** (which compares two consecutive
//! prices via `prices(asset, 2)`), this adapter accumulates an on-chain
//! ring buffer across writes. **This history is adapter-layer synthesized,
//! NOT Noeracle's native oracle ticks.** Its cadence is the cadence at which
//! the adapter is written to (e.g. by a keeper), so the "previous" price is
//! adapter-call-spaced, not resolution-spaced like Reflector's native history.
//! Read-only `simulateTransaction` invocations do **not** persist the buffer,
//! so deviation only becomes available once real (signed) writes have seeded
//! at least `records` distinct timestamps.

use safe_oracle::{Asset, PriceData};
use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, vec, Address, BytesN, Env, Symbol, Vec,
};

/// Ring-buffer capacity per asset (newest-first). `safe-oracle` reads
/// `records = 2`, but a slightly deeper buffer lets a keeper retain a few
/// recent ticks without extra storage churn.
const RING_BUFFER_MAX: u32 = 10;

/// `safe-oracle`'s hard-coded expectation (`REFLECTOR_DECIMALS_EXPECTED = 14`).
/// A mismatch raises `OracleSafetyViolation::DecimalsMismatch`.
const ADAPTER_DECIMALS: u32 = 14;

/// Reflector mainnet convention (mock-reflector also returns 300). Declared
/// for interface parity; `safe-oracle` does not currently call `resolution`.
const ADAPTER_RESOLUTION: u32 = 300;

/// Noeracle stores prices scaled by 1e7 (7 dp); `safe-oracle` expects 1e14
/// (14 dp). Rescale factor = 10^(14 - 7).
const RESCALE_7_TO_14: i128 = 10_000_000;

/// Mirror of Noeracle's on-chain `PriceEntry` (field names must match for
/// cross-contract map decoding; order is irrelevant — Soroban keys by name).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceEntry {
    pub price: i128,
    pub round_id: u64,
    pub timestamp: u64,
}

/// Cross-contract client for the wrapped Noeracle contract — only the single
/// read the adapter needs.
#[contractclient(name = "NoeracleClient")]
pub trait Noeracle {
    fn get_price_pers(env: Env, asset: BytesN<8>) -> Option<PriceEntry>;
}

#[contracttype]
enum DataKey {
    /// Wrapped Noeracle contract address (set in `init`).
    Noeracle,
    /// Per-asset synthesized price ring buffer (newest-first, ≤ RING_BUFFER_MAX).
    History(Asset),
}

#[contract]
pub struct NoeracleAdapter;

#[contractimpl]
impl NoeracleAdapter {
    /// One-time setup: which Noeracle contract this adapter wraps.
    pub fn init(env: Env, noeracle: Address) {
        env.storage().instance().set(&DataKey::Noeracle, &noeracle);
    }

    /// SEP-40 method 1 — current price.
    ///
    /// Reads Noeracle's persistent slot, rescales 7→14 dp, drops `round_id`
    /// (no place in SEP-40 `PriceData`), and records the result in the ring
    /// buffer (deduplicated by timestamp, since `lastprice` may be called
    /// multiple times within one Noeracle round).
    pub fn lastprice(env: Env, asset: Asset) -> Option<PriceData> {
        let noeracle_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::Noeracle)
            .expect("adapter not initialized — call init() first");

        let tag = asset_to_tag(&env, &asset)?;
        let client = NoeracleClient::new(&env, &noeracle_addr);
        let entry = client.get_price_pers(&tag)?;

        // Rescale 1e7 → 1e14. Treat overflow as "no price" (fail-safe).
        let price = entry.price.checked_mul(RESCALE_7_TO_14)?;
        let pd = PriceData {
            price,
            timestamp: entry.timestamp,
        };

        push_history(&env, &asset, &pd);
        Some(pd)
    }

    /// SEP-40 method 2 — last `records` prices, newest-first.
    ///
    /// Noeracle does not provide history; this returns the adapter's
    /// synthesized ring buffer. Refreshes the buffer with the current price
    /// first, then returns the newest `records` entries — or `None` when fewer
    /// than `records` distinct timestamps have accumulated, which `safe-oracle`
    /// maps fail-safe to `StaleData`.
    pub fn prices(env: Env, asset: Asset, records: u32) -> Option<Vec<PriceData>> {
        // Refresh: pull the current Noeracle price into the buffer.
        let _ = Self::lastprice(env.clone(), asset.clone());

        let hist = load_history(&env, &asset);
        if hist.len() < records {
            return None;
        }
        let mut out: Vec<PriceData> = vec![&env];
        for i in 0..records {
            out.push_back(hist.get(i).unwrap());
        }
        Some(out)
    }

    /// SEP-40 method 3 — precision. Must report 14 (post-rescale).
    pub fn decimals(_env: Env) -> u32 {
        ADAPTER_DECIMALS
    }

    /// SEP-40 method 4 — round cadence (interface parity; unused by safe-oracle).
    pub fn resolution(_env: Env) -> u32 {
        ADAPTER_RESOLUTION
    }

    /// Test/keeper helper — seed a price into the ring buffer directly,
    /// already in 14-dp scale. In production the buffer is filled by
    /// `lastprice`/`prices` calls; this lets a keeper or test pre-seed history.
    pub fn record_price(env: Env, asset: Asset, price: i128, timestamp: u64) {
        push_history(&env, &asset, &PriceData { price, timestamp });
    }

    /// Test helper — expose the `Asset → BytesN<8>` mapping for verification.
    pub fn tag_for(env: Env, asset: Asset) -> Option<BytesN<8>> {
        asset_to_tag(&env, &asset)
    }
}

/// `Asset::Other(Symbol)` → Noeracle UTF-8 pair tag (`BytesN<8>`).
///
/// Noeracle keys assets by the UTF-8 bytes of the `BASE`+`USD` pair, right
/// zero-padded to 8 bytes (e.g. `BTC` → `b"BTCUSD\0\0"` = `0x4254435553440000`,
/// `USDC` → `b"USDCUSD\0"` = `0x5553444355534400`). Verified live against the
/// testnet contract. `Asset::Stellar(_)` and unknown symbols return `None`.
fn asset_to_tag(env: &Env, asset: &Asset) -> Option<BytesN<8>> {
    let sym = match asset {
        Asset::Other(s) => s.clone(),
        Asset::Stellar(_) => return None,
    };

    let bytes: [u8; 8] = if sym == Symbol::new(env, "BTC") {
        *b"BTCUSD\0\0"
    } else if sym == Symbol::new(env, "ETH") {
        *b"ETHUSD\0\0"
    } else if sym == Symbol::new(env, "XLM") {
        *b"XLMUSD\0\0"
    } else if sym == Symbol::new(env, "USDC") {
        *b"USDCUSD\0"
    } else if sym == Symbol::new(env, "SOL") {
        *b"SOLUSD\0\0"
    } else if sym == Symbol::new(env, "XRP") {
        *b"XRPUSD\0\0"
    } else if sym == Symbol::new(env, "ADA") {
        *b"ADAUSD\0\0"
    } else {
        return None;
    };

    Some(BytesN::from_array(env, &bytes))
}

fn load_history(env: &Env, asset: &Asset) -> Vec<PriceData> {
    env.storage()
        .persistent()
        .get(&DataKey::History(asset.clone()))
        .unwrap_or_else(|| vec![env])
}

/// Push newest-first with timestamp dedup; cap at `RING_BUFFER_MAX`.
fn push_history(env: &Env, asset: &Asset, price: &PriceData) {
    let mut hist = load_history(env, asset);
    if hist.iter().any(|p| p.timestamp == price.timestamp) {
        return;
    }
    hist.push_front(price.clone());
    while hist.len() > RING_BUFFER_MAX {
        hist.pop_back();
    }
    env.storage()
        .persistent()
        .set(&DataKey::History(asset.clone()), &hist);
}

#[cfg(test)]
mod test;
