//! Ed25519 keypair management and snapshot signing.
//!
//! Wraps `ed25519-dalek` for off-chain attestation signing.
//! Implemented in Phase 6.4.
//!
//! Note: LiquidityRegistry currently uses Stellar `require_auth()` for
//! attester verification (Phase 3.3); off-chain ed25519 signature is
//! kept for spec compliance and forward compatibility but is not
//! verified on-chain in the current contract.

// TODO Phase 6.4: Signer struct (SigningKey + VerifyingKey)
// TODO Phase 6.4: from_hex_secret() loader
// TODO Phase 6.4: sign_snapshot() returning [u8; 64]
// TODO Phase 6.4: SignerError enum
