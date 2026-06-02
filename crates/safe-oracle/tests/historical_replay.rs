//! Historical Exploit Replay Harness
//!
//! Empirically validates `safe-oracle`'s **default** thresholds against real
//! DeFi exploits. Each test loads an attack's price series from a cited JSON
//! evidence file in `historical_replay/data/` and replays it through the live
//! `safe_oracle::lastprice()` path (via `test_utils::TestEnv`), asserting
//! whether the default configuration would have rejected the malicious feed.
//!
//! This converts the project's "sezgisel default" (intuitive-default) critique
//! into an empirical claim: the defaults are not hand-waved — they reject
//! historically-observed oracle manipulation.
//!
//! # Honesty rules baked into this harness
//!
//! 1. Every price comes from a public source (see each JSON `sources` array).
//! 2. Attacks that the defaults do NOT catch are not hidden — Euler is
//!    included as an explicit out-of-scope negative control (it was not an
//!    oracle attack), and the sub-threshold YieldBlox variant documents that
//!    Layer 1 alone is insufficient there (Layer 2 is required).
//! 3. Non-Stellar attacks are marked `adapted: true` and replay real foreign-
//!    chain prices through Soroban semantics.
//!
//! Run with: `cargo test --test historical_replay`

// This file is the integration-test crate root, so `mod foo;` would resolve
// to `tests/foo.rs`. The replay modules live in the `historical_replay/`
// subdirectory to keep them grouped with their JSON evidence files, so each
// is pointed at explicitly with `#[path]`.
#[path = "historical_replay/data_loader.rs"]
mod data_loader;

#[path = "historical_replay/bonqdao_feb_2023.rs"]
mod bonqdao_feb_2023;
#[path = "historical_replay/euler_mar_2023.rs"]
mod euler_mar_2023;
#[path = "historical_replay/mango_oct_2022.rs"]
mod mango_oct_2022;
#[path = "historical_replay/yieldblox_feb_2026.rs"]
mod yieldblox_feb_2026;
