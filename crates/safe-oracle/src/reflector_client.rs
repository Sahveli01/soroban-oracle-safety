use crate::{Asset, PriceData};
use soroban_sdk::{contractclient, Env, Vec};

/// Reflector oracle interface (SEP-40 compliant subset that `safe_oracle` consumes).
///
/// Defines the public surface of the Reflector contract. The
/// `#[contractclient]` macro auto-generates a `ReflectorClient` struct from
/// this trait — clients are instantiated as `ReflectorClient::new(env, address)`
/// and used as `client.lastprice(&asset)`.
///
/// In production this binds to the real Reflector mainnet address; in tests it
/// binds to the `MockReflector` address. The integration code is identical
/// across both — only the runtime address changes.
///
/// Method signatures must match `mock_reflector` (and the real Reflector
/// contract) exactly:
/// - `mock_reflector::lastprice(env: Env, asset: Asset) -> Option<PriceData>`
/// - `mock_reflector::lastprices(env: Env, asset: Asset, records: u32) -> Option<Vec<PriceData>>`
/// - `mock_reflector::decimals(env: Env) -> u32`
/// - `mock_reflector::resolution(env: Env) -> u32`
// The trait exists solely so `#[contractclient]` can synthesize the client
// struct; nothing calls it directly (the macro re-emits the signatures).
#[allow(dead_code)]
#[contractclient(name = "ReflectorClient")]
pub trait Reflector {
    fn lastprice(env: Env, asset: Asset) -> Option<PriceData>;
    fn lastprices(env: Env, asset: Asset, records: u32) -> Option<Vec<PriceData>>;
    fn decimals(env: Env) -> u32;
    fn resolution(env: Env) -> u32;
}
