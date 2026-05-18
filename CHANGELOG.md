# Changelog

All notable changes to safe-oracle are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2026-05-18

### Added

- **SlackSink** — Slack incoming webhook sink with Block Kit formatting.
  Configurable via `ORACLE_WATCH_SLACK_WEBHOOK_URL`.
- **PagerDutySink** — PagerDuty Events API v2 sink with dedup-key for
  alert storm prevention. Configurable via `ORACLE_WATCH_PAGERDUTY_INTEGRATION_KEY`.
- **GenericWebhookSink** — POST JSON to any endpoint with optional
  custom HTTP headers. Configurable via `ORACLE_WATCH_GENERIC_WEBHOOK_URL`
  and `ORACLE_WATCH_GENERIC_WEBHOOK_HEADERS`.
- **Stale Oracle scenario** — End-to-end testnet validation of the
  Layer 1 staleness guard. Reflector price set with 48h-old timestamp,
  borrow rejected with `StaleData` (Error #2). See `deployment/testnet.json`
  `e2e_validations.stale_oracle`. Live tx: `7b799e02...21f9`.
- **DEPLOYMENT.md operator section** — Setup steps for Slack, PagerDuty,
  and Generic webhook sinks.

### Changed

- **Doc strings (AR.H I2 closure)** — Updated "library is stateless"
  references to accurately reflect that the library maintains per-asset
  circuit breaker halt state in the caller's instance storage. Locations:
  `crates/safe-oracle/src/lib.rs` (`SafeOracleConfig` doc) and
  `crates/safe-oracle/src/circuit_breaker.rs` (authorization doc).
- **liquidity-registry test (AR.H L4 closure)** — The
  `test_initialize_rejects_unauthorized_signer` test now uses
  `#[should_panic(expected = "HostError: Error(Context, InvalidAction)")]`
  with the stable Soroban error code, instead of bare `#[should_panic]`.
  More precise failure-mode verification while remaining resilient to
  SDK message-format changes.

### Removed

- Public web site "Adversarial Review" section (the AR.H findings remain
  documented as inline source doc-comments and in commit history;
  removed from the marketing site at the reviewers' request).
- Internal/transient documentation files trimmed from the repository
  (phase notes, pre-phase discoveries, debt/research inventories, agent
  guidance). Public deliverables (READMEs, `DEPLOYMENT.md`, `LICENSE`)
  retained.

### Fixed

- No bug fixes — v0.1.0 had no known bugs at release.

### Security

- No security vulnerabilities. `cargo audit` reports 0 vulnerabilities
  across 339 dependencies. Two unmaintained transitive crates flagged
  (`derivative`, `paste`) — both via `soroban-sdk` and upstream-uncontrolled.

### Test Coverage

- 290 → **310 tests** passing (5 ignored).
- New tests cover the three new sinks (Slack, PagerDuty, Generic) with
  mockito-based HTTP testing.

## [0.1.0] — 2026-05-09

### Initial Release

- **Five guardrails** for oracle integration safety:
  - Layer 1: deviation check, staleness check, cross-source verification
  - Layer 2: liquidity check, thin sampling check
  - Layer 2.5: circuit breaker
- **`lastprice()` API** — Drop-in replacement for direct Reflector calls.
- **`SafeOracleConfig`** — Per-integrator tunable thresholds with
  mainnet-ready defaults.
- **`OracleSafetyViolation` enum** — 10 granular error variants for
  precise failure-mode handling.
- **`liquidity-registry` contract** — On-chain LiquidityRegistry for
  Layer 2 market structure checks.
- **`oracle-watch` daemon** — Off-chain anomaly detection service with
  pluggable webhook sinks (Discord, Telegram at launch).
- **`mock-reflector` + `mock-lending`** — Reference contracts for
  testnet validation.
- **Live testnet deployment** — 3 contracts deployed on Stellar testnet
  with 25+ public transaction hashes documenting successful and
  adversarial flows.
- **AR.H adversarial review** — Independent review with 0 critical,
  0 high findings. 3 medium, 5 low, 10 info — closed or accepted.

### Acknowledgments

This library is a direct response to the YieldBlox/Blend incident of
22 February 2026, in which a $5 trade against a thin SDEX market
inflated collateral valuation and drained $10.2 million from a Stellar
lending protocol. Reflector worked correctly. Stellar worked correctly.
The gap was integrator-side. safe-oracle closes that gap.

[0.2.0]: https://github.com/Sahveli01/soroban-oracle-safety/releases/tag/v0.2.0
[0.1.0]: https://github.com/Sahveli01/soroban-oracle-safety/releases/tag/phase-8-complete
