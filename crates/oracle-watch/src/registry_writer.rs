//! Submits signed liquidity snapshots to the on-chain `LiquidityRegistry`.
//!
//! Uses `stellar-rpc-client` (Soroban RPC) and `stellar-xdr` for transaction
//! construction. **Subprocess invocation (e.g., `stellar contract invoke`)
//! is forbidden** — Rust-native SDK only.
//!
//! Implemented in Phase 6.5.

// TODO Phase 6.5: RegistryWriter struct (rpc_client, signing_keypair, contract_id)
// TODO Phase 6.5: write_snapshot() — build XDR, sign tx, submit via stellar-rpc-client
// TODO Phase 6.5: WriterError enum (rpc, sign, build)
