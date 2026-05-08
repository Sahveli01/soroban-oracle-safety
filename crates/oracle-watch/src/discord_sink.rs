//! Discord webhook sink for oracle-watch alerts.
//!
//! Implements the [`WebhookSink`] trait by POSTing a JSON-formatted
//! message to a Discord webhook URL. Webhook URLs are issued by Discord's
//! "Integrations → Webhooks" UI on a per-channel basis.

use async_trait::async_trait;

use crate::monitor::{DispatchError, WebhookSink};

/// Discord webhook sink.
///
/// Holds the webhook URL and a reused `reqwest::Client` for connection
/// reuse across alerts. Messages are wrapped in Discord's JSON envelope
/// `{"content": "..."}`.
#[derive(Debug, Clone)]
pub struct DiscordSink {
    webhook_url: String,
    http: reqwest::Client,
}

impl DiscordSink {
    /// Constructs a Discord sink for the given webhook URL.
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl WebhookSink for DiscordSink {
    fn kind(&self) -> &'static str {
        "discord"
    }

    async fn send(&self, message: &str) -> Result<(), DispatchError> {
        // Discord prefers messages with a small visual indicator in the body.
        // The dispatcher's plain message is wrapped here with Discord-specific
        // formatting; sinks own their channel's presentation conventions.
        let formatted = format!("⚠️ **{}**", message);

        let body = serde_json::json!({ "content": formatted });

        let response = self
            .http
            .post(&self.webhook_url)
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
    async fn test_kind_returns_discord() {
        let sink = DiscordSink::new("https://example.test/webhook".to_string());
        assert_eq!(sink.kind(), "discord");
    }

    #[tokio::test]
    async fn test_send_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/webhook")
            .with_status(204)
            .create_async()
            .await;

        let sink = DiscordSink::new(format!("{}/webhook", server.url()));
        let result = sink.send("test message").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_unroutable_ip_returns_dispatch_error() {
        // Phase 7.2 reliability fix (Phase 7.1 follow-up): the prior
        // `.invalid` hostname assumed DNS would return NXDOMAIN, but some
        // ISPs (notably some Turkish residential ISPs) DNS-hijack unknown
        // names to a search/redirect page that responds HTTP 200 — the
        // assertion `result.is_err()` then failed in those environments.
        //
        // RFC 5737 reserves `192.0.2.0/24` as TEST-NET-1: addresses
        // guaranteed to be unroutable on the public Internet, with no DNS
        // resolution involved. Port 1 is privileged and never bound by
        // ordinary services, so the connection attempt must fail at the
        // network layer regardless of ISP behavior.
        let sink = DiscordSink::new("http://192.0.2.1:1/webhook".to_string());
        let result = sink.send("test message").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_5xx_returns_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/webhook")
            .with_status(500)
            .with_body("internal error")
            .create_async()
            .await;

        let sink = DiscordSink::new(format!("{}/webhook", server.url()));
        let result = sink.send("test").await;

        match result {
            Err(DispatchError(msg)) => assert!(msg.contains("500")),
            Ok(()) => panic!("expected error"),
        }
    }
}
