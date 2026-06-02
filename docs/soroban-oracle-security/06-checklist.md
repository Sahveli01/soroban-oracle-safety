# 6. Audit Checklist

Verify before mainnet. Each item maps to a pattern in this standard; the
`file:line` references are the implementation to compare your integration against.

## Layer 1 (required)

- [ ] **Every** price read goes through `safe_oracle::lastprice(...).into_result()`
      — pattern: `mocks/mock-lending/src/lib.rs:327`.
- [ ] No bare `oracle.lastprice()` / `reflector.lastprice()` calls remain in
      critical paths (borrow, liquidate, mint). Grep for them.
- [ ] `SafeOracleConfig::validate()` is called at deploy, before any storage
      write — pattern: `mocks/mock-lending/src/lib.rs:255`.
- [ ] Your entry point returns the `Ok`-wrapped pattern (not `Result::Err`) so
      circuit-breaker / state writes commit — pattern: `BorrowOutcome`,
      `mocks/mock-lending/src/lib.rs:149`.
- [ ] Any non-default threshold is documented with a rationale
      (see [Threshold Calibration](./05-threshold-calibration.md)).
- [ ] Guardrail violations are surfaced to your caller (not swallowed) so the
      reason a borrow failed is auditable.

## Layer 2 (if `layer2_enabled = true`)

- [ ] `LiquidityRegistry` deployed and its attester whitelist verified.
- [ ] At least **2-of-N attester redundancy** — a single attester is a single
      point of failure and a trust bottleneck.
- [ ] Attester liveness + snapshot-freshness monitoring and alerting in place
      (a stale pipeline fail-safes to rejection, halting borrows).
- [ ] `max_snapshot_age_seconds` matches the asset's volatility and your
      attester cadence.
- [ ] You have tested the missing-snapshot path: it must fail-safe to
      `InsufficientLiquidity` (`crates/safe-oracle/src/lib.rs:1115`), not pass.

## Operational

- [ ] Circuit-breaker policy defined: halt window, and who/how can
      `close_circuit_breaker` (governance).
- [ ] Incident-response runbook for an oracle alert (who is paged, what they do).
- [ ] On-call rotation for oracle/attester alerts.
- [ ] **Historical replay tests pass on your custom config:**
      `cargo test -p safe-oracle --test historical_replay` — and you have
      confirmed any threshold you loosened still rejects what you intend.

## Out of scope — verify these are covered elsewhere

`safe-oracle` does not defend against these. Confirm a *different* control does
(see [Threat Model §What we don't defend against](./01-threat-model.md#what-we-do-not-defend-against)):

- [ ] Collateral-accounting / liquidation-logic solvency invariants (Euler class)
      — covered by your lending contract's own checks + audit.
- [ ] General smart-contract security (reentrancy, access control, math) — covered
      by a full audit.
- [ ] Attester signing-key and protocol-admin key management — covered by your
      key-management / governance process.

---

When every box is checked, you have a defensible answer to *"why is this price
safe to act on?"* — at every read, backed by code and by replay against $244M of
real exploits.
