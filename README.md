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

Work in progress. Phase 3 (LiquidityRegistry contract) complete:

- Workspace scaffolding (6 crates) — Phase 1
- CI pipeline (GitHub Actions) — Phase 1
- Mock Reflector + mock Lending contracts (mock-lending integrated with real `safe_oracle::lastprice`)
- Core type definitions (`OracleSafetyViolation`, `SafeOracleConfig`, `Asset`, `PriceData`, `LiquiditySnapshot`)
- Test infrastructure (`test-utils` crate, primary + secondary mock Reflectors, LiquidityRegistry integration)
- **Layer 1 guardrails** (real implementations) — Phase 2:
  - `check_deviation` — BPS-based, blocks YieldBlox-class SDEX manipulation
  - `check_staleness` — Unix timestamp comparison via `env.ledger().timestamp()`
  - `check_cross_source` — opt-in secondary oracle cross-check
- **LiquidityRegistry contract** — Phase 3:
  - `initialize` with reinitialization protection
  - Whitelist management (`add_attester`, `remove_attester`, `is_attester`)
  - `write_snapshot` with 5-step validation, replay protection (strict greater timestamp)
  - `get_snapshot` read function (Option return, freshness deferred to consumers)
  - `safe_oracle ↔ LiquidityRegistry` cross-contract binding (`#[contractclient]` pattern)
  - 8 error variants, 3 events (AttesterAdded, AttesterRemoved, SnapshotWritten)
- 68 tests passing, 0 warnings

Phase 4 (Layer 2 guardrails — `check_liquidity`, `check_thin_sampling`) starting next.

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
