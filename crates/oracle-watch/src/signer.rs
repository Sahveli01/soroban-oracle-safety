//! Ed25519 keypair management and snapshot signing.
//!
//! Wraps `ed25519-dalek` for off-chain attestation signing. See module
//! `types::LiquiditySnapshotPayload` doc-comment for the design rationale
//! around why this exists despite being unused on-chain in the current
//! `LiquidityRegistry` contract.

use crate::types::LiquiditySnapshotPayload;
use ed25519_dalek::{Signer as _, SigningKey, VerifyingKey};

/// Errors that can occur during signer operations.
#[derive(Debug)]
pub enum SignerError {
    /// Hex string had wrong length or non-hex characters.
    InvalidHex(String),

    /// Hex parsed but bytes wrong length (must be exactly 32).
    InvalidKeyLength { expected: usize, got: usize },

    /// Payload byte-serialization failed (string too long).
    PayloadSerialization,
}

impl std::fmt::Display for SignerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignerError::InvalidHex(s) => write!(f, "invalid hex: {s}"),
            SignerError::InvalidKeyLength { expected, got } => {
                write!(
                    f,
                    "invalid key length: expected {expected} bytes, got {got}"
                )
            }
            SignerError::PayloadSerialization => write!(f, "payload too long to serialize"),
        }
    }
}

impl std::error::Error for SignerError {}

/// Signs liquidity snapshots with an ed25519 keypair.
///
/// Loaded from a hex-encoded 32-byte secret key (typically read from
/// the `ORACLE_WATCH_SIGNING_SECRET_KEY` environment variable in
/// `Config::from_env`).
///
/// # Security note
///
/// The signing key lives in process memory for the lifetime of the
/// service. There is no key rotation in this design (Phase 6 scope).
/// Phase 8 deployment must handle key provisioning carefully — env-var
/// loading is convenient but exposes the key to anyone with read access
/// to the process environment (`/proc/self/environ`, container inspect
/// commands, etc.). HSM/KMS integration is post-Phase-8 work if SCF
/// deployment scales beyond a single attester.
#[derive(Debug)]
pub struct Signer {
    keypair: SigningKey,
}

impl Signer {
    /// Constructs a Signer from a hex-encoded 32-byte secret key.
    ///
    /// # Errors
    ///
    /// - `InvalidHex` if the input is not valid hex
    /// - `InvalidKeyLength` if the decoded bytes are not exactly 32
    pub fn from_hex_secret(hex_key: &str) -> Result<Self, SignerError> {
        let bytes =
            hex::decode(hex_key.trim()).map_err(|e| SignerError::InvalidHex(e.to_string()))?;

        if bytes.len() != 32 {
            return Err(SignerError::InvalidKeyLength {
                expected: 32,
                got: bytes.len(),
            });
        }

        let array: [u8; 32] = bytes.try_into().expect("length checked above");
        let keypair = SigningKey::from_bytes(&array);

        Ok(Signer { keypair })
    }

    /// Signs a `LiquiditySnapshotPayload`, returning the 64-byte signature.
    ///
    /// Internally calls `payload.to_signing_bytes()` for the canonical
    /// byte form, then signs with ed25519. See payload's doc-comment for
    /// the byte format.
    ///
    /// # Errors
    ///
    /// `PayloadSerialization` if either asset string exceeds 255 bytes
    /// (defensive; production assets are far below this cap).
    pub fn sign_snapshot(
        &self,
        payload: &LiquiditySnapshotPayload,
    ) -> Result<[u8; 64], SignerError> {
        let bytes = payload
            .to_signing_bytes()
            .ok_or(SignerError::PayloadSerialization)?;

        let sig = self.keypair.sign(&bytes);
        Ok(sig.to_bytes())
    }

    /// Returns the ed25519 public key as 32 raw bytes.
    ///
    /// Used by downstream consumers (Phase 6.5 registry_writer or future
    /// on-chain verification) to identify the signing attester.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.keypair.verifying_key().to_bytes()
    }

    /// Returns the verifying key for direct use with ed25519-dalek's
    /// `Verifier` trait (e.g., in tests).
    ///
    /// Test-only alternative to [`Signer::public_key_bytes`] — main loop
    /// uses the byte form for logging/registration; verification flows
    /// (currently in unit tests, Phase 8 cross-verification) use this.
    #[allow(dead_code)]
    pub fn verifying_key(&self) -> VerifyingKey {
        self.keypair.verifying_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;

    /// Deterministic test secret key (32 bytes = 64 hex chars).
    /// Hard-coded for reproducibility — real deployments use env-var loading.
    const TEST_HEX_KEY: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

    #[test]
    fn test_from_hex_secret_valid_key() {
        let signer = Signer::from_hex_secret(TEST_HEX_KEY).unwrap();
        // Public key derived deterministically from this seed (RFC 8032 test vector)
        let pk = signer.public_key_bytes();
        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn test_from_hex_secret_handles_whitespace() {
        let key_with_whitespace = format!("  {TEST_HEX_KEY}\n");
        let signer = Signer::from_hex_secret(&key_with_whitespace);
        assert!(signer.is_ok());
    }

    #[test]
    fn test_from_hex_secret_invalid_hex() {
        let result = Signer::from_hex_secret("not-hex-zzzzz");
        assert!(matches!(result, Err(SignerError::InvalidHex(_))));
    }

    #[test]
    fn test_from_hex_secret_wrong_length() {
        // 16 bytes (32 hex chars), too short
        let result = Signer::from_hex_secret("00112233445566778899aabbccddeeff");
        match result {
            Err(SignerError::InvalidKeyLength { expected, got }) => {
                assert_eq!(expected, 32);
                assert_eq!(got, 16);
            }
            other => panic!("expected InvalidKeyLength, got {other:?}"),
        }
    }

    fn sample_payload() -> LiquiditySnapshotPayload {
        LiquiditySnapshotPayload {
            asset_code: "USDC".to_string(),
            asset_issuer: "GA5ZSEJ".to_string(),
            volume_30m_usd_i128: 1_000_000_000,
            unique_trades_1h: 25,
            timestamp: 1_715_000_000,
        }
    }

    #[test]
    fn test_sign_snapshot_roundtrip() {
        // Sign → verify with same key → must succeed
        let signer = Signer::from_hex_secret(TEST_HEX_KEY).unwrap();
        let payload = sample_payload();
        let sig_bytes = signer.sign_snapshot(&payload).unwrap();
        assert_eq!(sig_bytes.len(), 64);

        let bytes_to_verify = payload.to_signing_bytes().unwrap();
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        let verifying_key = signer.verifying_key();
        verifying_key.verify(&bytes_to_verify, &signature).unwrap();
    }

    #[test]
    fn test_sign_snapshot_deterministic() {
        // Ed25519 is deterministic — same key + same message → same signature
        let signer = Signer::from_hex_secret(TEST_HEX_KEY).unwrap();
        let payload = sample_payload();
        let sig1 = signer.sign_snapshot(&payload).unwrap();
        let sig2 = signer.sign_snapshot(&payload).unwrap();
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_sign_snapshot_different_payload_different_sig() {
        let signer = Signer::from_hex_secret(TEST_HEX_KEY).unwrap();
        let p1 = sample_payload();
        let mut p2 = sample_payload();
        p2.unique_trades_1h = 26; // change one field
        let sig1 = signer.sign_snapshot(&p1).unwrap();
        let sig2 = signer.sign_snapshot(&p2).unwrap();
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_sign_verify_with_wrong_key_fails() {
        // Different keys produce different signatures, verification fails
        let signer1 = Signer::from_hex_secret(TEST_HEX_KEY).unwrap();
        let signer2_hex = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        let signer2 = Signer::from_hex_secret(signer2_hex).unwrap();

        let payload = sample_payload();
        let sig1 = signer1.sign_snapshot(&payload).unwrap();
        let bytes_to_verify = payload.to_signing_bytes().unwrap();
        let signature = ed25519_dalek::Signature::from_bytes(&sig1);

        let result = signer2.verifying_key().verify(&bytes_to_verify, &signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_verify_tampered_payload_fails() {
        let signer = Signer::from_hex_secret(TEST_HEX_KEY).unwrap();
        let payload = sample_payload();
        let sig_bytes = signer.sign_snapshot(&payload).unwrap();

        let mut tampered_bytes = payload.to_signing_bytes().unwrap();
        tampered_bytes[10] ^= 0xff; // flip a byte

        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        let result = signer.verifying_key().verify(&tampered_bytes, &signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_public_key_bytes_returns_32_bytes() {
        let signer = Signer::from_hex_secret(TEST_HEX_KEY).unwrap();
        let pk = signer.public_key_bytes();
        assert_eq!(pk.len(), 32);
    }

    // ===== Payload serialization tests =====

    #[test]
    fn test_payload_to_signing_bytes_format() {
        let payload = LiquiditySnapshotPayload {
            asset_code: "USDC".to_string(),
            asset_issuer: "GA".to_string(),
            volume_30m_usd_i128: 0,
            unique_trades_1h: 0,
            timestamp: 0,
        };
        let bytes = payload.to_signing_bytes().unwrap();
        // 1 (code_len) + 4 (USDC) + 1 (issuer_len) + 2 (GA) + 16 (i128) + 4 (u32) + 8 (u64) = 36
        assert_eq!(bytes.len(), 36);
        assert_eq!(bytes[0], 4); // code length
        assert_eq!(&bytes[1..5], b"USDC");
        assert_eq!(bytes[5], 2); // issuer length
        assert_eq!(&bytes[6..8], b"GA");
    }

    #[test]
    fn test_payload_to_signing_bytes_string_too_long() {
        let payload = LiquiditySnapshotPayload {
            asset_code: "X".repeat(256), // exceeds u8::MAX
            asset_issuer: "GA".to_string(),
            volume_30m_usd_i128: 0,
            unique_trades_1h: 0,
            timestamp: 0,
        };
        assert!(payload.to_signing_bytes().is_none());
    }

    #[test]
    fn test_payload_deterministic_serialization() {
        let p1 = sample_payload();
        let p2 = sample_payload();
        assert_eq!(p1.to_signing_bytes(), p2.to_signing_bytes());
    }

    #[test]
    fn test_payload_from_aggregated_snapshot() {
        use crate::types::AggregatedSnapshot;
        let snap = AggregatedSnapshot {
            asset_code: "XLM".to_string(),
            asset_issuer: "native".to_string(),
            volume_30m_usd_i128: 5_000_000_000,
            unique_trades_1h: 12,
            computed_at: 1_715_000_000,
        };
        let payload: LiquiditySnapshotPayload = (&snap).into();
        assert_eq!(payload.asset_code, "XLM");
        assert_eq!(payload.timestamp, 1_715_000_000);
        assert_eq!(payload.volume_30m_usd_i128, 5_000_000_000);
    }
}
