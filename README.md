# soroban-oracle-safety

Oracle integration safety layer for Stellar Soroban — protects lending and DeFi protocols against thin-liquidity oracle manipulation attacks.

## Background

On 22 February 2026, an attacker drained ~$10.2M from YieldBlox's Blend lending pool on Stellar with a single $5 trade. The attack exploited a thin-liquidity oracle manipulation vector: low-volume SDEX market → manipulated VWAP → inflated collateral valuation → over-borrowing.

Reflector worked correctly. Stellar protocol worked correctly. Blend V2 worked correctly. The gap was integrator-side: no deviation guard, no liquidity threshold, no thin-sampling check.

This project closes that gap.

## Components

- **`safe-oracle`** — Soroban library wrapping Reflector with five guardrails (deviation, staleness, multi-source, liquidity, thin-sampling)
- **`liquidity-registry`** — On-chain attestation contract storing SDEX liquidity snapshots signed by oracle-watch
- **`oracle-watch`** — Off-chain monitor that signs liquidity snapshots + emits anomaly alerts
- **`SEP-Oracle-Safety`** — Standardization proposal (planned)

## Status

Work in progress. Phase 1 (foundation) complete:

- Workspace scaffolding (6 crates)
- CI pipeline (GitHub Actions)
- Mock Reflector + mock Lending contracts
- Core type definitions (OracleSafetyViolation, SafeOracleConfig, Asset)
- Test infrastructure (test-utils crate)
- 13 unit tests, 0 warnings

Phase 2 (safe_oracle Layer 1 guardrails) starting next.

## Building

Requires Rust stable + MSVC toolchain (Windows) or stable (Linux/macOS).

```bash
cargo build --workspace
cargo test --workspace
```

## License

Apache License 2.0. See [LICENSE](./LICENSE).

## Author

[@Sahveli01](https://github.com/Sahveli01)
