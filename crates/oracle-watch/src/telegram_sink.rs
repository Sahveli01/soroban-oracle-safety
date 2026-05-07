//! Telegram bot sink for oracle-watch alerts.
//!
//! Implements the [`WebhookSink`] trait by calling Telegram's
//! `sendMessage` bot API endpoint. Requires both a bot token (from
//! @BotFather) and a chat ID (from `getUpdates` after the target chat
//! sends the bot a message).

use async_trait::async_trait;

use crate::monitor::{DispatchError, WebhookSink};

/// Telegram bot API base URL.
///
/// Default points to the public api.telegram.org. Tests override this
/// via [`TelegramSink::new_with_base_url`] to route through mockito.
const TELEGRAM_BASE_URL: &str = "https://api.telegram.org";

/// Telegram bot sink.
///
/// Stores `(bot_token, chat_id)` plus an HTTP client. Each `send()`
/// constructs a per-call URL `<base>/bot<token>/sendMessage` and POSTs
/// JSON `{"chat_id": ..., "text": ...}`.
#[derive(Debug, Clone)]
pub struct TelegramSink {
    bot_token: String,
    chat_id: String,
    base_url: String,
    http: reqwest::Client,
}

impl TelegramSink {
    /// Constructs a Telegram sink against the production Telegram API.
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token,
            chat_id,
            base_url: TELEGRAM_BASE_URL.to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Constructs a Telegram sink with a custom base URL (for testing).
    ///
    /// Production code uses [`TelegramSink::new`]. Tests use this to
    /// route through a mockito server.
    #[cfg(test)]
    pub fn new_with_base_url(bot_token: String, chat_id: String, base_url: String) -> Self {
        Self {
            bot_token,
            chat_id,
            base_url,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl WebhookSink for TelegramSink {
    fn kind(&self) -> &'static str {
        "telegram"
    }

    async fn send(&self, message: &str) -> Result<(), DispatchError> {
        let url = format!("{}/bot{}/sendMessage", self.base_url, self.bot_token);
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": message,
        });

        let response = self
            .http
            .post(&url)
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
    async fn test_kind_returns_telegram() {
        let sink = TelegramSink::new("token".to_string(), "chat".to_string());
        assert_eq!(sink.kind(), "telegram");
    }

    #[tokio::test]
    async fn test_send_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/bot123:abc/sendMessage")
            .with_status(200)
            .with_body(r#"{"ok":true}"#)
            .create_async()
            .await;

        let sink = TelegramSink::new_with_base_url(
            "123:abc".to_string(),
            "12345".to_string(),
            server.url(),
        );

        let result = sink.send("test message").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_4xx_returns_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/bot123:abc/sendMessage")
            .with_status(400)
            .with_body(r#"{"ok":false,"description":"bad request"}"#)
            .create_async()
            .await;

        let sink = TelegramSink::new_with_base_url(
            "123:abc".to_string(),
            "12345".to_string(),
            server.url(),
        );

        let result = sink.send("test").await;
        match result {
            Err(DispatchError(msg)) => assert!(msg.contains("400")),
            Ok(()) => panic!("expected error"),
        }
    }
}
