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
//! # Phase 7.3 scope
//!
//! This module implements the **complete transaction submission path**:
//! 1. Account sequence fetch (Horizon GET /accounts/{id})
//! 2. Build base TransactionEnvelopeV1 (no Soroban resources yet)
//! 3. simulateTransaction RPC → footprint + min resource fee
//! 4. Attach SorobanTransactionData to envelope
//! 5. Compute envelope hash (SHA256 of TransactionSignaturePayload XDR)
//! 6. Sign hash with attester ed25519 key (ed25519-dalek)
//! 7. Attach DecoratedSignature to envelope
//! 8. sendTransaction RPC → tx hash + PENDING status
//! 9. Poll getTransaction every 1s (max 30s) until SUCCESS or FAILED
//!
//! Real testnet/mainnet end-to-end is Phase 7.7 work (requires funded
//! attester account). Phase 7.3 builds the mechanical plumbing; every
//! step is exercised by mockito-based unit tests.

use crate::types::AggregatedSnapshot;
use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use sha2::{Digest, Sha256};
use stellar_xdr::curr::{
    AccountId, BytesM, ContractId, DecoratedSignature, Hash, HostFunction, Int128Parts,
    InvokeContractArgs, InvokeHostFunctionOp, Limits, Memo, MuxedAccount, Operation, OperationBody,
    Preconditions, PublicKey, ReadXdr, ScAddress, ScMap, ScMapEntry, ScSymbol, ScVal,
    SequenceNumber, Signature, SignatureHint, SorobanAuthorizationEntry, SorobanTransactionData,
    Transaction, TransactionEnvelope, TransactionExt, TransactionSignaturePayload,
    TransactionSignaturePayloadTaggedTransaction, TransactionV1Envelope, Uint256, VecM, WriteXdr,
};

/// Errors that can occur during registry writing.
#[derive(Debug)]
pub enum WriterError {
    /// XDR / envelope construction failed.
    Build(String),

    /// Transaction signing failed (ed25519 error).
    Sign(String),

    /// RPC client error (network, parsing, server response).
    Rpc(String),

    /// Asset code length is invalid (1-12 characters required).
    InvalidAssetCode(String),

    /// Horizon account fetch failed (network or non-200 status).
    AccountFetch(String),

    /// simulateTransaction RPC failed or returned a simulation error.
    Simulation(String),

    /// getTransaction polling exceeded the maximum wait duration.
    SubmissionTimeout,

    /// Transaction was included in a ledger but execution failed.
    SubmissionFailed { tx_hash: String, error: String },
}

impl std::fmt::Display for WriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriterError::Build(s) => write!(f, "build error: {s}"),
            WriterError::Sign(s) => write!(f, "sign error: {s}"),
            WriterError::Rpc(s) => write!(f, "rpc error: {s}"),
            WriterError::InvalidAssetCode(s) => write!(f, "invalid asset code: {s}"),
            WriterError::AccountFetch(s) => write!(f, "account fetch error: {s}"),
            WriterError::Simulation(s) => write!(f, "simulation error: {s}"),
            WriterError::SubmissionTimeout => write!(f, "transaction submission timed out"),
            WriterError::SubmissionFailed { tx_hash, error } => {
                write!(f, "transaction {tx_hash} failed: {error}")
            }
        }
    }
}

impl std::error::Error for WriterError {}

/// Result of a successful transaction submission and confirmation.
#[derive(Debug, Clone)]
pub struct TransactionResult {
    pub tx_hash: String,
    #[allow(dead_code)]
    pub successful: bool,
    pub ledger: u64,
    #[allow(dead_code)]
    pub diagnostic_events: Vec<String>,
}

/// Intermediate result from simulateTransaction RPC.
#[derive(Debug, Clone)]
struct SimulationResult {
    transaction_data_b64: String,
    min_resource_fee: i64,
    /// Base64-encoded SorobanAuthorizationEntry XDRs returned by simulation.
    /// Must be attached to InvokeHostFunctionOp.auth before sending.
    auth_entries_b64: Vec<String>,
}

/// Submits signed liquidity snapshots to the `LiquidityRegistry` contract.
#[derive(Debug)]
pub struct RegistryWriter {
    horizon_url: String,
    rpc_url: String,
    contract_id: String,
    network_passphrase: String,
    signing_key_hex: String,
    http: reqwest::Client,
}

impl RegistryWriter {
    /// Constructs a new RegistryWriter.
    ///
    /// # Parameters
    ///
    /// - `horizon_url`: Horizon REST endpoint for account sequence fetch
    ///   (e.g., `https://horizon-testnet.stellar.org`)
    /// - `rpc_url`: Soroban RPC endpoint (e.g.,
    ///   `https://soroban-testnet.stellar.org`)
    /// - `contract_id`: hex-encoded LiquidityRegistry contract address
    ///   (32-byte hash)
    /// - `network_passphrase`: e.g., `"Test SDF Network ; September 2015"`
    ///   for testnet
    /// - `signing_key_hex`: hex-encoded 32-byte ed25519 secret key for
    ///   the attester's Stellar account
    pub fn new(
        horizon_url: String,
        rpc_url: String,
        contract_id: String,
        network_passphrase: String,
        signing_key_hex: String,
    ) -> Self {
        Self {
            horizon_url,
            rpc_url,
            contract_id,
            network_passphrase,
            signing_key_hex,
            http: reqwest::Client::new(),
        }
    }

    /// Returns the configured Horizon URL (read-only accessor).
    pub fn horizon_url(&self) -> &str {
        &self.horizon_url
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

        // Derive attester public key from the signing secret — this is the
        // address that will be passed as the first arg to write_snapshot and
        // must match the attester whitelisted in LiquidityRegistry.
        let attester_pub_bytes = derive_public_key_bytes(&self.signing_key_hex)?;
        let attester_scval = build_account_address_scval(&attester_pub_bytes)?;
        let snapshot_scval = build_snapshot_scval(snapshot, &attester_pub_bytes)?;

        let function_name = ScSymbol::try_from("write_snapshot".as_bytes().to_vec())
            .map_err(|e| WriterError::Build(format!("function name: {e:?}")))?;

        // write_snapshot(env, attester: Address, snapshot: LiquiditySnapshot)
        let args_vec: VecM<ScVal> = vec![attester_scval, snapshot_scval]
            .try_into()
            .map_err(|e| WriterError::Build(format!("args vec: {e:?}")))?;

        Ok(InvokeContractArgs {
            contract_address,
            function_name,
            args: args_vec,
        })
    }

    /// Submits a transaction to Soroban RPC via the 9-step Phase 7.3 flow.
    ///
    /// Steps:
    ///   1. Account sequence fetch (Horizon GET /accounts/{source_account_id})
    ///   2. Build base TransactionEnvelopeV1
    ///   3. simulateTransaction → footprint + min resource fee
    ///   4. Attach SorobanTransactionData to envelope
    ///   5. Compute envelope hash (SHA256 of TransactionSignaturePayload XDR)
    ///   6. Sign hash with attester ed25519 key
    ///   7. Attach DecoratedSignature to envelope
    ///   8. sendTransaction → tx hash (status: PENDING)
    ///   9. Poll getTransaction every 1s (max 30s) until SUCCESS or FAILED
    pub async fn submit_transaction(
        &self,
        invoke_args: InvokeContractArgs,
        source_account_id: &str,
    ) -> Result<TransactionResult, WriterError> {
        // Step 1: account sequence
        let current_seq =
            fetch_account_sequence(&self.http, &self.horizon_url, source_account_id).await?;
        let next_seq = current_seq + 1;

        // Derive source public key from signing key
        let source_pub_bytes = derive_public_key_bytes(&self.signing_key_hex)?;

        // Step 2: build base envelope (100 stroops inclusion fee, no soroban data yet)
        let base_envelope = build_base_envelope(invoke_args, source_pub_bytes, next_seq, 100u32)?;
        let base_xdr_b64 = envelope_to_b64(&base_envelope)?;

        // Step 3: simulate
        let simulation = simulate_transaction(&self.http, &self.rpc_url, &base_xdr_b64).await?;

        // Decode SorobanTransactionData from simulation result
        let soroban_data = SorobanTransactionData::from_xdr_base64(
            &simulation.transaction_data_b64,
            Limits::none(),
        )
        .map_err(|e| WriterError::Simulation(format!("decode soroban data: {e:?}")))?;

        // Step 4: attach resources + update fee + apply simulation auth entries
        let final_envelope = attach_resources(
            base_envelope,
            soroban_data,
            simulation.min_resource_fee,
            &simulation.auth_entries_b64,
        )?;

        // Step 5: compute envelope hash
        let tx = match &final_envelope {
            TransactionEnvelope::Tx(v1) => &v1.tx,
            _ => return Err(WriterError::Build("expected Tx envelope".to_string())),
        };
        let hash = compute_envelope_hash(tx, &self.network_passphrase)?;

        // Step 6: sign hash
        let signature = sign_envelope_hash(&hash, &self.signing_key_hex)?;

        // Step 7: attach signature
        let signed_envelope = attach_signature(final_envelope, signature)?;
        let signed_xdr_b64 = envelope_to_b64(&signed_envelope)?;

        // Step 8: send
        let tx_hash = send_transaction(&self.http, &self.rpc_url, &signed_xdr_b64).await?;

        // Step 9: poll (max 30s)
        poll_transaction(&self.http, &self.rpc_url, &tx_hash, 30).await
    }

    /// Legacy stub: submits a pre-built envelope XDR to Soroban RPC.
    ///
    /// Retained for backward compatibility with Phase 6.5 tests. The real
    /// submission path is `submit_transaction` (Phase 7.3).
    #[allow(dead_code)]
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

        let response = self
            .http
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

// =====================================================================
// Step 1: account sequence fetch
// =====================================================================

/// Fetches the current sequence number for an account from Horizon.
///
/// Stellar transactions require the source account's current sequence + 1
/// as the transaction sequence. Horizon returns it as a decimal string.
async fn fetch_account_sequence(
    http: &reqwest::Client,
    horizon_url: &str,
    account_id: &str,
) -> Result<i64, WriterError> {
    let url = format!("{horizon_url}/accounts/{account_id}");

    let response = http
        .get(&url)
        .send()
        .await
        .map_err(|e| WriterError::AccountFetch(format!("network: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(WriterError::AccountFetch(format!(
            "status {status}: {body}"
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| WriterError::AccountFetch(format!("parse: {e}")))?;

    let seq_str = json
        .get("sequence")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WriterError::AccountFetch("missing sequence field".to_string()))?;

    seq_str
        .parse::<i64>()
        .map_err(|e| WriterError::AccountFetch(format!("invalid sequence: {e}")))
}

// =====================================================================
// Step 2: build base envelope
// =====================================================================

fn derive_public_key_bytes(signing_key_hex: &str) -> Result<[u8; 32], WriterError> {
    let secret =
        hex::decode(signing_key_hex).map_err(|e| WriterError::Sign(format!("hex: {e}")))?;
    let secret_array: [u8; 32] = secret
        .try_into()
        .map_err(|_| WriterError::Sign("expected 32-byte secret".to_string()))?;
    let signing_key = SigningKey::from_bytes(&secret_array);
    Ok(signing_key.verifying_key().to_bytes())
}

/// Builds a base TransactionEnvelope with the InvokeHostFunction operation.
///
/// This is the pre-simulation envelope: it has no SorobanTransactionData
/// and only the minimum base fee (100 stroops). After simulation the
/// caller must call `attach_resources` to add the Soroban resource data
/// and update the fee before signing.
fn build_base_envelope(
    invoke_args: InvokeContractArgs,
    source_pub_bytes: [u8; 32],
    next_seq: i64,
    base_fee: u32,
) -> Result<TransactionEnvelope, WriterError> {
    let source_account = MuxedAccount::Ed25519(Uint256(source_pub_bytes));

    let host_fn = HostFunction::InvokeContract(invoke_args);
    let invoke_op = InvokeHostFunctionOp {
        host_function: host_fn,
        auth: VecM::default(),
    };

    let operation = Operation {
        source_account: None,
        body: OperationBody::InvokeHostFunction(invoke_op),
    };

    let ops: VecM<Operation, 100> = vec![operation]
        .try_into()
        .map_err(|e| WriterError::Build(format!("ops vec: {e:?}")))?;

    let tx = Transaction {
        source_account,
        fee: base_fee,
        seq_num: SequenceNumber(next_seq),
        cond: Preconditions::None,
        memo: Memo::None,
        operations: ops,
        ext: TransactionExt::V0,
    };

    Ok(TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        signatures: VecM::default(),
    }))
}

fn envelope_to_b64(envelope: &TransactionEnvelope) -> Result<String, WriterError> {
    envelope
        .to_xdr_base64(Limits::none())
        .map_err(|e| WriterError::Build(format!("envelope to base64: {e:?}")))
}

// =====================================================================
// Step 3: simulate
// =====================================================================

async fn simulate_transaction(
    http: &reqwest::Client,
    rpc_url: &str,
    envelope_xdr_b64: &str,
) -> Result<SimulationResult, WriterError> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "simulateTransaction",
        "params": { "transaction": envelope_xdr_b64 }
    });

    let response = http
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| WriterError::Simulation(format!("network: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        return Err(WriterError::Simulation(format!(
            "status {status}: {body_text}"
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| WriterError::Simulation(format!("parse: {e}")))?;

    if let Some(err) = json.get("error") {
        return Err(WriterError::Simulation(format!("rpc error: {err}")));
    }

    let result = json
        .get("result")
        .ok_or_else(|| WriterError::Simulation("missing result".to_string()))?;

    // Simulation-level error (distinct from JSON-RPC transport error)
    if let Some(sim_err) = result.get("error") {
        return Err(WriterError::Simulation(format!("simulation: {sim_err}")));
    }

    let transaction_data_b64 = result
        .get("transactionData")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WriterError::Simulation("missing transactionData".to_string()))?
        .to_string();

    let min_resource_fee: i64 = result
        .get("minResourceFee")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);

    // Extract auth entries from first invocation result (if any).
    // Soroban simulation pre-populates these; we must include them verbatim
    // in InvokeHostFunctionOp.auth — without them, require_auth() traps.
    let auth_entries_b64 = result
        .get("results")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|r| r.get("auth"))
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(SimulationResult {
        transaction_data_b64,
        min_resource_fee,
        auth_entries_b64,
    })
}

// =====================================================================
// Step 4: attach resources
// =====================================================================

/// Attaches simulation-provided SorobanTransactionData to the envelope,
/// updates the fee, and wires in auth entries returned by simulation.
///
/// The auth entries come from `simulateTransaction.result.results[0].auth`
/// as base64-encoded XDR `SorobanAuthorizationEntry` values. Without them,
/// the contract's `require_auth()` call traps even when the transaction is
/// signed by the correct key.
fn attach_resources(
    envelope: TransactionEnvelope,
    soroban_data: SorobanTransactionData,
    min_resource_fee: i64,
    auth_entries_b64: &[String],
) -> Result<TransactionEnvelope, WriterError> {
    // Decode auth entries from simulation
    let auth_entries: Vec<SorobanAuthorizationEntry> = auth_entries_b64
        .iter()
        .map(|b64| {
            SorobanAuthorizationEntry::from_xdr_base64(b64, Limits::none())
                .map_err(|e| WriterError::Build(format!("decode auth entry: {e:?}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let auth_vec: VecM<SorobanAuthorizationEntry> = auth_entries
        .try_into()
        .map_err(|e| WriterError::Build(format!("auth vec: {e:?}")))?;

    match envelope {
        TransactionEnvelope::Tx(mut v1) => {
            let new_fee = (v1.tx.fee as i64).saturating_add(min_resource_fee);
            v1.tx.fee = new_fee.min(u32::MAX as i64) as u32;
            v1.tx.ext = TransactionExt::V1(soroban_data);

            // Apply auth entries to the InvokeHostFunction operation.
            // VecM doesn't implement DerefMut, so round-trip through Vec.
            let mut ops: Vec<Operation> = v1.tx.operations.into();
            if let Some(op) = ops.first_mut() {
                if let OperationBody::InvokeHostFunction(ref mut ihf) = op.body {
                    ihf.auth = auth_vec;
                }
            }
            v1.tx.operations = ops
                .try_into()
                .map_err(|e| WriterError::Build(format!("ops vec: {e:?}")))?;

            Ok(TransactionEnvelope::Tx(v1))
        }
        _ => Err(WriterError::Build("expected Tx envelope".to_string())),
    }
}

// =====================================================================
// Step 5: compute envelope hash
// =====================================================================

/// Computes the SHA256 hash of the TransactionSignaturePayload XDR.
///
/// Stellar transaction signing hash:
///   SHA256(TransactionSignaturePayload { network_id, tagged_transaction })
/// where network_id = SHA256(network_passphrase) and tagged_transaction
/// wraps the inner Transaction (not the full envelope).
fn compute_envelope_hash(
    tx: &Transaction,
    network_passphrase: &str,
) -> Result<[u8; 32], WriterError> {
    let network_id = Hash(Sha256::digest(network_passphrase.as_bytes()).into());

    let payload = TransactionSignaturePayload {
        network_id,
        tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(tx.clone()),
    };

    let xdr_bytes = payload
        .to_xdr(Limits::none())
        .map_err(|e| WriterError::Build(format!("hash xdr: {e:?}")))?;

    Ok(Sha256::digest(&xdr_bytes).into())
}

// =====================================================================
// Step 6: sign hash
// =====================================================================

/// Signs the envelope hash with the attester ed25519 key.
///
/// Returns a DecoratedSignature with:
///   hint = last 4 bytes of the public key
///   signature = ed25519 signature over the 32-byte hash
fn sign_envelope_hash(
    hash: &[u8; 32],
    signing_key_hex: &str,
) -> Result<DecoratedSignature, WriterError> {
    let secret_bytes =
        hex::decode(signing_key_hex).map_err(|e| WriterError::Sign(format!("hex: {e}")))?;

    let secret_array: [u8; 32] = secret_bytes
        .try_into()
        .map_err(|_| WriterError::Sign("expected 32-byte secret".to_string()))?;

    let signing_key = SigningKey::from_bytes(&secret_array);
    let pub_bytes = signing_key.verifying_key().to_bytes();

    let signature = signing_key.sign(hash);

    let hint = SignatureHint([pub_bytes[28], pub_bytes[29], pub_bytes[30], pub_bytes[31]]);

    let sig_arr: [u8; 64] = signature.to_bytes();
    let sig_bytesm: BytesM<64> = sig_arr
        .try_into()
        .expect("64-byte signature always fits BytesM<64>");

    Ok(DecoratedSignature {
        hint,
        signature: Signature(sig_bytesm),
    })
}

// =====================================================================
// Step 7: attach signature
// =====================================================================

fn attach_signature(
    envelope: TransactionEnvelope,
    sig: DecoratedSignature,
) -> Result<TransactionEnvelope, WriterError> {
    match envelope {
        TransactionEnvelope::Tx(mut v1) => {
            let mut sigs: Vec<DecoratedSignature> = v1.signatures.into();
            sigs.push(sig);
            v1.signatures = sigs
                .try_into()
                .map_err(|_| WriterError::Sign("too many signatures".to_string()))?;
            Ok(TransactionEnvelope::Tx(v1))
        }
        _ => Err(WriterError::Build("expected Tx envelope".to_string())),
    }
}

// =====================================================================
// Step 8: send transaction
// =====================================================================

async fn send_transaction(
    http: &reqwest::Client,
    rpc_url: &str,
    signed_envelope_xdr_b64: &str,
) -> Result<String, WriterError> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sendTransaction",
        "params": { "transaction": signed_envelope_xdr_b64 }
    });

    let response = http
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| WriterError::Rpc(format!("send network: {e}")))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| WriterError::Rpc(format!("send parse: {e}")))?;

    if let Some(error) = json.get("error") {
        return Err(WriterError::Rpc(format!("send: {error}")));
    }

    let result = json
        .get("result")
        .ok_or_else(|| WriterError::Rpc("send: missing result".to_string()))?;

    let status = result
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");

    if status != "PENDING" {
        let error_result = result
            .get("errorResult")
            .map(|v| v.to_string())
            .unwrap_or_default();
        return Err(WriterError::Rpc(format!(
            "send status {status}: {error_result}"
        )));
    }

    result
        .get("hash")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| WriterError::Rpc("send: missing hash".to_string()))
}

// =====================================================================
// Step 9: poll for confirmation
// =====================================================================

async fn poll_transaction(
    http: &reqwest::Client,
    rpc_url: &str,
    tx_hash: &str,
    max_wait_seconds: u64,
) -> Result<TransactionResult, WriterError> {
    let start = std::time::Instant::now();
    let max_duration = std::time::Duration::from_secs(max_wait_seconds);
    let poll_interval = std::time::Duration::from_secs(1);

    loop {
        if start.elapsed() >= max_duration {
            return Err(WriterError::SubmissionTimeout);
        }

        tokio::time::sleep(poll_interval).await;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": { "hash": tx_hash }
        });

        let response = match http.post(rpc_url).json(&body).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };

        let json: serde_json::Value = match response.json().await {
            Ok(j) => j,
            Err(_) => continue,
        };

        let result = match json.get("result") {
            Some(r) => r,
            None => continue,
        };

        let status = result
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN");

        match status {
            "NOT_FOUND" => continue,
            "SUCCESS" => {
                let ledger = result.get("ledger").and_then(|v| v.as_u64()).unwrap_or(0);
                return Ok(TransactionResult {
                    tx_hash: tx_hash.to_string(),
                    successful: true,
                    ledger,
                    diagnostic_events: Vec::new(),
                });
            }
            "FAILED" => {
                let error = result
                    .get("resultXdr")
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                return Err(WriterError::SubmissionFailed {
                    tx_hash: tx_hash.to_string(),
                    error,
                });
            }
            _ => continue,
        }
    }
}

// =====================================================================
// XDR helpers (build path — unchanged from Phase 6.5)
// =====================================================================

/// Decodes a Stellar base32 string (no padding, uppercase alphabet).
fn base32_decode_nopad(s: &str) -> Result<Vec<u8>, String> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits: u32 = 0;
    let mut bit_count: u32 = 0;
    let mut out = Vec::new();
    for c in s.chars() {
        let val = ALPHABET
            .iter()
            .position(|&x| x == c as u8)
            .ok_or_else(|| format!("invalid base32 char: {c}"))? as u32;
        bits = (bits << 5) | val;
        bit_count += 5;
        if bit_count >= 8 {
            bit_count -= 8;
            out.push(((bits >> bit_count) & 0xff) as u8);
        }
    }
    Ok(out)
}

/// Decodes a Stellar C-address StrKey (56-char base32) into its 32-byte hash.
///
/// C-address format: version_byte(0x10) + 32_byte_hash + 2_byte_crc16 = 35 bytes
/// encoded as 56 base32 characters. CRC not verified (malformed addresses fail
/// at the network level; valid C-addresses from env/config are trusted here).
fn strkey_decode_contract(c_addr: &str) -> Result<[u8; 32], WriterError> {
    let decoded = base32_decode_nopad(c_addr)
        .map_err(|e| WriterError::Build(format!("strkey decode: {e}")))?;
    if decoded.len() != 35 {
        return Err(WriterError::Build(format!(
            "strkey wrong length: expected 35, got {}",
            decoded.len()
        )));
    }
    // Contract address version byte = 2 << 3 = 0x10
    if decoded[0] != 0x10 {
        return Err(WriterError::Build(format!(
            "not a contract address (version byte=0x{:02x})",
            decoded[0]
        )));
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&decoded[1..33]);
    Ok(bytes)
}

fn parse_contract_address(c_addr: &str) -> Result<ScAddress, WriterError> {
    let bytes = strkey_decode_contract(c_addr.trim())?;
    Ok(ScAddress::Contract(ContractId(Hash(bytes))))
}

fn build_account_address_scval(pub_key_bytes: &[u8; 32]) -> Result<ScVal, WriterError> {
    Ok(ScVal::Address(ScAddress::Account(AccountId(
        PublicKey::PublicKeyTypeEd25519(Uint256(*pub_key_bytes)),
    ))))
}

fn build_contract_address_scval(c_addr: &str) -> Result<ScVal, WriterError> {
    let bytes = strkey_decode_contract(c_addr)?;
    Ok(ScVal::Address(ScAddress::Contract(ContractId(Hash(bytes)))))
}

/// Symbol fallback for the asset field when no SAC address is available.
/// Production callers always have a SAC; this path is unit-test only.
fn build_asset_scval(code: &str, issuer: &str) -> Result<ScVal, WriterError> {
    if code.is_empty() || code.len() > 12 {
        return Err(WriterError::InvalidAssetCode(code.to_string()));
    }

    if issuer == "native" {
        let sym = ScSymbol::try_from("Native".as_bytes().to_vec())
            .map_err(|e| WriterError::Build(format!("native symbol: {e:?}")))?;
        return Ok(ScVal::Symbol(sym));
    }

    let sym = ScSymbol::try_from(code.as_bytes().to_vec())
        .map_err(|e| WriterError::Build(format!("asset symbol: {e:?}")))?;
    Ok(ScVal::Symbol(sym))
}

/// Builds a `LiquiditySnapshot` ScVal map matching the on-chain struct layout.
///
/// Five fields in alphabetical order (Soroban ScMap key ordering requirement):
/// asset, attester, timestamp, unique_trades_1h, volume_30m_usd.
///
/// `asset` is `ScVal::Address(Contract)` when `sac_contract_id` is present,
/// falling back to `ScVal::Symbol` for unit tests that have no SAC (simulation
/// rejects the Symbol form).
fn build_snapshot_scval(
    snapshot: &AggregatedSnapshot,
    attester_pub_bytes: &[u8; 32],
) -> Result<ScVal, WriterError> {
    let i128_val = snapshot.volume_30m_usd_i128;
    let hi = (i128_val >> 64) as i64;
    let lo = i128_val as u64;

    let volume_scval = ScVal::I128(Int128Parts { hi, lo });
    let trade_count_scval = ScVal::U32(snapshot.unique_trades_1h);
    let timestamp_scval = ScVal::U64(snapshot.computed_at);

    let asset_scval = match &snapshot.sac_contract_id {
        Some(sac) => build_contract_address_scval(sac)?,
        None => build_asset_scval(&snapshot.asset_code, &snapshot.asset_issuer)?,
    };
    let attester_scval = build_account_address_scval(attester_pub_bytes)?;

    // Keys in alphabetical order — Soroban ScMap lexicographic requirement
    let entries = vec![
        make_map_entry("asset", asset_scval)?,
        make_map_entry("attester", attester_scval)?,
        make_map_entry("timestamp", timestamp_scval)?,
        make_map_entry("unique_trades_1h", trade_count_scval)?,
        make_map_entry("volume_30m_usd", volume_scval)?,
    ];

    let map_inner: VecM<ScMapEntry> = entries
        .try_into()
        .map_err(|e| WriterError::Build(format!("map: {e:?}")))?;

    Ok(ScVal::Map(Some(ScMap(map_inner))))
}

fn make_map_entry(key: &str, val: ScVal) -> Result<ScMapEntry, WriterError> {
    let key_sym = ScSymbol::try_from(key.as_bytes().to_vec())
        .map_err(|e| WriterError::Build(format!("key {key}: {e:?}")))?;
    Ok(ScMapEntry {
        key: ScVal::Symbol(key_sym),
        val,
    })
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AggregatedSnapshot;
    use mockito::Server;

    fn sample_snapshot() -> AggregatedSnapshot {
        AggregatedSnapshot {
            asset_code: "USDC".to_string(),
            asset_issuer: "GA5ZSEJ".to_string(),
            // XLM native SAC on Stellar testnet — valid C-address for XDR encoding tests
            sac_contract_id: Some(
                "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
            ),
            volume_30m_usd_i128: 230_000_000_000,
            unique_trades_1h: 25,
            computed_at: 1_715_000_000,
        }
    }

    fn sample_writer_with_urls(horizon_url: String, rpc_url: String) -> RegistryWriter {
        RegistryWriter::new(
            horizon_url,
            rpc_url,
            // Testnet LiquidityRegistry C-address (valid StrKey for parse_contract_address)
            "CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND".to_string(),
            "Test SDF Network ; September 2015".to_string(),
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60".to_string(),
        )
    }

    fn sample_writer() -> RegistryWriter {
        sample_writer_with_urls(
            "https://example.test".to_string(),
            "https://example.test".to_string(),
        )
    }

    // ===== parse_contract_address =====

    #[test]
    fn test_parse_contract_address_valid() {
        let c_addr = "CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND";
        let addr = parse_contract_address(c_addr).unwrap();
        match addr {
            ScAddress::Contract(ContractId(Hash(_bytes))) => {}
            _ => panic!("expected Contract variant"),
        }
    }

    #[test]
    fn test_parse_contract_address_invalid_strkey() {
        assert!(parse_contract_address("not-a-strkey!").is_err());
    }

    #[test]
    fn test_parse_contract_address_wrong_length() {
        // Too short to be a valid C-address (needs 56 chars)
        assert!(parse_contract_address("CSHORT").is_err());
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
        let attester_bytes = derive_public_key_bytes(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        )
        .unwrap();
        let scval = build_snapshot_scval(&snap, &attester_bytes).unwrap();
        match scval {
            ScVal::Map(Some(map)) => {
                assert_eq!(map.0.len(), 5);
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn test_build_snapshot_scval_i128_split() {
        let mut snap = sample_snapshot();
        snap.volume_30m_usd_i128 = 1;
        let attester_bytes = derive_public_key_bytes(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        )
        .unwrap();
        let scval = build_snapshot_scval(&snap, &attester_bytes).unwrap();
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

    // ===== submit_transaction_stub (Phase 6.5 backward compat) =====

    #[tokio::test]
    async fn test_submit_transaction_success() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":{"status":"PENDING","hash":"abc"}}"#)
            .create_async()
            .await;

        let writer = sample_writer_with_urls(server.url(), server.url());
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

        let writer = sample_writer_with_urls(server.url(), server.url());
        let result = writer
            .submit_transaction_stub("AAAAAg...".to_string())
            .await;
        assert!(matches!(result, Err(WriterError::Rpc(_))));
    }

    // ===== fetch_account_sequence (Step 1) =====

    #[tokio::test]
    async fn test_fetch_account_sequence_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/accounts/GAEFVT3LBRMLN5UN6WPI2RPFQTMXQRF3JMTWRJEI4CEGBR2GYSMRGCFR",
            )
            .with_status(200)
            .with_body(
                r#"{
                    "id": "GAEFVT3LBRMLN5UN6WPI2RPFQTMXQRF3JMTWRJEI4CEGBR2GYSMRGCFR",
                    "sequence": "12345678901"
                }"#,
            )
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let seq = fetch_account_sequence(
            &client,
            &server.url(),
            "GAEFVT3LBRMLN5UN6WPI2RPFQTMXQRF3JMTWRJEI4CEGBR2GYSMRGCFR",
        )
        .await
        .unwrap();

        mock.assert_async().await;
        assert_eq!(seq, 12_345_678_901);
    }

    #[tokio::test]
    async fn test_fetch_account_sequence_404_returns_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", mockito::Matcher::Regex(r"/accounts/.*".to_string()))
            .with_status(404)
            .with_body(r#"{"status":404,"detail":"Account not found"}"#)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let result = fetch_account_sequence(&client, &server.url(), "GBADADDRESS").await;

        match result {
            Err(WriterError::AccountFetch(msg)) => assert!(msg.contains("404")),
            other => panic!("expected AccountFetch error, got {other:?}"),
        }
    }

    // ===== simulate_transaction (Step 3) =====

    #[tokio::test]
    async fn test_simulate_transaction_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({
                "method": "simulateTransaction"
            })))
            .with_status(200)
            .with_body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "transactionData": "AAAAAA==",
                        "minResourceFee": "150000"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let result = simulate_transaction(&client, &server.url(), "BASE_XDR")
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(result.min_resource_fee, 150_000);
        assert_eq!(result.transaction_data_b64, "AAAAAA==");
    }

    #[tokio::test]
    async fn test_simulate_transaction_simulation_error() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "error": "invalid contract invocation"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let result = simulate_transaction(&client, &server.url(), "BAD_XDR").await;

        match result {
            Err(WriterError::Simulation(msg)) => assert!(msg.contains("simulation")),
            other => panic!("expected Simulation error, got {other:?}"),
        }
    }

    // ===== send_transaction (Step 8) =====

    #[tokio::test]
    async fn test_send_transaction_pending_returns_hash() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({
                "method": "sendTransaction"
            })))
            .with_status(200)
            .with_body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "status": "PENDING",
                        "hash": "deadbeef1234"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let hash = send_transaction(&client, &server.url(), "SIGNED_XDR")
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(hash, "deadbeef1234");
    }

    #[tokio::test]
    async fn test_send_transaction_error_status() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "status": "ERROR",
                        "errorResult": "AAAA"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let result = send_transaction(&client, &server.url(), "BAD_XDR").await;

        match result {
            Err(WriterError::Rpc(msg)) => assert!(msg.contains("ERROR")),
            other => panic!("expected Rpc error, got {other:?}"),
        }
    }

    // ===== poll_transaction (Step 9) =====

    #[tokio::test]
    async fn test_poll_transaction_success() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({
                "method": "getTransaction"
            })))
            .with_status(200)
            .with_body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "status": "SUCCESS",
                        "ledger": 9876
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let result = poll_transaction(&client, &server.url(), "abc123", 5)
            .await
            .unwrap();

        assert_eq!(result.tx_hash, "abc123");
        assert!(result.successful);
        assert_eq!(result.ledger, 9876);
    }

    #[tokio::test]
    async fn test_poll_transaction_failed_status() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "status": "FAILED",
                        "resultXdr": "AAAB"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let result = poll_transaction(&client, &server.url(), "failtx", 5).await;

        match result {
            Err(WriterError::SubmissionFailed { tx_hash, error }) => {
                assert_eq!(tx_hash, "failtx");
                assert!(!error.is_empty());
            }
            other => panic!("expected SubmissionFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_poll_transaction_timeout() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": { "status": "NOT_FOUND" }
                }"#,
            )
            .expect_at_least(1)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let result = poll_transaction(&client, &server.url(), "pendingtx", 1).await;

        assert!(
            matches!(result, Err(WriterError::SubmissionTimeout)),
            "expected SubmissionTimeout, got {result:?}"
        );
    }

    // ===== sign_envelope_hash (Step 6) =====

    #[test]
    fn test_sign_envelope_hash_deterministic() {
        let hash = [0u8; 32];
        let key_hex = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

        let sig1 = sign_envelope_hash(&hash, key_hex).unwrap();
        let sig2 = sign_envelope_hash(&hash, key_hex).unwrap();

        // ed25519 is deterministic — same key + same message → same signature
        assert_eq!(sig1.signature.0.as_slice(), sig2.signature.0.as_slice());
        assert_eq!(sig1.hint.0, sig2.hint.0);
    }

    #[test]
    fn test_sign_envelope_hash_hint_is_last_4_bytes_of_pubkey() {
        let hash = [0u8; 32];
        let key_hex = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        let sig = sign_envelope_hash(&hash, key_hex).unwrap();

        // Verify hint = last 4 bytes of the corresponding public key
        let pub_bytes = derive_public_key_bytes(key_hex).unwrap();
        assert_eq!(
            sig.hint.0,
            [pub_bytes[28], pub_bytes[29], pub_bytes[30], pub_bytes[31]]
        );
    }

    // ===== build_base_envelope (Step 2) =====

    #[test]
    fn test_build_base_envelope_produces_tx_envelope() {
        let snap = sample_snapshot();
        let writer = sample_writer();
        let args = writer.build_invoke_args(&snap).unwrap();
        let pub_bytes = derive_public_key_bytes(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        )
        .unwrap();

        let envelope = build_base_envelope(args, pub_bytes, 1001, 100).unwrap();
        match &envelope {
            TransactionEnvelope::Tx(v1) => {
                assert_eq!(v1.tx.fee, 100);
                assert_eq!(v1.tx.seq_num.0, 1001);
                assert_eq!(v1.tx.operations.len(), 1);
                assert!(v1.signatures.is_empty());
            }
            _ => panic!("expected Tx envelope"),
        }
    }

    // ===== Subprocess absence guard (mottomuz enforcement) =====

    #[test]
    fn test_no_subprocess_invocation() {
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
