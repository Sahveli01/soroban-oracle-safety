//! Stellar Horizon API HTTP client for SDEX trade history.
//!
//! Wraps `reqwest` with typed responses for `/trades` endpoint queries.

use crate::types::TradeRecord;
use serde::Deserialize;

/// Errors that can occur during Horizon API interaction.
#[derive(Debug)]
pub enum HorizonError {
    /// HTTP-level failure (network, DNS, TLS, non-2xx response).
    Http(reqwest::Error),

    /// Response body could not be parsed as expected JSON.
    Parse(reqwest::Error),

    /// Horizon returned a 4xx/5xx with body content (rate limit, bad request).
    Api { status: u16, body: String },
}

impl std::fmt::Display for HorizonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HorizonError::Http(e) => write!(f, "horizon http error: {e}"),
            HorizonError::Parse(e) => write!(f, "horizon parse error: {e}"),
            HorizonError::Api { status, body } => {
                write!(f, "horizon api error {status}: {body}")
            }
        }
    }
}

impl std::error::Error for HorizonError {}

/// Wrapper around Horizon's HAL+JSON envelope.
///
/// Horizon paginates trade lists inside `_embedded.records[]`. We extract
/// just the records here; pagination cursors are out of scope for the
/// 30-minute polling window (Phase 6.3 truncates by `ledger_close_time`).
#[derive(Debug, Deserialize)]
struct HorizonTradesEnvelope {
    _embedded: HorizonTradesEmbedded,
}

#[derive(Debug, Deserialize)]
struct HorizonTradesEmbedded {
    records: Vec<TradeRecord>,
}

/// HTTP client for Stellar Horizon API.
///
/// Holds a single `reqwest::Client` for connection reuse across polls.
/// The base URL is configurable per [`crate::config::Config::horizon_url`].
#[derive(Debug, Clone)]
pub struct HorizonClient {
    http: reqwest::Client,
    base_url: String,
}

impl HorizonClient {
    /// Constructs a new client for the given Horizon base URL.
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    /// Fetches recent trades for an SDEX asset pair.
    ///
    /// # Parameters
    ///
    /// - `base_code` / `base_issuer`: base asset identification (e.g.,
    ///   `("USDC", "GA5ZSEJ...")` or `("XLM", "native")`).
    /// - `counter_code` / `counter_issuer`: counter asset identification.
    /// - `limit`: maximum number of records to return (Horizon caps at 200
    ///   per page; this client does not paginate — Phase 6.3 polls
    ///   frequently enough that a single page suffices for 30-minute
    ///   windows on actively traded pairs).
    ///
    /// Trades are returned newest-first per Horizon convention. The
    /// `since_seconds` parameter from the spec is not used directly here
    /// — instead, Phase 6.3 filters by `ledger_close_time` after fetching.
    /// This keeps the HTTP layer stateless and simpler to mock.
    pub async fn get_recent_trades(
        &self,
        base_code: &str,
        base_issuer: &str,
        counter_code: &str,
        counter_issuer: &str,
        limit: u32,
    ) -> Result<Vec<TradeRecord>, HorizonError> {
        let (base_asset_type, base_asset_qs) = asset_query_params(base_code, base_issuer);
        let (counter_asset_type, counter_asset_qs) =
            asset_query_params(counter_code, counter_issuer);

        let url = format!(
            "{base}/trades?base_asset_type={bat}{baq}&counter_asset_type={cat}{caq}&limit={limit}&order=desc",
            base = self.base_url.trim_end_matches('/'),
            bat = base_asset_type,
            baq = base_asset_qs,
            cat = counter_asset_type,
            caq = counter_asset_qs,
            limit = limit,
        );

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(HorizonError::Http)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(HorizonError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let envelope: HorizonTradesEnvelope = response.json().await.map_err(HorizonError::Parse)?;
        Ok(envelope._embedded.records)
    }
}

/// Builds Horizon `asset_type` + `asset_code/asset_issuer` query string.
///
/// - `(code="XLM", issuer="native")` → `("native", "")`
/// - `(code, issuer)` → `("credit_alphanum4", "&asset_code=...&asset_issuer=...")`
///   (Horizon uses `credit_alphanum12` for codes >4 chars; we infer.)
fn asset_query_params(code: &str, issuer: &str) -> (&'static str, String) {
    if issuer == "native" {
        return ("native", String::new());
    }

    let asset_type = if code.len() <= 4 {
        "credit_alphanum4"
    } else {
        "credit_alphanum12"
    };

    let qs = format!("&asset_code={code}&asset_issuer={issuer}");
    (asset_type, qs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn sample_horizon_response() -> &'static str {
        r#"{
          "_embedded": {
            "records": [
              {
                "id": "165255658479288321-0",
                "ledger_close_time": "2026-05-06T10:00:00Z",
                "base_amount": "100.0000000",
                "counter_amount": "12345.6789012",
                "price_r": { "n": 12345, "d": 100 },
                "base_account": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
              },
              {
                "id": "165255658479288322-0",
                "ledger_close_time": "2026-05-06T09:59:55Z",
                "base_amount": "50.0000000",
                "counter_amount": "6172.8394506",
                "price_r": { "n": 6173, "d": 50 },
                "base_account": "GBVZUSEAWQHBVB6QF6AGFCDR2K2ZH6DVYWQGNVHIOI6F5WYSAXTLGNQ4"
              }
            ]
          }
        }"#
    }

    #[tokio::test]
    async fn test_get_recent_trades_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/trades\?.*".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(sample_horizon_response())
            .create_async()
            .await;

        let client = HorizonClient::new(server.url());
        let result = client
            .get_recent_trades("USDC", "GA5ZSEJ", "XLM", "native", 200)
            .await
            .expect("expected ok response");

        mock.assert_async().await;
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "165255658479288321-0");
        assert_eq!(result[0].base_amount, "100.0000000");
        assert_eq!(
            result[0].source_account,
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        );
        assert_eq!(result[0].price_r.n, 12345);
    }

    #[tokio::test]
    async fn test_get_recent_trades_api_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/trades\?.*".to_string()))
            .with_status(429)
            .with_body("rate limited")
            .create_async()
            .await;

        let client = HorizonClient::new(server.url());
        let result = client
            .get_recent_trades("USDC", "GA5ZSEJ", "XLM", "native", 200)
            .await;

        match result {
            Err(HorizonError::Api { status, .. }) => assert_eq!(status, 429),
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_get_recent_trades_parse_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/trades\?.*".to_string()))
            .with_status(200)
            .with_body("not json")
            .create_async()
            .await;

        let client = HorizonClient::new(server.url());
        let result = client
            .get_recent_trades("USDC", "GA5ZSEJ", "XLM", "native", 200)
            .await;

        assert!(matches!(result, Err(HorizonError::Parse(_))));
    }

    #[test]
    fn test_asset_query_params_native() {
        let (asset_type, qs) = asset_query_params("XLM", "native");
        assert_eq!(asset_type, "native");
        assert_eq!(qs, "");
    }

    #[test]
    fn test_asset_query_params_credit_alphanum4() {
        let (asset_type, qs) = asset_query_params("USDC", "GA5ZSEJ");
        assert_eq!(asset_type, "credit_alphanum4");
        assert!(qs.contains("asset_code=USDC"));
        assert!(qs.contains("asset_issuer=GA5ZSEJ"));
    }

    #[test]
    fn test_asset_query_params_credit_alphanum12() {
        let (asset_type, qs) = asset_query_params("LONGCODE12", "GA5ZSEJ");
        assert_eq!(asset_type, "credit_alphanum12");
        assert!(qs.contains("asset_code=LONGCODE12"));
    }
}
