//! Shared types used across the oracle-watch service.
//!
//! Defines the core data types for trade records, aggregated snapshots,
//! and signed payload structures that flow between modules.

use serde::Deserialize;

/// A single SDEX trade record from Stellar Horizon.
///
/// Mirrors the JSON shape of Horizon's `/trades` endpoint response. Field
/// types preserve the wire format exactly — amounts are stringified
/// decimals (Stellar's standard representation), and `price_r` is a
/// rational fraction (numerator/denominator) rather than a float.
///
/// # Spec reference
///
/// See spec Bölüm 5 — oracle-watch İşlev A. The `source_account` field
/// supports the unique-trade-count rule: distinct `source_account`s in
/// the 1-hour window count as separate trades; the same `source_account`
/// across multiple ledgers in the window counts as one (anti-wash
/// trading minimal defense).
///
/// # Field provenance
///
/// Horizon emits trades from the perspective of either the base or
/// counter side. `source_account` is taken from `base_account` (the
/// account whose offer was active and matched). For SDEX-style trades
/// this aligns with "the trader who initiated the matching offer".
#[derive(Debug, Clone, Deserialize)]
pub struct TradeRecord {
    /// Trade identifier (Horizon's `id` field). Used for deduplication
    /// across paginated polling.
    pub id: String,

    /// Ledger close time as ISO-8601 string (Horizon emits this format).
    /// Parsed to `chrono::DateTime` or Unix timestamp at the consumer side.
    pub ledger_close_time: String,

    /// Base asset amount as stringified decimal (Stellar standard).
    /// Example: `"100.0000000"` for 100 units of base asset.
    pub base_amount: String,

    /// Counter asset amount as stringified decimal.
    pub counter_amount: String,

    /// Trade price as rational `n/d` fraction (avoids float imprecision).
    pub price_r: PriceRatio,

    /// Account that initiated the matching trade (used by aggregator's
    /// unique-trade-count rule). Horizon emits this as `base_account`.
    #[serde(rename = "base_account")]
    pub source_account: String,
}

/// Trade price as a rational `n/d` fraction.
///
/// Horizon's wire format for `price_r`. Aggregator may convert to f64
/// for USD valuation, but the rational form is preserved here for any
/// future precision-sensitive consumer.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PriceRatio {
    pub n: i64,
    pub d: i64,
}

// TODO Phase 6.3: AggregatedSnapshot struct (volume_30m_usd_i128, unique_trades_1h, computed_at)
// TODO Phase 6.4: LiquiditySnapshotPayload struct (signing input)
