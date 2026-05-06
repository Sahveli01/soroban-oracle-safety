//! Aggregates raw trade records into volume and trade-count snapshots.
//!
//! Implements the spec's "Trade Sayım Tanımı":
//! - 30-minute USD volume sum
//! - 1-hour unique trade count (distinct source_account, $10 minimum)
//!
//! Implemented in Phase 6.3.

// TODO Phase 6.3: aggregate_trades(trades_30m, trades_1h, usdc_price) -> AggregatedSnapshot
// TODO Phase 6.3: USD value computation per trade
// TODO Phase 6.3: unique source_account counting with $10 spam filter
