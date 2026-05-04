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

Work in progress. Phase 5 (circuit breaker — auto-halt + governance override) complete:

- Workspace scaffolding (6 crates) — Phase 1
- CI pipeline (GitHub Actions) — Phase 1
- Mock Reflector + mock Lending contracts (mock-lending wired to real `safe_oracle` chain end-to-end)
- Core type definitions (`OracleSafetyViolation`, `SafeOracleConfig`, `Asset`, `PriceData`, `LiquiditySnapshot`)
- Test infrastructure (`test-utils` crate, primary + secondary mock Reflectors, real LiquidityRegistry wiring, `OracleHost` + `TestHost` harnesses)
- **Layer 1 guardrails** — Phase 2:
  - `check_deviation` — BPS-based, blocks YieldBlox-class SDEX manipulation
  - `check_staleness` — Unix timestamp comparison via `env.ledger().timestamp()`
  - `check_cross_source` — opt-in secondary oracle cross-check
- **LiquidityRegistry contract** — Phase 3:
  - `initialize` with reinitialization protection, whitelist management, `write_snapshot` (5-step validation + replay protection), `get_snapshot` read
  - `safe_oracle ↔ LiquidityRegistry` cross-contract binding (`#[contractclient]` pattern)
- **Layer 2 guardrails** — Phase 4:
  - `check_liquidity` — SDEX 30-minute volume threshold (`min_liquidity_usd`)
  - `check_thin_sampling` — unique 1-hour trade count threshold (`min_trade_count_1h`)
  - `get_validated_snapshot` helper — single cross-contract call shared by both Layer 2 checks; `Asset::Other` skip semantics; fail-safe `InsufficientLiquidity` on missing snapshot; consumer-side freshness via `max_snapshot_age_seconds`
- **End-to-end attack scenarios** — Phase 4.4 / 4.5:
  - YieldBlox classic ($5 trade, 100× spike) → blocked by Layer 1 (`ExcessiveDeviation`)
  - YieldBlox sophisticated (5% spike + thin order book) → blocked by Layer 2 (`InsufficientLiquidity`) — the unique value proposition
  - Stale Reflector, stale registry snapshot, drained order book, single-trade window
  - Demo command: `cargo test --test e2e_attack_scenarios`
  - Lending-perspective counterpart: `cargo test --test integration -p mock-lending`
- **Circuit breaker** — Phase 5:
  - Per-asset isolated halt mechanism (`CircuitBreakerState` Closed / Open with auto-recovery)
  - Auto-halt on guardrail violation, opt-in via `config.circuit_breaker_enabled` (default `false` preserves Phase 1-4 behavior)
  - Auto-recovery after `config.circuit_breaker_halt_ledgers` (default 720, ~1 hour at 5s close time)
  - Governance manual override via `close_circuit_breaker` (caller MUST gate with `require_auth`; library does not enforce)
  - Asset::Stellar + Asset::Other coverage with isolated storage paths
  - Pre-flight check short-circuits `lastprice()` for halted assets (near-zero gas to reject)
  - 26 dedicated tests across state machine, lastprice integration, lending e2e, manual override, edge cases
- **Ok-API contract types** — Phase 5.2 v2 + 5.4 v2:
  - `safe_oracle::PriceResult` and `mock_lending::BorrowOutcome` enums (Ok-only at the Soroban boundary)
  - Required because Soroban rolls back all storage writes when a contract method returns `Result::Err`; auto-halt writes need to commit
  - `into_result()` shim preserves `?` operator ergonomics for callers
  - `From<Result<...>>` impl for internal helpers producing `Result`
- 122 tests passing, 0 warnings

Demo commands:
- Attack scenarios (Phase 4): `cargo test --test e2e_attack_scenarios`
- Auto-halt verification (Phase 5.4 v2 regression): `cargo test --test integration -p mock-lending test_borrow_circuit_breaker_opens`

Phase 6 (audit + tracked debts cleanup) starting next.

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
