use crate::{Asset, PriceData};
use soroban_sdk::{contractclient, Env};

/// Reflector oracle interface (SEP-40 compliant subset that `safe_oracle` consumes).
///
/// Bu trait Reflector kontratının public fonksiyonlarını tanımlar.
/// `#[contractclient]` macro otomatik olarak `ReflectorClient` struct'ı generate eder
/// (`ReflectorClient::new(env, address)` ile instantiate edilir, sonra
/// `client.lastprice(&asset)` gibi çağırılır).
///
/// Production'da gerçek Reflector adresine, test'te mock-reflector adresine bağlanır —
/// interface aynı olduğu için kod değişmez.
///
/// Trait method imzaları mock-reflector ile birebir uyumlu:
/// - `mock_reflector::lastprice(env: Env, asset: Asset) -> Option<PriceData>`
/// - `mock_reflector::decimals(env: Env) -> u32`
/// - `mock_reflector::resolution(env: Env) -> u32`
// Trait yalnızca `#[contractclient]` macro'sunun client struct'ı generate
// etmesi için tanımlandı; doğrudan çağrı yapılmıyor (macro imzaları kopyalar).
#[allow(dead_code)]
#[contractclient(name = "ReflectorClient")]
pub trait Reflector {
    fn lastprice(env: Env, asset: Asset) -> Option<PriceData>;
    fn decimals(env: Env) -> u32;
    fn resolution(env: Env) -> u32;
}
