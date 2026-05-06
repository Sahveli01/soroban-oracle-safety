//! Anomaly detection and alert dispatch.
//!
//! Compares consecutive snapshots for excessive price changes, insufficient
//! liquidity, or thin sampling. Dispatches to Discord/Telegram webhooks.
//!
//! Implemented across Phase 6.6 (detection) and Phase 6.7 (dispatch).

// TODO Phase 6.6: AnomalyDetector struct (thresholds)
// TODO Phase 6.6: Anomaly enum (ExcessivePriceChange, InsufficientLiquidity, ThinSampling)
// TODO Phase 6.6: check() comparing prev/curr snapshots
// TODO Phase 6.7: AlertConfig struct (discord_webhook, telegram_webhook)
// TODO Phase 6.7: dispatch_alerts() async webhook POST
