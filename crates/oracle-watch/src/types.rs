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
// Fields are populated by serde from Horizon JSON; the aggregator only
// consumes a subset directly, but downstream Phase 8 work (dedup by id,
// time-window filtering by ledger_close_time) needs the rest. The
// dead_code lint cannot see serde's field reads.
#[allow(dead_code)]
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
#[allow(dead_code)] // serde-populated; consumed by Phase 8 precision-sensitive paths.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PriceRatio {
    pub n: i64,
    pub d: i64,
}

/// Aggregated trade statistics for a single asset over a time window.
///
/// Output of the [`crate::aggregator::aggregate_trades`] function. Mirrors
/// the on-chain `LiquiditySnapshot` shape (volume + trade count + timestamp)
/// but uses `i128` stroops for the volume field per Stellar 7-decimal
/// convention.
///
/// # Spec reference
///
/// See spec Bölüm 5 — oracle-watch İşlev A.
///
/// - `volume_30m_usd_i128`: 30-minute USD volume sum × 10^7 (stroop unit)
/// - `unique_trades_1h`: distinct `source_account` count over 1-hour
///   window, with $10 minimum-trade-value spam filter
/// - `computed_at`: Unix timestamp when this snapshot was computed
///   (oracle-watch wall clock, not ledger time)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedSnapshot {
    /// Asset code (e.g., "USDC", "XLM"). Used for downstream
    /// `LiquidityRegistry::write_snapshot` payload identification.
    pub asset_code: String,

    /// Asset issuer (Stellar address or `"native"` for XLM).
    pub asset_issuer: String,

    /// 30-minute USD volume × 10^7 (stroop convention).
    ///
    /// `i128` matches the on-chain `LiquiditySnapshot.volume_30m_usd` type
    /// for direct write-through without conversion.
    pub volume_30m_usd_i128: i128,

    /// 1-hour unique trade count per spec's Trade Sayım Tanımı.
    ///
    /// Counts distinct `source_account` values that contributed at least
    /// one trade with USD value ≥ `MIN_TRADE_USD_VALUE` ($10) within the
    /// 1-hour window. Multiple trades from the same `source_account` count
    /// once.
    pub unique_trades_1h: u32,

    /// Unix timestamp (seconds) when this snapshot was computed.
    /// Set by `aggregate_trades` from `std::time::SystemTime::now()`.
    pub computed_at: u64,
}

/// Payload that gets signed by oracle-watch and (in spec) verified
/// on-chain. The byte serialization is deterministic — same struct →
/// same bytes → same signature, regardless of platform endianness or
/// Rust version.
///
/// # On-chain status (Phase 6 design note)
///
/// The current `LiquidityRegistry` contract does **not** verify this
/// signature on-chain. Per Phase 3.3, write_snapshot uses Stellar's
/// `require_auth_for_args` to authenticate the attester via the
/// Stellar keypair signing the submit transaction. The ed25519
/// signature here is **redundant** with require_auth in the present
/// design.
///
/// Why we still produce it:
/// - Spec compliance — spec Bölüm 5 calls for ed25519 attestation
/// - Forward compat — future SDK or contract revisions may verify
///   it directly without round-tripping through Stellar transaction auth
/// - Off-chain audit — alternative auditors can independently verify
///   the attester signed each snapshot
/// - Defense-in-depth — if Stellar transaction auth is ever bypassed
///   (extremely unlikely), the ed25519 signature is a second factor
///
/// # Serialization format
///
/// Concatenated big-endian bytes in fixed field order:
///
/// ```text
/// asset_code_len (u8) || asset_code (utf8) ||
/// asset_issuer_len (u8) || asset_issuer (utf8) ||
/// volume_30m_usd_i128 (16 bytes, big-endian, two's complement) ||
/// unique_trades_1h (4 bytes, big-endian) ||
/// timestamp (8 bytes, big-endian)
/// ```
///
/// The length prefixes prevent ambiguity when concatenating string fields.
/// Fixed integer widths and big-endian ordering ensure cross-platform
/// determinism.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiquiditySnapshotPayload {
    pub asset_code: String,
    pub asset_issuer: String,
    pub volume_30m_usd_i128: i128,
    pub unique_trades_1h: u32,
    pub timestamp: u64,
}

impl LiquiditySnapshotPayload {
    /// Builds the canonical byte representation for signing.
    ///
    /// Length-prefixed UTF-8 strings, big-endian fixed-width integers.
    /// Asset string lengths are bounded at u8::MAX (255 bytes) — typical
    /// asset codes are 1-12 bytes, issuers are 56-byte Stellar addresses,
    /// well within the cap.
    ///
    /// Returns `None` if either string exceeds 255 bytes (defensive;
    /// production assets never trigger this).
    pub fn to_signing_bytes(&self) -> Option<Vec<u8>> {
        let code_bytes = self.asset_code.as_bytes();
        let issuer_bytes = self.asset_issuer.as_bytes();

        if code_bytes.len() > u8::MAX as usize || issuer_bytes.len() > u8::MAX as usize {
            return None;
        }

        let mut buf =
            Vec::with_capacity(1 + code_bytes.len() + 1 + issuer_bytes.len() + 16 + 4 + 8);

        buf.push(code_bytes.len() as u8);
        buf.extend_from_slice(code_bytes);

        buf.push(issuer_bytes.len() as u8);
        buf.extend_from_slice(issuer_bytes);

        buf.extend_from_slice(&self.volume_30m_usd_i128.to_be_bytes());
        buf.extend_from_slice(&self.unique_trades_1h.to_be_bytes());
        buf.extend_from_slice(&self.timestamp.to_be_bytes());

        Some(buf)
    }
}

impl From<&AggregatedSnapshot> for LiquiditySnapshotPayload {
    /// Converts an `AggregatedSnapshot` into a signable payload.
    ///
    /// The `computed_at` field of `AggregatedSnapshot` becomes `timestamp`
    /// in the payload. Both represent oracle-watch wall-clock time, not
    /// ledger time.
    fn from(snap: &AggregatedSnapshot) -> Self {
        Self {
            asset_code: snap.asset_code.clone(),
            asset_issuer: snap.asset_issuer.clone(),
            volume_30m_usd_i128: snap.volume_30m_usd_i128,
            unique_trades_1h: snap.unique_trades_1h,
            timestamp: snap.computed_at,
        }
    }
}
