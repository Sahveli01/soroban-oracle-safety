//! PagerDuty Events API v2 sink for oracle-watch alerts.
//!
//! Implements the [`WebhookSink`] trait by triggering a PagerDuty
//! incident via the Events API v2 (`/v2/enqueue`). Each dispatched
//! alert triggers an incident routed by the operator's integration key
//! (the 32-char "Events API v2" routing key from a PagerDuty service).
//!
//! # Severity
//!
//! The [`WebhookSink`] contract carries no per-anomaly severity (the
//! dispatcher fires only on detected anomalies, which are uniformly
//! operator-actionable). PagerDuty requires a `severity` field, so a
//! fixed `"warning"` is sent: it opens an incident without asserting a
//! confirmed outage, which matches oracle-watch's "surface this for a
//! human to judge" semantics.
//!
//! # Deduplication
//!
//! `dedup_key` is a stable hash of the message body. Identical repeated
//! anomalies (same assets, same thresholds) collapse into one open
//! PagerDuty incident instead of paging on every poll.
//!
//! Configuration: `ORACLE_WATCH_PAGERDUTY_INTEGRATION_KEY` (routing key).
//!
//! Reference: <https://developer.pagerduty.com/api-reference/368ae3d938c9e-send-an-event>

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::monitor::{DispatchError, WebhookSink};

/// PagerDuty Events API v2 enqueue endpoint.
///
/// Default points to the public events.pagerduty.com. Tests override
/// this via [`PagerDutySink::new_with_base_url`] to route through
/// mockito (mirrors the Telegram sink's test-injection pattern, since
/// PagerDuty has no per-call URL component to redirect otherwise).
const PAGERDUTY_ENQUEUE_URL: &str = "https://events.pagerduty.com/v2/enqueue";

/// Fixed PagerDuty severity — see module docs for rationale.
const PAGERDUTY_SEVERITY: &str = "warning";

/// PagerDuty Events API v2 alert sink.
#[derive(Debug, Clone)]
pub struct PagerDutySink {
    integration_key: String,
    api_url: String,
    http: reqwest::Client,
}

impl PagerDutySink {
    /// Constructs a PagerDuty sink against the production Events API.
    pub fn new(integration_key: String) -> Self {
        Self {
            integration_key,
            api_url: PAGERDUTY_ENQUEUE_URL.to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Constructs a PagerDuty sink with a custom API URL (for testing).
    ///
    /// Production code uses [`PagerDutySink::new`]. Tests use this to
    /// route through a mockito server.
    #[cfg(test)]
    pub fn new_with_base_url(integration_key: String, api_url: String) -> Self {
        Self {
            integration_key,
            api_url,
            http: reqwest::Client::new(),
        }
    }

    /// Stable dedup key from the message body (hex SHA-256, 32 chars).
    fn dedup_key(message: &str) -> String {
        let digest = Sha256::digest(message.as_bytes());
        format!("oracle-watch-{}", hex::encode(&digest[..16]))
    }
}

#[async_trait]
impl WebhookSink for PagerDutySink {
    fn kind(&self) -> &'static str {
        "pagerduty"
    }

    async fn send(&self, message: &str) -> Result<(), DispatchError> {
        // PagerDuty `summary` is capped at 1024 chars; truncate defensively.
        let summary: String = message.chars().take(1024).collect();

        let body = serde_json::json!({
            "routing_key": self.integration_key,
            "event_action": "trigger",
            "dedup_key": Self::dedup_key(message),
            "payload": {
                "summary": summary,
                "source": "oracle-watch",
                "severity": PAGERDUTY_SEVERITY,
                "custom_details": { "message": message }
            }
        });

        let response = self
            .http
            .post(&self.api_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| DispatchError(format!("network: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(DispatchError(format!("status {status}: {body_text}")));
        }
        // PagerDuty returns 202 with {"status":"success","dedup_key":...}
        // on accept; any 2xx is treated as delivered.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_kind_returns_pagerduty() {
        let sink = PagerDutySink::new("routingkey".to_string());
        assert_eq!(sink.kind(), "pagerduty");
    }

    #[tokio::test]
    async fn test_send_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v2/enqueue")
            .with_status(202)
            .with_body(r#"{"status":"success","dedup_key":"x"}"#)
            .create_async()
            .await;

        let sink = PagerDutySink::new_with_base_url(
            "routingkey".to_string(),
            format!("{}/v2/enqueue", server.url()),
        );
        let result = sink.send("test message").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_4xx_returns_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/v2/enqueue")
            .with_status(400)
            .with_body(r#"{"status":"invalid event","errors":["bad routing_key"]}"#)
            .create_async()
            .await;

        let sink = PagerDutySink::new_with_base_url(
            "routingkey".to_string(),
            format!("{}/v2/enqueue", server.url()),
        );
        let result = sink.send("test").await;

        match result {
            Err(DispatchError(msg)) => assert!(msg.contains("400")),
            Ok(()) => panic!("expected error"),
        }
    }

    #[test]
    fn test_dedup_key_stable_and_message_sensitive() {
        let a1 = PagerDutySink::dedup_key("anomaly A");
        let a2 = PagerDutySink::dedup_key("anomaly A");
        let b = PagerDutySink::dedup_key("anomaly B");
        assert_eq!(a1, a2, "same message → same dedup key");
        assert_ne!(a1, b, "different message → different dedup key");
        assert!(a1.starts_with("oracle-watch-"));
    }
}
