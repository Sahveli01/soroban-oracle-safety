//! Stellar Horizon API HTTP client for SDEX trade history.
//!
//! Wraps `reqwest` with typed responses for `/trades` endpoint queries.
//! Implemented in Phase 6.2.

// TODO Phase 6.2: HorizonClient struct (reqwest::Client + base_url)
// TODO Phase 6.2: get_recent_trades() — query SDEX /trades, parse JSON to Vec<TradeRecord>
// TODO Phase 6.2: HorizonError enum (network, parse, rate-limit)
