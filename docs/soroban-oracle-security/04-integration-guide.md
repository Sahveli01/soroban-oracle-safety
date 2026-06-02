# 4. Integration Guide

The reference integration is `mock-lending`
(`mocks/mock-lending/src/lib.rs`) — a minimal lending contract that routes every
borrow through `safe-oracle`. Every example below cites its real `file:line`.

---

## 1. Quick Start — Layer 1 only (trustless default)

The default config is fully trustless: pure on-chain Reflector math, Layer 2
disabled, no attester, no secondary, no breaker. Replace your bare oracle read
with `lastprice(...).into_result()?`:

```rust
// mocks/mock-lending/src/lib.rs:327 — inside borrow()
let price_data =
    safe_oracle::lastprice(&env, &asset, &oracle, &registry, &config).into_result()?;
```

`into_result()` (`crates/safe-oracle/src/lib.rs:242`) re-hydrates the typed
`OracleSafetyViolation`, so a guardrail violation flows through your existing `?`
path. With `SafeOracleConfig::default()` this preserves availability in normal
operation and rejects only on a real anomaly.

> **Return-type caveat.** `lastprice` returns `PriceResult` (an `Ok`-wrapped
> enum), not `Result`, so the circuit-breaker halt write can commit at the
> Soroban boundary. Mirror this in your own entry point: `mock-lending` returns
> `BorrowOutcome` (`mocks/mock-lending/src/lib.rs:149`) with its own
> `into_result()` for callers (`:171`). A method that returns `Result::Err`
> rolls back all storage writes in the same invocation — including the halt.

---

## 2. Validate the config at deploy

Validation is opt-in at the library layer (no per-call gas cost); the integrator
opts in **once**, at construction, before persisting anything:

```rust
// mocks/mock-lending/src/lib.rs:255 — inside __constructor
config.validate().map_err(MockLendingError::from)?;
```

`validate()` (`crates/safe-oracle/src/lib.rs:532`) rejects silent-disable
misconfigurations (`max_deviation_bps = 0`, halt window of 0, etc.). Never store
an unvalidated config.

---

## 3. Adding Layer 2 (defense-in-depth)

Layer 2 catches the *sub-threshold* manipulation that Layer 1 misses (a small
price nudge against a drained book). It requires the off-chain attester pipeline,
which is a second trust vector — enable it deliberately:

1. Deploy `LiquidityRegistry` and whitelist your attester(s).
2. Run `oracle-watch` to poll SDEX trade flow and submit signed
   `LiquiditySnapshot`s.
3. Set `layer2_enabled = true` in the stored config.

With Layer 2 on, `Asset::Stellar` borrows additionally require a fresh, attested
snapshot clearing `min_liquidity_usd` and `min_trade_count_1h`
(`get_validated_snapshot`, `crates/safe-oracle/src/lib.rs:1115`). A missing
snapshot fail-safes to `InsufficientLiquidity` — a forgotten pipeline cannot
silently bypass the guardrail.

**Trade-off:** Layer 2 trades a small availability cost (depends on an attester
being live and fresh) for protection against the thin-market precondition. Run
≥2-of-N attesters and monitor freshness (see [Checklist](./06-checklist.md)).

---

## 4. Circuit Breaker (advanced, opt-in)

Set `circuit_breaker_enabled = true` and a halt window
(`circuit_breaker_halt_ledgers`, default 720 ≈ 1h). After any violation the asset
auto-halts; subsequent reads short-circuit cheaply until the window expires or
governance force-closes via `close_circuit_breaker`
(`crates/safe-oracle/src/circuit_breaker.rs:227`). Because the halt is a storage
write, your entry point **must** use the `Ok`-wrapped return pattern (§1) or the
write rolls back.

---

## 5. Per-asset configuration

Config is per-pool and immutable after deploy. Use tight thresholds for stable
pairs and looser ones for volatile assets:

```rust
let stable = SafeOracleConfig { max_deviation_bps: 300,  ..Default::default() }; // 3%
let major  = SafeOracleConfig { max_deviation_bps: 2000, ..Default::default() }; // 20% (default)
let volatile = SafeOracleConfig { max_deviation_bps: 4000, ..Default::default() }; // 40%
```

Document the rationale for any non-default threshold (it is an audit item). See
[Threshold Calibration](./05-threshold-calibration.md) for ranges.

---

## 6. Testing checklist (before mainnet)

- **Adversarial replay:** run a spike and a stale scenario against *your* config
  and assert the expected `OracleSafetyViolation`. Pattern:
  `crates/safe-oracle/tests/e2e_attack_scenarios.rs`.
- **Historical replay on your config:** the corpus in
  `crates/safe-oracle/tests/historical_replay/` should still reject with your
  thresholds (a looser deviation cap may let a smaller spike through — verify
  that is intentional).
- **Fuzz:** property tests over price/timestamp inputs
  (`crates/safe-oracle/tests/property_based.rs`).
- **Testnet dry-run:** exercise the full borrow path against a live Reflector and
  registry before mainnet.

→ Next: [Threshold Calibration](./05-threshold-calibration.md)
