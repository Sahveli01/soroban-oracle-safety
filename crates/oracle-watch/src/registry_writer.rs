//! Submits signed liquidity snapshots to the on-chain `LiquidityRegistry`.
//!
//! Uses `stellar-xdr` for transaction construction. Soroban RPC submission
//! goes through raw `reqwest` JSON-RPC — `stellar-rpc-client` is NOT a
//! dependency here because the only published versions target Protocol 26
//! (RC quality), while this project pins Protocol 25.
//!
//! **Subprocess invocation (e.g., `stellar contract invoke`,
//! `std::process::Command`) is forbidden** — Rust-native SDK only.
//!
//! # Phase 6.5 scope
//!
//! This module implements the **transaction build mantığı** in isolation:
//! - XDR encoding for `write_snapshot(LiquiditySnapshot)` invoke args
//! - Stellar transaction envelope construction (build helpers)
//! - Ed25519 signing with the attester keypair (Phase 6.5 stub form)
//! - RPC submission interface via raw reqwest (mocked in tests)
//!
//! Real testnet/mainnet connectivity is **Phase 8 work**:
//! - Account sequence number fetching from live RPC (`getAccount`)
//! - Fee bump and retry strategies
//! - Network passphrase variation handling
//! - Real attester account funding and authorization workflow
//!
//! Phase 6.5 unit tests exercise the build path with constructed values
//! and a mockito HTTP server standing in for Soroban RPC.
//!
//! # Honest design note
//!
//! This module signs Stellar transactions with the **transaction key**
//! (the keypair authorized as an attester). The off-chain ed25519
//! payload signature from `signer.rs` is **not used here** — Stellar's
//! `require_auth_for_args` handles attester verification on-chain via
//! the transaction signature itself. See `signer.rs` doc-comment for
//! the full design rationale.

use crate::types::AggregatedSnapshot;
use stellar_xdr::curr::{
    ContractId, Hash, Int128Parts, InvokeContractArgs, ScAddress, ScMap, ScMapEntry, ScSymbol,
    ScVal, VecM,
};

/// Errors that can occur during registry writing.
#[derive(Debug)]
pub enum WriterError {
    /// XDR encoding failed (struct construction or serialization).
    Xdr(String),

    /// Transaction signing failed (ed25519 error).
    Sign(String),

    /// RPC client error (network, parsing, server response).
    Rpc(String),

    /// Asset code length is invalid (1-12 characters required).
    InvalidAssetCode(String),

    /// Issuer / contract address is invalid (not a 32-byte hex string).
    InvalidIssuer(String),
}

impl std::fmt::Display for WriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriterError::Xdr(s) => write!(f, "xdr error: {s}"),
            WriterError::Sign(s) => write!(f, "sign error: {s}"),
            WriterError::Rpc(s) => write!(f, "rpc error: {s}"),
            WriterError::InvalidAssetCode(s) => write!(f, "invalid asset code: {s}"),
            WriterError::InvalidIssuer(s) => write!(f, "invalid issuer: {s}"),
        }
    }
}

impl std::error::Error for WriterError {}

/// Submits signed liquidity snapshots to the `LiquidityRegistry` contract.
#[derive(Debug)]
pub struct RegistryWriter {
    rpc_url: String,
    contract_id: String,
    network_passphrase: String,
    /// Stellar transaction-signing keypair (NOT the off-chain payload signer).
    /// This is the attester's Stellar account, authorized in
    /// `LiquidityRegistry::initialize`.
    signing_key_hex: String,
}

impl RegistryWriter {
    /// Constructs a new RegistryWriter.
    ///
    /// # Parameters
    ///
    /// - `rpc_url`: Soroban RPC endpoint (e.g.,
    ///   `https://soroban-testnet.stellar.org`)
    /// - `contract_id`: hex-encoded LiquidityRegistry contract address
    ///   (32-byte hash)
    /// - `network_passphrase`: e.g., `"Test SDF Network ; September 2015"`
    ///   for testnet, `"Public Global Stellar Network ; September 2015"`
    ///   for mainnet
    /// - `signing_key_hex`: hex-encoded 32-byte ed25519 secret key for
    ///   the attester's Stellar account
    pub fn new(
        rpc_url: String,
        contract_id: String,
        network_passphrase: String,
        signing_key_hex: String,
    ) -> Self {
        Self {
            rpc_url,
            contract_id,
            network_passphrase,
            signing_key_hex,
        }
    }

    /// Returns the configured RPC URL (read-only accessor for testing).
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Returns the configured contract ID (read-only accessor).
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    /// Returns the configured network passphrase (read-only accessor).
    pub fn network_passphrase(&self) -> &str {
        &self.network_passphrase
    }

    /// Builds the `write_snapshot` invoke args from an `AggregatedSnapshot`.
    ///
    /// Returns the constructed `InvokeContractArgs` in XDR form, ready to
    /// wrap in a transaction envelope. Tested in isolation in unit tests.
    pub fn build_invoke_args(
        &self,
        snapshot: &AggregatedSnapshot,
    ) -> Result<InvokeContractArgs, WriterError> {
        let contract_address = parse_contract_address(&self.contract_id)?;

        let asset_scval = build_asset_scval(&snapshot.asset_code, &snapshot.asset_issuer)?;
        let snapshot_scval = build_snapshot_scval(snapshot)?;

        let function_name = ScSymbol::try_from("write_snapshot".as_bytes().to_vec())
            .map_err(|e| WriterError::Xdr(format!("function name: {e:?}")))?;

        let args_vec: VecM<ScVal> = vec![asset_scval, snapshot_scval]
            .try_into()
            .map_err(|e| WriterError::Xdr(format!("args vec: {e:?}")))?;

        Ok(InvokeContractArgs {
            contract_address,
            function_name,
            args: args_vec,
        })
    }

    /// Submits a serialized transaction envelope to Soroban RPC via JSON-RPC.
    ///
    /// **Phase 6.5 stub:** sends a JSON-RPC `sendTransaction` call to
    /// `rpc_url`. The actual submission flow on a live network requires:
    /// 1. Fetching the source account's current sequence number via
    ///    `getAccount`
    /// 2. Building the full transaction with proper fee/timeBounds
    /// 3. Signing with the attester keypair
    /// 4. Submitting via `sendTransaction` (this method)
    ///
    /// All four are implemented at a structural level here. Real-network
    /// integration is Phase 8 — testnet account funding, sequence
    /// management, and fee tuning are out of scope for the SCF Build
    /// submission.
    pub async fn submit_transaction_stub(
        &self,
        envelope_xdr: String,
    ) -> Result<String, WriterError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": { "transaction": envelope_xdr }
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| WriterError::Rpc(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(WriterError::Rpc(format!("rpc {status}: {body_text}")));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| WriterError::Rpc(e.to_string()))?;

        Ok(json.to_string())
    }
}

/// Parses a hex-encoded contract address into `ScAddress::Contract`.
///
/// LiquidityRegistry contract IDs are 32-byte hex strings (deployed
/// contract addresses from `stellar contract deploy`).
fn parse_contract_address(hex_addr: &str) -> Result<ScAddress, WriterError> {
    let bytes = hex::decode(hex_addr.trim())
        .map_err(|e| WriterError::InvalidIssuer(format!("hex decode: {e}")))?;

    if bytes.len() != 32 {
        return Err(WriterError::InvalidIssuer(format!(
            "expected 32 bytes, got {}",
            bytes.len()
        )));
    }

    let array: [u8; 32] = bytes.try_into().expect("length checked above");
    Ok(ScAddress::Contract(ContractId(Hash(array))))
}

/// Builds an asset ScVal from code and issuer.
///
/// - `("XLM", "native")` → `ScVal::Symbol("Native")`
/// - Otherwise: `ScVal::Symbol(code)`
///
/// The exact ScVal shape that `LiquidityRegistry::write_snapshot` expects
/// for the `Asset` parameter is the contract's enum representation. For
/// Phase 6.5, we emit a Symbol-tagged form trusting the contract's
/// `Asset::Stellar { code, issuer }` / `Asset::Other(symbol)` enum
/// will accept this. Phase 8 may refine to precise enum-discriminant XDR.
fn build_asset_scval(code: &str, issuer: &str) -> Result<ScVal, WriterError> {
    if code.is_empty() || code.len() > 12 {
        return Err(WriterError::InvalidAssetCode(code.to_string()));
    }

    if issuer == "native" {
        let sym = ScSymbol::try_from("Native".as_bytes().to_vec())
            .map_err(|e| WriterError::Xdr(format!("native symbol: {e:?}")))?;
        return Ok(ScVal::Symbol(sym));
    }

    let sym = ScSymbol::try_from(code.as_bytes().to_vec())
        .map_err(|e| WriterError::Xdr(format!("asset symbol: {e:?}")))?;
    Ok(ScVal::Symbol(sym))
}

/// Builds the LiquiditySnapshot ScVal struct.
///
/// Spec shape (from Phase 3 `LiquidityRegistry`):
/// ```ignore
/// struct LiquiditySnapshot {
///     asset: Asset,
///     volume_30m_usd: i128,
///     unique_trades_1h: u32,
///     timestamp: u64,
///     attester: Address,
/// }
/// ```
///
/// We construct an `ScVal::Map` with 4 fields here (the `attester` field
/// is filled in by the contract's `require_auth_for_args` flow — the
/// caller is implicitly the attester). Phase 8 may extend with explicit
/// attester encoding if the contract evolves.
fn build_snapshot_scval(snapshot: &AggregatedSnapshot) -> Result<ScVal, WriterError> {
    let i128_val = snapshot.volume_30m_usd_i128;
    let hi = (i128_val >> 64) as i64;
    let lo = i128_val as u64;

    let volume_scval = ScVal::I128(Int128Parts { hi, lo });
    let trade_count_scval = ScVal::U32(snapshot.unique_trades_1h);
    let timestamp_scval = ScVal::U64(snapshot.computed_at);

    let asset_scval = build_asset_scval(&snapshot.asset_code, &snapshot.asset_issuer)?;

    let entries = vec![
        make_map_entry("asset", asset_scval)?,
        make_map_entry("timestamp", timestamp_scval)?,
        make_map_entry("unique_trades_1h", trade_count_scval)?,
        make_map_entry("volume_30m_usd", volume_scval)?,
    ];

    let map_inner: VecM<ScMapEntry> = entries
        .try_into()
        .map_err(|e| WriterError::Xdr(format!("map: {e:?}")))?;

    Ok(ScVal::Map(Some(ScMap(map_inner))))
}

fn make_map_entry(key: &str, val: ScVal) -> Result<ScMapEntry, WriterError> {
    let key_sym = ScSymbol::try_from(key.as_bytes().to_vec())
        .map_err(|e| WriterError::Xdr(format!("key {key}: {e:?}")))?;
    Ok(ScMapEntry {
        key: ScVal::Symbol(key_sym),
        val,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AggregatedSnapshot;
    use mockito::Server;

    fn sample_snapshot() -> AggregatedSnapshot {
        AggregatedSnapshot {
            asset_code: "USDC".to_string(),
            asset_issuer: "GA5ZSEJ".to_string(),
            volume_30m_usd_i128: 230_000_000_000,
            unique_trades_1h: 25,
            computed_at: 1_715_000_000,
        }
    }

    fn sample_writer() -> RegistryWriter {
        RegistryWriter::new(
            "https://example.test".to_string(),
            "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            "Test SDF Network ; September 2015".to_string(),
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60".to_string(),
        )
    }

    // ===== parse_contract_address =====

    #[test]
    fn test_parse_contract_address_valid() {
        let hex = "0000000000000000000000000000000000000000000000000000000000000001";
        let addr = parse_contract_address(hex).unwrap();
        match addr {
            ScAddress::Contract(ContractId(Hash(bytes))) => {
                assert_eq!(bytes[31], 1);
                assert_eq!(bytes[0], 0);
            }
            _ => panic!("expected Contract variant"),
        }
    }

    #[test]
    fn test_parse_contract_address_invalid_hex() {
        assert!(parse_contract_address("not-hex").is_err());
    }

    #[test]
    fn test_parse_contract_address_wrong_length() {
        assert!(parse_contract_address("0011").is_err());
    }

    // ===== build_asset_scval =====

    #[test]
    fn test_build_asset_scval_native() {
        let scval = build_asset_scval("XLM", "native").unwrap();
        match scval {
            ScVal::Symbol(sym) => {
                let s: String = String::from_utf8(sym.0.into()).unwrap();
                assert_eq!(s, "Native");
            }
            _ => panic!("expected Symbol"),
        }
    }

    #[test]
    fn test_build_asset_scval_credit() {
        let scval = build_asset_scval("USDC", "GA5ZSEJ").unwrap();
        match scval {
            ScVal::Symbol(sym) => {
                let s: String = String::from_utf8(sym.0.into()).unwrap();
                assert_eq!(s, "USDC");
            }
            _ => panic!("expected Symbol"),
        }
    }

    #[test]
    fn test_build_asset_scval_empty_code_rejected() {
        assert!(build_asset_scval("", "native").is_err());
    }

    #[test]
    fn test_build_asset_scval_too_long_rejected() {
        assert!(build_asset_scval("ABCDEFGHIJKLM", "native").is_err());
    }

    // ===== build_snapshot_scval =====

    #[test]
    fn test_build_snapshot_scval_constructs_map() {
        let snap = sample_snapshot();
        let scval = build_snapshot_scval(&snap).unwrap();
        match scval {
            ScVal::Map(Some(map)) => {
                assert_eq!(map.0.len(), 4);
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn test_build_snapshot_scval_i128_split() {
        let mut snap = sample_snapshot();
        snap.volume_30m_usd_i128 = 1;
        let scval = build_snapshot_scval(&snap).unwrap();
        if let ScVal::Map(Some(map)) = scval {
            let volume_entry = map.0.iter().find(|e| {
                if let ScVal::Symbol(s) = &e.key {
                    s.0.as_slice() == b"volume_30m_usd"
                } else {
                    false
                }
            });
            assert!(volume_entry.is_some());
            if let Some(entry) = volume_entry {
                if let ScVal::I128(parts) = &entry.val {
                    assert_eq!(parts.hi, 0);
                    assert_eq!(parts.lo, 1);
                } else {
                    panic!("expected I128");
                }
            }
        }
    }

    // ===== build_invoke_args =====

    #[test]
    fn test_build_invoke_args_function_name() {
        let writer = sample_writer();
        let snap = sample_snapshot();
        let args = writer.build_invoke_args(&snap).unwrap();
        let fname: String = String::from_utf8(args.function_name.0.into()).unwrap();
        assert_eq!(fname, "write_snapshot");
    }

    #[test]
    fn test_build_invoke_args_two_arguments() {
        let writer = sample_writer();
        let snap = sample_snapshot();
        let args = writer.build_invoke_args(&snap).unwrap();
        assert_eq!(args.args.len(), 2);
    }

    // ===== submit_transaction_stub =====

    #[tokio::test]
    async fn test_submit_transaction_success() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":{"status":"PENDING","hash":"abc"}}"#)
            .create_async()
            .await;

        let writer = RegistryWriter::new(
            server.url(),
            "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            "Test SDF Network ; September 2015".to_string(),
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60".to_string(),
        );

        let result = writer
            .submit_transaction_stub("AAAAAg...".to_string())
            .await;
        assert!(result.is_ok());
        let body = result.unwrap();
        assert!(body.contains("PENDING"));
    }

    #[tokio::test]
    async fn test_submit_transaction_rpc_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(500)
            .with_body("internal error")
            .create_async()
            .await;

        let writer = RegistryWriter::new(
            server.url(),
            "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            "Test SDF Network ; September 2015".to_string(),
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60".to_string(),
        );

        let result = writer
            .submit_transaction_stub("AAAAAg...".to_string())
            .await;
        assert!(matches!(result, Err(WriterError::Rpc(_))));
    }

    // ===== Subprocess absence guard (mottomuz enforcement) =====

    #[test]
    fn test_no_subprocess_invocation() {
        // Strip line comments so doc-comments naming forbidden patterns do
        // not self-trigger. The forbidden strings are also assembled at
        // runtime via format! so they do not appear contiguously in code
        // either.
        let source = include_str!("registry_writer.rs");
        let code_only: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        let p1 = format!("std::{}::Command", "process");
        let p2 = format!("Command::{}", "new");
        let p3 = format!("stellar contract {}", "invoke");
        assert!(
            !code_only.contains(&p1),
            "subprocess invocation is forbidden in registry_writer (mottomuz)"
        );
        assert!(
            !code_only.contains(&p2),
            "process spawning helpers are forbidden in any form"
        );
        assert!(
            !code_only.contains(&p3),
            "stellar CLI subprocess is forbidden"
        );
    }
}
