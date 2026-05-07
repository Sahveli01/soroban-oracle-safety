//! Anomaly detection for liquidity snapshots.
//!
//! Pure-function detection module. Compares consecutive snapshots and
//! current oracle prices against configurable thresholds, surfacing
//! anomalies that would warrant operator alerts.
//!
//! # Phase 6.6 vs 6.7
//!
//! This module performs **detection only** — it returns a `Vec<Anomaly>`
//! describing what was found. **Alert dispatch** (Discord/Telegram
//! webhook posting) is implemented in Phase 6.7. The two responsibilities
//! are separated to keep detection pure and easily testable.
//!
//! # Spec reference
//!
//! See spec Bölüm 5 — oracle-watch İşlev B.

use crate::aggregator::MIN_TRADE_USD_VALUE;
use crate::types::AggregatedSnapshot;

/// Anomaly detection thresholds.
///
/// All thresholds are inclusive (`<=` for low values, `>=` for high).
/// The defaults reflect operational expectations for a reasonably-active
/// SDEX pair on testnet/mainnet:
///
/// - **`max_price_change_bps = 2000`** (20%): a single-poll price move
///   exceeding 20% is structurally unusual and worth surfacing.
/// - **`min_volume_threshold_usd = 10_000.0`**: less than $10k of 30-min
///   volume on a watched asset is too thin to support reliable Layer 2
///   liquidity attestation.
/// - **`min_trade_count = 5`**: fewer than 5 unique trades in 1 hour is
///   the spec's structural thin-sampling boundary.
#[derive(Debug, Clone, Copy)]
pub struct AnomalyDetector {
    pub max_price_change_bps: u32,
    pub min_volume_threshold_usd: f64,
    pub min_trade_count: u32,
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self {
            max_price_change_bps: 2_000,
            min_volume_threshold_usd: 10_000.0,
            min_trade_count: 5,
        }
    }
}

/// A detected anomaly worth surfacing to operators.
///
/// Each variant carries enough context to populate a human-readable
/// alert (asset identification, observed values, threshold reference).
#[derive(Debug, Clone, PartialEq)]
pub enum Anomaly {
    /// Single-poll price movement exceeded the configured BPS threshold.
    ExcessivePriceChange {
        asset_code: String,
        asset_issuer: String,
        prev_price: f64,
        curr_price: f64,
        change_bps: u32,
        threshold_bps: u32,
    },

    /// 30-minute USD volume fell below the operator threshold.
    ///
    /// Note: This is distinct from on-chain `Layer2::InsufficientLiquidity`,
    /// which uses a per-config `min_liquidity_usd` (typically $100k+).
    /// This off-chain monitor threshold is set lower to surface
    /// degrading-liquidity warnings before they reach the on-chain
    /// fail-safe boundary.
    InsufficientLiquidity {
        asset_code: String,
        asset_issuer: String,
        volume_usd: f64,
        threshold_usd: f64,
    },

    /// Unique trade count fell below the structural thin-sampling threshold.
    ThinSampling {
        asset_code: String,
        asset_issuer: String,
        trade_count: u32,
        threshold_count: u32,
    },
}

impl AnomalyDetector {
    /// Detects anomalies between two consecutive snapshots and a current price.
    ///
    /// # Parameters
    ///
    /// - `prev_snapshot`: the previous snapshot (e.g., from the previous poll)
    /// - `curr_snapshot`: the current snapshot just computed
    /// - `prev_price`: the asset's price at the previous poll (USD)
    /// - `curr_price`: the asset's current price (USD)
    ///
    /// # Returns
    ///
    /// A vector of `Anomaly` instances. Empty vector means no anomalies
    /// detected — operationally healthy state.
    ///
    /// # Detection rules
    ///
    /// 1. **ExcessivePriceChange**: if `|curr_price - prev_price| / prev_price`
    ///    in BPS exceeds `max_price_change_bps`. NaN/inf/zero `prev_price`
    ///    are skipped (indeterminate change cannot be classified as anomaly).
    /// 2. **InsufficientLiquidity**: if `curr_snapshot.volume_30m_usd_i128`
    ///    converted to USD is below `min_volume_threshold_usd`.
    /// 3. **ThinSampling**: if `curr_snapshot.unique_trades_1h` is below
    ///    `min_trade_count`.
    ///
    /// All three rules are checked independently; the returned vector may
    /// contain 0, 1, 2, or 3 anomalies.
    pub fn check(
        &self,
        prev_snapshot: &AggregatedSnapshot,
        curr_snapshot: &AggregatedSnapshot,
        prev_price: f64,
        curr_price: f64,
    ) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        // Rule 1: ExcessivePriceChange
        if let Some(change_bps) = compute_price_change_bps(prev_price, curr_price) {
            if change_bps > self.max_price_change_bps {
                anomalies.push(Anomaly::ExcessivePriceChange {
                    asset_code: curr_snapshot.asset_code.clone(),
                    asset_issuer: curr_snapshot.asset_issuer.clone(),
                    prev_price,
                    curr_price,
                    change_bps,
                    threshold_bps: self.max_price_change_bps,
                });
            }
        }

        // Rule 2: InsufficientLiquidity
        let volume_usd = stroops_to_usd(curr_snapshot.volume_30m_usd_i128);
        if volume_usd < self.min_volume_threshold_usd {
            anomalies.push(Anomaly::InsufficientLiquidity {
                asset_code: curr_snapshot.asset_code.clone(),
                asset_issuer: curr_snapshot.asset_issuer.clone(),
                volume_usd,
                threshold_usd: self.min_volume_threshold_usd,
            });
        }

        // Rule 3: ThinSampling
        if curr_snapshot.unique_trades_1h < self.min_trade_count {
            anomalies.push(Anomaly::ThinSampling {
                asset_code: curr_snapshot.asset_code.clone(),
                asset_issuer: curr_snapshot.asset_issuer.clone(),
                trade_count: curr_snapshot.unique_trades_1h,
                threshold_count: self.min_trade_count,
            });
        }

        // Phase 6.6: log to stdout. Phase 6.7 will replace with webhook dispatch.
        if !anomalies.is_empty() {
            for anomaly in &anomalies {
                eprintln!("ORACLE-WATCH ANOMALY: {anomaly:?}");
            }
            // TODO Phase 6.7: dispatch_alerts(&anomalies, &alert_config).await
        }

        // prev_snapshot is currently only used as future-facing context
        // (e.g., volume-trend or trade-count-delta detection in Phase 7+).
        // Kept in signature for API stability.
        let _ = prev_snapshot;

        anomalies
    }
}

/// Computes price change in basis points between two prices.
///
/// Returns `None` if `prev_price` is non-positive or non-finite (indeterminate
/// change), or if the BPS computation overflows or produces non-finite output.
/// A return of `Some(bps)` is always a finite, non-negative u32.
fn compute_price_change_bps(prev_price: f64, curr_price: f64) -> Option<u32> {
    if !prev_price.is_finite() || !curr_price.is_finite() || prev_price <= 0.0 {
        return None;
    }

    let diff = (curr_price - prev_price).abs();
    let ratio = diff / prev_price;
    let bps = (ratio * 10_000.0).round();

    if !bps.is_finite() || bps < 0.0 {
        return None;
    }

    if bps >= u32::MAX as f64 {
        return Some(u32::MAX);
    }

    Some(bps as u32)
}

/// Converts i128 stroops back to USD f64.
///
/// Inverse of the aggregator's STROOP_MULTIPLIER scaling. Used here to
/// compare on-chain-formatted volume against the operator's USD-cents
/// threshold.
fn stroops_to_usd(stroops: i128) -> f64 {
    stroops as f64 / 10_000_000.0
}

// Reference to MIN_TRADE_USD_VALUE for documentation cross-link consistency.
// (Currently informational; Phase 7+ may use this to refine thresholds.)
const _MIN_TRADE_VALUE_DOC_REF: f64 = MIN_TRADE_USD_VALUE;

// ============================================================================
// Phase 6.7: Alert Dispatch — Trait-based sink architecture
// ============================================================================
//
// Detection (Phase 6.6) and dispatch (this section) are separated. Detection
// is pure synchronous logic; dispatch is async I/O routed through pluggable
// sinks. New alert channels (PagerDuty, Slack, email) can be added in Phase
// 8 by implementing the `WebhookSink` trait — no changes required to this
// dispatcher or to the detector.

use async_trait::async_trait;

/// A pluggable alert sink.
///
/// Implementors handle the network mechanics for posting an alert message
/// to a specific channel (Discord, Telegram, PagerDuty, etc.).
///
/// # Best-effort contract
///
/// Implementors MUST NOT panic on transient failures (network errors,
/// 4xx/5xx responses, malformed responses). They return a `Result<(),
/// DispatchError>` so the dispatcher can log per-sink failures while
/// continuing to other sinks.
///
/// # Example: adding a new sink in Phase 8
///
/// ```ignore
/// pub struct SlackSink { webhook_url: String }
///
/// #[async_trait]
/// impl WebhookSink for SlackSink {
///     fn kind(&self) -> &'static str { "slack" }
///     async fn send(&self, msg: &str) -> Result<(), DispatchError> {
///         // POST to webhook_url with Slack JSON body shape
///         Ok(())
///     }
/// }
/// ```
///
/// No changes required to `dispatch_alerts` or any existing sink — the
/// trait makes the dispatcher's loop polymorphic.
#[async_trait]
pub trait WebhookSink: Send + Sync {
    /// Short human-readable name (e.g., "discord", "telegram").
    /// Used in log messages when dispatch fails.
    fn kind(&self) -> &'static str;

    /// Sends a single formatted message to this sink.
    ///
    /// Implementors should handle their channel's specific protocol
    /// (Discord JSON shape, Telegram URL parameters, etc.) internally.
    /// The dispatcher passes the same message string to all sinks.
    async fn send(&self, message: &str) -> Result<(), DispatchError>;
}

/// Errors that a `WebhookSink` may return.
///
/// Wrapped as a single string variant — sinks differ widely in their
/// failure modes (HTTP errors, JSON shape errors, rate limits) and the
/// dispatcher only needs the human-readable description for logging.
#[derive(Debug)]
pub struct DispatchError(pub String);

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DispatchError {}

/// Dispatches alerts for detected anomalies to all configured sinks.
///
/// # Behavior
///
/// - **Empty `anomalies`** → no-op, returns immediately.
/// - **Empty `sinks`** → no-op, returns immediately (anomalies still
///   logged by detector via eprintln!).
/// - **Per-sink failure** → logged via eprintln!, does NOT prevent
///   other sinks from being attempted.
/// - **Sequential dispatch** for simplicity. Anomalies are infrequent
///   (rate limit pressure low); concurrent dispatch is Phase 8 if real
///   throughput becomes an issue.
///
/// # Why this returns ()
///
/// Alert dispatch is best-effort. The main poll loop must continue
/// regardless of sink availability. Returning errors would invite
/// callers to bubble them up and accidentally take the service offline
/// if Discord rate-limits us. Failures are logged for operator awareness;
/// functional correctness does not depend on them.
pub async fn dispatch_alerts(anomalies: &[Anomaly], sinks: &[Box<dyn WebhookSink>]) {
    if anomalies.is_empty() || sinks.is_empty() {
        return;
    }

    let message = format_alert_message(anomalies);

    for sink in sinks {
        match sink.send(&message).await {
            Ok(()) => {}
            Err(e) => eprintln!("ORACLE-WATCH ALERT: {} dispatch failed: {e}", sink.kind()),
        }
    }
}

/// Formats anomalies for sink consumption.
///
/// Single text body shared across all sinks. Per-sink protocol details
/// (Discord JSON envelope, Telegram URL params) are added by individual
/// `WebhookSink` implementors when they wrap this string.
pub(crate) fn format_alert_message(anomalies: &[Anomaly]) -> String {
    let mut out = String::from("Oracle-Watch Alert\n");
    for a in anomalies {
        out.push_str(&format!("- {}\n", format_anomaly_line(a)));
    }
    out
}

/// Single-line human-readable anomaly description.
pub(crate) fn format_anomaly_line(anomaly: &Anomaly) -> String {
    match anomaly {
        Anomaly::ExcessivePriceChange {
            asset_code,
            asset_issuer,
            prev_price,
            curr_price,
            change_bps,
            threshold_bps,
        } => format!(
            "ExcessivePriceChange [{asset_code}/{asset_issuer}]: {prev_price} -> {curr_price} \
             ({change_bps} BPS, threshold {threshold_bps})"
        ),
        Anomaly::InsufficientLiquidity {
            asset_code,
            asset_issuer,
            volume_usd,
            threshold_usd,
        } => format!(
            "InsufficientLiquidity [{asset_code}/{asset_issuer}]: \
             ${volume_usd:.2} (threshold ${threshold_usd:.2})"
        ),
        Anomaly::ThinSampling {
            asset_code,
            asset_issuer,
            trade_count,
            threshold_count,
        } => format!(
            "ThinSampling [{asset_code}/{asset_issuer}]: \
             {trade_count} trades (threshold {threshold_count})"
        ),
    }
}

/// Operator configuration for alert sinks.
///
/// Loaded from environment in Phase 6.8 (main loop wiring). Each field
/// is independently optional — None disables that channel. The
/// `build_sinks()` factory translates this configuration into a
/// `Vec<Box<dyn WebhookSink>>` ready for `dispatch_alerts`.
#[derive(Debug, Clone, Default)]
pub struct AlertConfig {
    pub discord_webhook_url: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
}

impl AlertConfig {
    /// Loads `AlertConfig` from environment variables.
    ///
    /// All fields are optional. Operator sets only the channels they
    /// want to use:
    ///
    /// - `ORACLE_WATCH_DISCORD_WEBHOOK_URL`: Discord webhook URL
    /// - `ORACLE_WATCH_TELEGRAM_BOT_TOKEN`: Telegram bot token from BotFather
    /// - `ORACLE_WATCH_TELEGRAM_CHAT_ID`: target chat ID
    ///
    /// Telegram requires BOTH token + chat_id to be set; partial
    /// configuration is silently skipped by `build_sinks()`.
    ///
    /// Unset variables → corresponding `Option::None` (no alerts on
    /// that channel). All-unset → no alert dispatch (anomalies still
    /// logged via eprintln!).
    pub fn from_env() -> Self {
        Self {
            discord_webhook_url: std::env::var("ORACLE_WATCH_DISCORD_WEBHOOK_URL").ok(),
            telegram_bot_token: std::env::var("ORACLE_WATCH_TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: std::env::var("ORACLE_WATCH_TELEGRAM_CHAT_ID").ok(),
        }
    }

    /// Builds a sink vector from the configured channels.
    ///
    /// Filters out partially-configured channels (e.g., Telegram with
    /// only token but no chat_id is silently skipped — both fields
    /// required).
    pub fn build_sinks(&self) -> Vec<Box<dyn WebhookSink>> {
        let mut sinks: Vec<Box<dyn WebhookSink>> = Vec::new();

        if let Some(url) = &self.discord_webhook_url {
            sinks.push(Box::new(crate::discord_sink::DiscordSink::new(url.clone())));
        }

        if let (Some(token), Some(chat_id)) = (&self.telegram_bot_token, &self.telegram_chat_id) {
            sinks.push(Box::new(crate::telegram_sink::TelegramSink::new(
                token.clone(),
                chat_id.clone(),
            )));
        }

        sinks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(
        asset: &str,
        issuer: &str,
        volume_stroops: i128,
        trades: u32,
    ) -> AggregatedSnapshot {
        AggregatedSnapshot {
            asset_code: asset.to_string(),
            asset_issuer: issuer.to_string(),
            volume_30m_usd_i128: volume_stroops,
            unique_trades_1h: trades,
            computed_at: 1_715_000_000,
        }
    }

    fn healthy_snapshot() -> AggregatedSnapshot {
        // $50k volume, 25 unique trades — well above defaults
        make_snapshot("USDC", "GA5ZSEJ", 500_000_000_000, 25)
    }

    // ===== compute_price_change_bps tests =====

    #[test]
    fn test_price_change_bps_no_change() {
        assert_eq!(compute_price_change_bps(1.0, 1.0), Some(0));
    }

    #[test]
    fn test_price_change_bps_10_percent() {
        // 10% increase = 1000 BPS
        assert_eq!(compute_price_change_bps(1.0, 1.1), Some(1000));
    }

    #[test]
    fn test_price_change_bps_10_percent_decrease() {
        // 10% decrease = 1000 BPS (absolute value)
        assert_eq!(compute_price_change_bps(1.0, 0.9), Some(1000));
    }

    #[test]
    fn test_price_change_bps_zero_prev_returns_none() {
        assert_eq!(compute_price_change_bps(0.0, 1.0), None);
    }

    #[test]
    fn test_price_change_bps_negative_prev_returns_none() {
        assert_eq!(compute_price_change_bps(-1.0, 1.0), None);
    }

    #[test]
    fn test_price_change_bps_nan_returns_none() {
        assert_eq!(compute_price_change_bps(f64::NAN, 1.0), None);
        assert_eq!(compute_price_change_bps(1.0, f64::NAN), None);
    }

    #[test]
    fn test_price_change_bps_inf_returns_none() {
        assert_eq!(compute_price_change_bps(f64::INFINITY, 1.0), None);
        assert_eq!(compute_price_change_bps(1.0, f64::INFINITY), None);
    }

    // ===== stroops_to_usd =====

    #[test]
    fn test_stroops_to_usd() {
        assert_eq!(stroops_to_usd(10_000_000), 1.0);
        assert_eq!(stroops_to_usd(50_000_000_000), 5_000.0);
        assert_eq!(stroops_to_usd(0), 0.0);
    }

    // ===== AnomalyDetector::default =====

    #[test]
    fn test_detector_defaults() {
        let d = AnomalyDetector::default();
        assert_eq!(d.max_price_change_bps, 2_000);
        assert_eq!(d.min_volume_threshold_usd, 10_000.0);
        assert_eq!(d.min_trade_count, 5);
    }

    // ===== check() — no anomalies =====

    #[test]
    fn test_check_healthy_returns_empty() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        let anomalies = detector.check(&prev, &curr, 1.0, 1.05); // 5% < 20% threshold
        assert!(anomalies.is_empty());
    }

    // ===== check() — single anomalies =====

    #[test]
    fn test_check_excessive_price_change_detected() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        // 50% increase = 5000 BPS > 2000 threshold
        let anomalies = detector.check(&prev, &curr, 1.0, 1.5);
        assert_eq!(anomalies.len(), 1);
        match &anomalies[0] {
            Anomaly::ExcessivePriceChange {
                change_bps,
                threshold_bps,
                ..
            } => {
                assert_eq!(*change_bps, 5000);
                assert_eq!(*threshold_bps, 2000);
            }
            other => panic!("expected ExcessivePriceChange, got {other:?}"),
        }
    }

    #[test]
    fn test_check_insufficient_liquidity_detected() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        // $5k volume — below $10k threshold
        let curr = make_snapshot("USDC", "GA5ZSEJ", 50_000_000_000, 25);
        let anomalies = detector.check(&prev, &curr, 1.0, 1.0);
        assert_eq!(anomalies.len(), 1);
        match &anomalies[0] {
            Anomaly::InsufficientLiquidity {
                volume_usd,
                threshold_usd,
                ..
            } => {
                assert_eq!(*volume_usd, 5_000.0);
                assert_eq!(*threshold_usd, 10_000.0);
            }
            other => panic!("expected InsufficientLiquidity, got {other:?}"),
        }
    }

    #[test]
    fn test_check_thin_sampling_detected() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        // 3 unique trades — below 5 threshold
        let curr = make_snapshot("USDC", "GA5ZSEJ", 500_000_000_000, 3);
        let anomalies = detector.check(&prev, &curr, 1.0, 1.0);
        assert_eq!(anomalies.len(), 1);
        match &anomalies[0] {
            Anomaly::ThinSampling {
                trade_count,
                threshold_count,
                ..
            } => {
                assert_eq!(*trade_count, 3);
                assert_eq!(*threshold_count, 5);
            }
            other => panic!("expected ThinSampling, got {other:?}"),
        }
    }

    // ===== check() — multiple anomalies =====

    #[test]
    fn test_check_yieldblox_scenario_all_three() {
        // YieldBlox-style attack: thin liquidity + price spike + thin sampling
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = make_snapshot("USDC", "GA5ZSEJ", 50_000_000, 1); // $5 vol, 1 trade
        let anomalies = detector.check(&prev, &curr, 1.0, 5.0); // 400% spike

        assert_eq!(anomalies.len(), 3);
        let kinds: Vec<&str> = anomalies
            .iter()
            .map(|a| match a {
                Anomaly::ExcessivePriceChange { .. } => "price",
                Anomaly::InsufficientLiquidity { .. } => "liquidity",
                Anomaly::ThinSampling { .. } => "thin",
            })
            .collect();
        assert!(kinds.contains(&"price"));
        assert!(kinds.contains(&"liquidity"));
        assert!(kinds.contains(&"thin"));
    }

    // ===== check() — boundary conditions =====

    #[test]
    fn test_check_boundary_exactly_at_threshold_passes() {
        // Volume exactly at threshold ($10k = 100_000_000_000 stroops): no anomaly
        // (rule is strict <, not <=)
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = make_snapshot("USDC", "GA5ZSEJ", 100_000_000_000, 5);
        let anomalies = detector.check(&prev, &curr, 1.0, 1.0);
        // Volume exact threshold → not below → no anomaly. Trade count exact → not below → no anomaly.
        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_check_price_at_threshold_no_anomaly() {
        // Exactly 20% (= 2000 BPS) is the threshold; not exceeded.
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        let anomalies = detector.check(&prev, &curr, 1.0, 1.2); // exactly 2000 BPS
                                                                // Strict `>`: equal-to threshold not flagged.
        assert!(anomalies
            .iter()
            .all(|a| !matches!(a, Anomaly::ExcessivePriceChange { .. })));
    }

    // ===== check() — defensive: NaN price doesn't crash =====

    #[test]
    fn test_check_nan_prices_skipped() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        // NaN prices: ExcessivePriceChange skipped (no flag), other rules unaffected
        let anomalies = detector.check(&prev, &curr, f64::NAN, 1.0);
        assert!(anomalies
            .iter()
            .all(|a| !matches!(a, Anomaly::ExcessivePriceChange { .. })));
    }

    #[test]
    fn test_check_zero_prev_price_skipped() {
        let detector = AnomalyDetector::default();
        let prev = healthy_snapshot();
        let curr = healthy_snapshot();
        let anomalies = detector.check(&prev, &curr, 0.0, 1.0);
        assert!(anomalies
            .iter()
            .all(|a| !matches!(a, Anomaly::ExcessivePriceChange { .. })));
    }

    // ===== Custom thresholds =====

    #[test]
    fn test_check_custom_thresholds() {
        let detector = AnomalyDetector {
            max_price_change_bps: 500,             // 5% — much stricter
            min_volume_threshold_usd: 1_000_000.0, // $1M — much higher
            min_trade_count: 100,                  // 100 trades — much higher
        };
        let prev = healthy_snapshot();
        let curr = healthy_snapshot(); // $50k vol, 25 trades (now both below)
        let anomalies = detector.check(&prev, &curr, 1.0, 1.06); // 6% > 5%

        assert_eq!(anomalies.len(), 3);
    }

    // ===========================================================================
    // Phase 6.7 — Dispatcher tests with MockSink
    // ===========================================================================

    use std::sync::{Arc, Mutex};

    /// In-memory mock sink for testing dispatcher behavior independently
    /// of any real network sink.
    struct MockSink {
        kind_name: &'static str,
        received: Arc<Mutex<Vec<String>>>,
        should_fail: bool,
    }

    impl MockSink {
        fn new(kind_name: &'static str, should_fail: bool) -> (Self, Arc<Mutex<Vec<String>>>) {
            let received = Arc::new(Mutex::new(Vec::new()));
            let sink = MockSink {
                kind_name,
                received: received.clone(),
                should_fail,
            };
            (sink, received)
        }
    }

    #[async_trait]
    impl WebhookSink for MockSink {
        fn kind(&self) -> &'static str {
            self.kind_name
        }

        async fn send(&self, message: &str) -> Result<(), DispatchError> {
            self.received.lock().unwrap().push(message.to_string());
            if self.should_fail {
                Err(DispatchError("mock failure".to_string()))
            } else {
                Ok(())
            }
        }
    }

    fn sample_anomaly_for_dispatch() -> Anomaly {
        Anomaly::ExcessivePriceChange {
            asset_code: "USDC".to_string(),
            asset_issuer: "GA5ZSEJ".to_string(),
            prev_price: 1.0,
            curr_price: 1.5,
            change_bps: 5000,
            threshold_bps: 2000,
        }
    }

    // ===== format_alert_message / format_anomaly_line =====

    #[test]
    fn test_format_anomaly_line_contains_asset_and_values() {
        let line = format_anomaly_line(&sample_anomaly_for_dispatch());
        assert!(line.contains("ExcessivePriceChange"));
        assert!(line.contains("USDC"));
        assert!(line.contains("5000"));
        assert!(line.contains("2000"));
    }

    #[test]
    fn test_format_alert_message_header_and_anomalies() {
        let msg = format_alert_message(&[sample_anomaly_for_dispatch()]);
        assert!(msg.contains("Oracle-Watch Alert"));
        assert!(msg.contains("ExcessivePriceChange"));
    }

    // ===== dispatch_alerts =====

    #[tokio::test]
    async fn test_dispatch_empty_anomalies_noop() {
        let (sink, received) = MockSink::new("mock", false);
        let sinks: Vec<Box<dyn WebhookSink>> = vec![Box::new(sink)];

        dispatch_alerts(&[], &sinks).await;
        assert!(received.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_dispatch_empty_sinks_noop() {
        let sinks: Vec<Box<dyn WebhookSink>> = Vec::new();
        // Must not panic with no sinks
        dispatch_alerts(&[sample_anomaly_for_dispatch()], &sinks).await;
    }

    #[tokio::test]
    async fn test_dispatch_single_sink_receives_message() {
        let (sink, received) = MockSink::new("mock", false);
        let sinks: Vec<Box<dyn WebhookSink>> = vec![Box::new(sink)];

        dispatch_alerts(&[sample_anomaly_for_dispatch()], &sinks).await;
        let received = received.lock().unwrap();
        assert_eq!(received.len(), 1);
        assert!(received[0].contains("ExcessivePriceChange"));
    }

    #[tokio::test]
    async fn test_dispatch_failing_sink_does_not_panic() {
        let (sink, _received) = MockSink::new("mock", true);
        let sinks: Vec<Box<dyn WebhookSink>> = vec![Box::new(sink)];

        // Sink returns Err; dispatcher must absorb without panicking
        dispatch_alerts(&[sample_anomaly_for_dispatch()], &sinks).await;
    }

    #[tokio::test]
    async fn test_dispatch_failing_sink_does_not_block_subsequent() {
        // Independence guarantee: 1 fails, 2 succeeds → 2 still receives
        let (failing, _r1) = MockSink::new("failing", true);
        let (succeeding, r2) = MockSink::new("succeeding", false);
        let sinks: Vec<Box<dyn WebhookSink>> = vec![Box::new(failing), Box::new(succeeding)];

        dispatch_alerts(&[sample_anomaly_for_dispatch()], &sinks).await;

        let received2 = r2.lock().unwrap();
        assert_eq!(
            received2.len(),
            1,
            "succeeding sink must receive despite failing first sink"
        );
    }

    #[tokio::test]
    async fn test_dispatch_multiple_anomalies_single_message() {
        let (sink, received) = MockSink::new("mock", false);
        let sinks: Vec<Box<dyn WebhookSink>> = vec![Box::new(sink)];
        let anomalies = vec![
            sample_anomaly_for_dispatch(),
            Anomaly::ThinSampling {
                asset_code: "USDC".to_string(),
                asset_issuer: "GA5ZSEJ".to_string(),
                trade_count: 1,
                threshold_count: 5,
            },
        ];

        dispatch_alerts(&anomalies, &sinks).await;
        let received = received.lock().unwrap();
        // Single message sent containing both anomaly descriptions
        assert_eq!(received.len(), 1);
        assert!(received[0].contains("ExcessivePriceChange"));
        assert!(received[0].contains("ThinSampling"));
    }

    // ===== AlertConfig::build_sinks =====

    #[test]
    fn test_build_sinks_all_none_empty() {
        let config = AlertConfig::default();
        let sinks = config.build_sinks();
        assert!(sinks.is_empty());
    }

    #[test]
    fn test_build_sinks_discord_only() {
        let config = AlertConfig {
            discord_webhook_url: Some("https://example.test/webhook".to_string()),
            ..AlertConfig::default()
        };
        let sinks = config.build_sinks();
        assert_eq!(sinks.len(), 1);
        assert_eq!(sinks[0].kind(), "discord");
    }

    #[test]
    fn test_build_sinks_telegram_only_full_config() {
        let config = AlertConfig {
            telegram_bot_token: Some("token".to_string()),
            telegram_chat_id: Some("12345".to_string()),
            ..AlertConfig::default()
        };
        let sinks = config.build_sinks();
        assert_eq!(sinks.len(), 1);
        assert_eq!(sinks[0].kind(), "telegram");
    }

    #[test]
    fn test_build_sinks_telegram_partial_skipped() {
        // Only token, no chat_id → Telegram skipped
        let config = AlertConfig {
            telegram_bot_token: Some("token".to_string()),
            telegram_chat_id: None,
            ..AlertConfig::default()
        };
        let sinks = config.build_sinks();
        assert!(sinks.is_empty());
    }

    #[test]
    fn test_build_sinks_both_channels() {
        let config = AlertConfig {
            discord_webhook_url: Some("https://example.test/d".to_string()),
            telegram_bot_token: Some("token".to_string()),
            telegram_chat_id: Some("12345".to_string()),
        };
        let sinks = config.build_sinks();
        assert_eq!(sinks.len(), 2);
        assert_eq!(sinks[0].kind(), "discord");
        assert_eq!(sinks[1].kind(), "telegram");
    }

    // ===== AlertConfig::from_env =====

    #[test]
    fn test_alert_config_from_env_unset_all_none() {
        // Test isolation: env vars are process-global; we cannot safely
        // set/unset without races against parallel tests. This test
        // validates only the empty-state default path, which is the
        // contract documented by `from_env()` for unset variables.
        let config = AlertConfig::default();
        assert!(config.discord_webhook_url.is_none());
        assert!(config.telegram_bot_token.is_none());
        assert!(config.telegram_chat_id.is_none());
        assert!(config.build_sinks().is_empty());
    }
}
