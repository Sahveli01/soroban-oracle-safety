//! Generic webhook sink for oracle-watch alerts.
//!
//! Implements the [`WebhookSink`] trait by POSTing a small, stable JSON
//! payload to an arbitrary operator-configured URL, optionally with
//! custom HTTP headers (e.g. an `Authorization` bearer token). This is
//! the least-opinionated sink — it integrates with any HTTP endpoint
//! (an internal monitoring API, a serverless function, a relay).
//!
//! # Payload shape
//!
//! ```json
//! { "message": "<dispatcher alert text>", "source": "oracle-watch" }
//! ```
//!
//! The dispatcher produces one shared plain-text message for all sinks;
//! this sink forwards it verbatim under `message` so downstream systems
//! can parse or re-format as they wish.
//!
//! Configuration:
//! - `ORACLE_WATCH_GENERIC_WEBHOOK_URL` — target endpoint URL
//! - `ORACLE_WATCH_GENERIC_WEBHOOK_HEADERS` — optional, comma-separated
//!   `key:value` pairs, e.g. `Authorization:Bearer abc,X-Source:ow`

use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::monitor::{DispatchError, WebhookSink};

/// Generic webhook sink — POST JSON to an arbitrary URL.
///
/// Holds the target URL, parsed custom headers, and a reused
/// `reqwest::Client`. Mirrors [`crate::discord_sink::DiscordSink`]; the
/// payload is channel-agnostic and headers are caller-supplied.
#[derive(Debug, Clone)]
pub struct GenericWebhookSink {
    webhook_url: String,
    headers: HashMap<String, String>,
    http: reqwest::Client,
}

impl GenericWebhookSink {
    /// Constructs a generic webhook sink for the given URL and headers.
    pub fn new(webhook_url: String, headers: HashMap<String, String>) -> Self {
        Self {
            webhook_url,
            headers,
            http: reqwest::Client::new(),
        }
    }

    /// Parses a `"key1:val1,key2:val2"` string into a header map.
    ///
    /// - Pairs without a `:` are skipped (malformed — no reasonable
    ///   interpretation).
    /// - Empty / whitespace-only segments are skipped.
    /// - Only the first `:` splits, so values may contain `:` (e.g.
    ///   `Authorization:Bearer x:y` → value `Bearer x:y`).
    /// - Keys and values are trimmed of surrounding whitespace.
    pub fn parse_headers(header_str: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for pair in header_str.split(',').filter(|s| !s.trim().is_empty()) {
            if let Some((k, v)) = pair.split_once(':') {
                let k = k.trim();
                if !k.is_empty() {
                    result.insert(k.to_string(), v.trim().to_string());
                }
            }
        }
        result
    }
}

#[async_trait]
impl WebhookSink for GenericWebhookSink {
    fn kind(&self) -> &'static str {
        "generic-webhook"
    }

    async fn send(&self, message: &str) -> Result<(), DispatchError> {
        let body = serde_json::json!({
            "message": message,
            "source": "oracle-watch",
        });

        let mut header_map = HeaderMap::new();
        for (k, v) in &self.headers {
            let name = HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| DispatchError(format!("invalid header name '{k}': {e}")))?;
            let value = HeaderValue::from_str(v)
                .map_err(|e| DispatchError(format!("invalid header value for '{k}': {e}")))?;
            header_map.insert(name, value);
        }

        let response = self
            .http
            .post(&self.webhook_url)
            .headers(header_map)
            .json(&body)
            .send()
            .await
            .map_err(|e| DispatchError(format!("network: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(DispatchError(format!("status {status}: {body_text}")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_kind_returns_generic_webhook() {
        let sink = GenericWebhookSink::new("https://example.test/h".to_string(), HashMap::new());
        assert_eq!(sink.kind(), "generic-webhook");
    }

    #[tokio::test]
    async fn test_send_success_no_headers() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/webhook")
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let sink = GenericWebhookSink::new(format!("{}/webhook", server.url()), HashMap::new());
        let result = sink.send("test message").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_with_headers() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .match_header("Authorization", "Bearer test123")
            .match_header("X-Custom", "foo")
            .with_status(200)
            .create_async()
            .await;

        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer test123".to_string());
        headers.insert("X-Custom".to_string(), "foo".to_string());

        let sink = GenericWebhookSink::new(server.url(), headers);
        let result = sink.send("test message").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_5xx_returns_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(500)
            .with_body("server error")
            .create_async()
            .await;

        let sink = GenericWebhookSink::new(server.url(), HashMap::new());
        let result = sink.send("test").await;

        match result {
            Err(DispatchError(msg)) => assert!(msg.contains("500")),
            Ok(()) => panic!("expected error"),
        }
    }

    #[test]
    fn test_parse_headers() {
        let parsed = GenericWebhookSink::parse_headers("Authorization:Bearer abc,X-API-Key:xyz123");
        assert_eq!(parsed.get("Authorization"), Some(&"Bearer abc".to_string()));
        assert_eq!(parsed.get("X-API-Key"), Some(&"xyz123".to_string()));
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_parse_headers_empty() {
        assert_eq!(GenericWebhookSink::parse_headers("").len(), 0);
        assert_eq!(GenericWebhookSink::parse_headers("   ").len(), 0);
    }

    #[test]
    fn test_parse_headers_malformed_skipped() {
        // No colon → skipped; valid pair still parsed.
        let parsed = GenericWebhookSink::parse_headers("nocolon,Good:value");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.get("Good"), Some(&"value".to_string()));
    }

    #[test]
    fn test_parse_headers_value_with_colon() {
        // Only the first colon splits — value may contain colons.
        let parsed = GenericWebhookSink::parse_headers("Authorization:Bearer a:b:c");
        assert_eq!(
            parsed.get("Authorization"),
            Some(&"Bearer a:b:c".to_string())
        );
    }
}
