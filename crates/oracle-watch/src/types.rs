//! Shared types used across the oracle-watch service.
//!
//! Defines the core data types for trade records, aggregated snapshots,
//! and signed payload structures that flow between modules.
//!
//! Implemented progressively across Phase 6.2-6.4:
//! - `TradeRecord` (Phase 6.2 — horizon_client output)
//! - `AggregatedSnapshot` (Phase 6.3 — aggregator output)
//! - `LiquiditySnapshotPayload` (Phase 6.4 — signer input)

// TODO Phase 6.2: TradeRecord struct (base/counter amounts, price_r, ledger_close_time, source_account)
// TODO Phase 6.3: AggregatedSnapshot struct (volume_30m_usd, unique_trades_1h, computed_at)
// TODO Phase 6.4: LiquiditySnapshotPayload struct (signing input)
