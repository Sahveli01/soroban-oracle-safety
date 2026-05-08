//! Service configuration loaded from environment variables.

use std::env;

/// oracle-watch service configuration.
///
/// All fields are loaded from environment variables via [`Config::from_env`].
/// Defaults are provided where reasonable; required fields (signing key,
/// registry address, watched assets) have no default and must be set
/// explicitly.
///
/// # Environment variables
///
/// | Variable | Required | Default | Description |
/// |---|---|---|---|
/// | `ORACLE_WATCH_HORIZON_URL` | no | `https://horizon-testnet.stellar.org` | Horizon REST endpoint |
/// | `ORACLE_WATCH_SOROBAN_RPC_URL` | no | `https://soroban-testnet.stellar.org` | Soroban RPC endpoint |
/// | `ORACLE_WATCH_REGISTRY_CONTRACT_ID` | yes | — | LiquidityRegistry contract address (C…) |
/// | `ORACLE_WATCH_POLL_INTERVAL_LEDGERS` | no | `5` | Ledgers between polls (~25s @ 5s/ledger) |
/// | `ORACLE_WATCH_SIGNING_SECRET_KEY` | yes | — | Ed25519 secret key (hex, 64 chars) |
/// | `ORACLE_WATCH_WATCHED_ASSETS` | yes | — | Comma-separated `code:issuer` pairs |
/// | `ORACLE_WATCH_MAX_SNAPSHOT_AGE_SECONDS` | no | `300` | Snapshot freshness threshold |
/// | `ORACLE_WATCH_USDC_PRICE_USD` | no | `1.0` | USD value of 1 unit of counter asset |
/// | `ORACLE_WATCH_NETWORK_PASSPHRASE` | no | testnet | Stellar network passphrase |
/// | `ORACLE_WATCH_COUNTER_ASSET_CODE` | no | `USDC` | Counter asset code for SDEX pair queries |
/// | `ORACLE_WATCH_COUNTER_ASSET_ISSUER` | no | testnet USDC issuer | Counter asset issuer |
///
/// # Watched asset format
///
/// `ORACLE_WATCH_WATCHED_ASSETS` parses as: `CODE1:ISSUER1:SAC1,CODE2:ISSUER2,...`
/// Example: `XLM:native:CDLZFC3...,USDC:GA5ZSEJ...:CCCC...`
#[derive(Debug, Clone)]
pub struct Config {
    pub horizon_url: String,
    pub soroban_rpc_url: String,
    pub registry_contract_id: String,
    pub poll_interval_ledgers: u32,
    pub signing_secret_key: String,
    pub watched_assets: Vec<WatchedAsset>,
    pub max_snapshot_age_seconds: u64,

    /// USD value of 1 unit of the counter asset for volume aggregation.
    ///
    /// **Phase 6.8 placeholder.** Currently a single static value applied
    /// to all watched pairs. Phase 9 (mainnet) will replace this with a
    /// real-time price feed (likely Reflector itself, with circular-dependency
    /// safeguards) per-asset. Default `1.0` is correct for USDC pairs and
    /// approximate for stablecoin-denominated pairs.
    pub usdc_price_usd: f64,

    /// Stellar network passphrase, wired to `RegistryWriter` for transaction
    /// signing. Defaults to testnet; mainnet operators must override via
    /// `ORACLE_WATCH_NETWORK_PASSPHRASE`.
    pub network_passphrase: String,

    /// Counter asset code for SDEX pair queries (e.g. "USDC").
    /// Watched assets are polled as `watched_asset / counter_asset` pairs.
    pub counter_asset_code: String,

    /// Counter asset issuer. Use `"native"` for XLM. Defaults to the
    /// testnet Circle USDC issuer (`GBBD47IF...`). Mainnet operators must
    /// set `ORACLE_WATCH_COUNTER_ASSET_ISSUER` to the correct issuer.
    pub counter_asset_issuer: String,
}

/// A single asset watched by oracle-watch.
///
/// Stellar assets are identified by `(code, issuer)` pairs. The native
/// XLM asset uses the literal issuer string `"native"` per Stellar
/// convention. `sac_contract_id` is the Stellar Asset Contract (SAC)
/// C-address for the asset; used as the on-chain `LiquiditySnapshot.asset`
/// Address field. When `None`, the registry writer falls back to a Symbol
/// (simulation will reject — for unit tests only).
#[derive(Debug, Clone)]
pub struct WatchedAsset {
    pub code: String,
    pub issuer: String,
    /// SAC contract C-address (e.g. `CDLZFC3...`). Set via the 3-part
    /// `CODE:ISSUER:SAC_ADDRESS` format in `ORACLE_WATCH_WATCHED_ASSETS`.
    pub sac_contract_id: Option<String>,
}

/// Errors that can occur while loading configuration from environment.
#[derive(Debug)]
pub enum ConfigError {
    /// Required environment variable is missing.
    MissingVar(&'static str),

    /// `ORACLE_WATCH_POLL_INTERVAL_LEDGERS` could not be parsed as u32.
    InvalidPollInterval(String),

    /// `ORACLE_WATCH_MAX_SNAPSHOT_AGE_SECONDS` could not be parsed as u64.
    InvalidSnapshotAge(String),

    /// `ORACLE_WATCH_WATCHED_ASSETS` is empty or has malformed `code:issuer` pairs.
    InvalidWatchedAssets(String),

    /// `ORACLE_WATCH_USDC_PRICE_USD` could not be parsed as f64.
    InvalidUsdcPrice(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingVar(v) => write!(f, "missing environment variable: {v}"),
            ConfigError::InvalidPollInterval(s) => {
                write!(f, "invalid ORACLE_WATCH_POLL_INTERVAL_LEDGERS: {s}")
            }
            ConfigError::InvalidSnapshotAge(s) => {
                write!(f, "invalid ORACLE_WATCH_MAX_SNAPSHOT_AGE_SECONDS: {s}")
            }
            ConfigError::InvalidWatchedAssets(s) => {
                write!(f, "invalid ORACLE_WATCH_WATCHED_ASSETS: {s}")
            }
            ConfigError::InvalidUsdcPrice(s) => {
                write!(f, "invalid ORACLE_WATCH_USDC_PRICE_USD: {s}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Loads configuration from environment variables.
    ///
    /// See module doc-comment for the full list of variables and their
    /// semantics.
    pub fn from_env() -> Result<Self, ConfigError> {
        let horizon_url = env::var("ORACLE_WATCH_HORIZON_URL")
            .unwrap_or_else(|_| "https://horizon-testnet.stellar.org".to_string());

        let soroban_rpc_url = env::var("ORACLE_WATCH_SOROBAN_RPC_URL")
            .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string());

        let registry_contract_id = env::var("ORACLE_WATCH_REGISTRY_CONTRACT_ID")
            .map_err(|_| ConfigError::MissingVar("ORACLE_WATCH_REGISTRY_CONTRACT_ID"))?;

        let poll_interval_ledgers = env::var("ORACLE_WATCH_POLL_INTERVAL_LEDGERS")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u32>()
            .map_err(|e| ConfigError::InvalidPollInterval(e.to_string()))?;

        let signing_secret_key = env::var("ORACLE_WATCH_SIGNING_SECRET_KEY")
            .map_err(|_| ConfigError::MissingVar("ORACLE_WATCH_SIGNING_SECRET_KEY"))?;

        let watched_assets_raw = env::var("ORACLE_WATCH_WATCHED_ASSETS")
            .map_err(|_| ConfigError::MissingVar("ORACLE_WATCH_WATCHED_ASSETS"))?;
        let watched_assets = parse_watched_assets(&watched_assets_raw)?;

        let max_snapshot_age_seconds = env::var("ORACLE_WATCH_MAX_SNAPSHOT_AGE_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .map_err(|e| ConfigError::InvalidSnapshotAge(e.to_string()))?;

        let usdc_price_usd = env::var("ORACLE_WATCH_USDC_PRICE_USD")
            .unwrap_or_else(|_| "1.0".to_string())
            .parse::<f64>()
            .map_err(|e| ConfigError::InvalidUsdcPrice(e.to_string()))?;

        let network_passphrase = env::var("ORACLE_WATCH_NETWORK_PASSPHRASE")
            .unwrap_or_else(|_| "Test SDF Network ; September 2015".to_string());

        let counter_asset_code =
            env::var("ORACLE_WATCH_COUNTER_ASSET_CODE").unwrap_or_else(|_| "USDC".to_string());

        // Default: testnet Circle USDC issuer. Mainnet operators must override.
        let counter_asset_issuer =
            env::var("ORACLE_WATCH_COUNTER_ASSET_ISSUER").unwrap_or_else(|_| {
                "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string()
            });

        Ok(Config {
            horizon_url,
            soroban_rpc_url,
            registry_contract_id,
            poll_interval_ledgers,
            signing_secret_key,
            watched_assets,
            max_snapshot_age_seconds,
            usdc_price_usd,
            network_passphrase,
            counter_asset_code,
            counter_asset_issuer,
        })
    }
}

/// Parses `code:issuer` comma-separated list into structured assets.
fn parse_watched_assets(raw: &str) -> Result<Vec<WatchedAsset>, ConfigError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::InvalidWatchedAssets("empty list".to_string()));
    }

    let mut assets = Vec::new();
    for pair in trimmed.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let mut parts = pair.splitn(3, ':');
        let code = parts.next().ok_or_else(|| {
            ConfigError::InvalidWatchedAssets(format!("missing code in pair: {pair}"))
        })?;
        let issuer = parts.next().ok_or_else(|| {
            ConfigError::InvalidWatchedAssets(format!("missing issuer in pair: {pair}"))
        })?;
        let sac_contract_id = parts.next().filter(|s| !s.is_empty()).map(str::to_string);
        if code.is_empty() || issuer.is_empty() {
            return Err(ConfigError::InvalidWatchedAssets(format!(
                "empty code or issuer in pair: {pair}"
            )));
        }
        assets.push(WatchedAsset {
            code: code.to_string(),
            issuer: issuer.to_string(),
            sac_contract_id,
        });
    }

    if assets.is_empty() {
        return Err(ConfigError::InvalidWatchedAssets(
            "no valid pairs".to_string(),
        ));
    }

    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_watched_assets_single() {
        let result = parse_watched_assets("USDC:GA5ZSEJ").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "USDC");
        assert_eq!(result[0].issuer, "GA5ZSEJ");
    }

    #[test]
    fn test_parse_watched_assets_multiple() {
        let result = parse_watched_assets("USDC:GA5ZSEJ,XLM:native").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].code, "XLM");
        assert_eq!(result[1].issuer, "native");
    }

    #[test]
    fn test_parse_watched_assets_empty_rejected() {
        assert!(parse_watched_assets("").is_err());
    }

    #[test]
    fn test_parse_watched_assets_malformed_rejected() {
        assert!(parse_watched_assets("USDCMISSINGCOLON").is_err());
        assert!(parse_watched_assets(":missing-code").is_err());
        assert!(parse_watched_assets("missing-issuer:").is_err());
    }

    #[test]
    fn test_parse_watched_assets_whitespace_tolerant() {
        let result = parse_watched_assets("  USDC:GA5ZSEJ  ,  XLM:native  ").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].code, "USDC");
    }
}
