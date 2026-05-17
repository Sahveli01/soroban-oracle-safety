//! Slack incoming-webhook sink for oracle-watch alerts.
//!
//! Implements the [`WebhookSink`] trait by POSTing a Block Kit payload
//! to a Slack incoming-webhook URL. Webhook URLs are issued by Slack's
//! "Apps → Incoming Webhooks" UI on a per-channel basis.
//!
//! # Payload shape
//!
//! The dispatcher passes a single plain-text alert message (shared across
//! all sinks). This sink wraps it in a Block Kit `attachments` envelope:
//! a colored header block plus a `mrkdwn` section carrying the message
//! body. There is no per-anomaly severity in the [`WebhookSink`]
//! contract (the dispatcher only fires on detected anomalies, which are
//! uniformly operator-actionable), so a single fixed alert color is used
//! rather than a severity gradient.
//!
//! Configuration: `ORACLE_WATCH_SLACK_WEBHOOK_URL` (full hooks.slack.com URL).

use async_trait::async_trait;

use crate::monitor::{DispatchError, WebhookSink};

/// Slack alert accent color (Slack attachment `color`, hex sans `#` works
/// too but Slack accepts the leading `#`). Amber — every dispatched
/// message is an anomaly worth an operator's attention.
const SLACK_ALERT_COLOR: &str = "#FFA500";

/// Slack incoming-webhook sink.
///
/// Holds the webhook URL and a reused `reqwest::Client` for connection
/// reuse across alerts. Mirrors [`crate::discord_sink::DiscordSink`];
/// only the JSON envelope differs.
#[derive(Debug, Clone)]
pub struct SlackSink {
    webhook_url: String,
    http: reqwest::Client,
}

impl SlackSink {
    /// Constructs a Slack sink for the given incoming-webhook URL.
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl WebhookSink for SlackSink {
    fn kind(&self) -> &'static str {
        "slack"
    }

    async fn send(&self, message: &str) -> Result<(), DispatchError> {
        // Block Kit: header (plain_text, no markdown) + section (mrkdwn).
        // `text` at the top level is the notification fallback shown in
        // push notifications and clients that don't render blocks.
        let body = serde_json::json!({
            "text": "oracle-watch alert",
            "attachments": [{
                "color": SLACK_ALERT_COLOR,
                "blocks": [
                    {
                        "type": "header",
                        "text": {
                            "type": "plain_text",
                            "text": "⚠️ oracle-watch alert",
                            "emoji": true
                        }
                    },
                    {
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": format!("```{message}```")
                        }
                    }
                ]
            }]
        });

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
    async fn test_kind_returns_slack() {
        let sink = SlackSink::new("https://example.test/webhook".to_string());
        assert_eq!(sink.kind(), "slack");
    }

    #[tokio::test]
    async fn test_send_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/webhook")
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let sink = SlackSink::new(format!("{}/webhook", server.url()));
        let result = sink.send("test message").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_4xx_returns_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/webhook")
            .with_status(400)
            .with_body("invalid_payload")
            .create_async()
            .await;

        let sink = SlackSink::new(format!("{}/webhook", server.url()));
        let result = sink.send("test").await;

        match result {
            Err(DispatchError(msg)) => assert!(msg.contains("400")),
            Ok(()) => panic!("expected error"),
        }
    }

    #[tokio::test]
    async fn test_send_unroutable_ip_returns_dispatch_error() {
        // RFC 5737 TEST-NET-1 (192.0.2.0/24): guaranteed-unroutable, no
        // DNS involved — matches the Discord sink's reliability test so
        // DNS-hijacking ISPs cannot turn the failure into a false 200.
        let sink = SlackSink::new("http://192.0.2.1:1/webhook".to_string());
        let result = sink.send("test message").await;
        assert!(result.is_err());
    }
}
