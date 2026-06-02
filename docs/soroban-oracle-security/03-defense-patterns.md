# 3. Defense Patterns

Five guardrails, composed by `lastprice()`
(`crates/safe-oracle/src/lib.rs:651`) and wrapped by the optional circuit
breaker. Layer 1 is always active; Layer 2 is opt-in (`layer2_enabled`, default
`false`). Every signature below is the **actual** code — not a sketch.

Defaults are defined once in `impl Default for SafeOracleConfig`
(`crates/safe-oracle/src/lib.rs:360`).

---

## Deviation Check — Layer 1

**Catches:** sudden price spikes between consecutive reads (Vector 1).

**Implementation:** `crates/safe-oracle/src/lib.rs:847`

```rust
fn check_deviation_from_pair(
    current: &PriceData,
    previous: &PriceData,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation>
```

Compares the newest Reflector price against its predecessor (one resolution
window earlier, ~5 min) and rejects when the BPS deviation exceeds
`config.max_deviation_bps`. Non-positive prices are treated as deviation; the
`abs_diff * 10_000` multiply is `checked_mul` (overflow → reject, fail-safe).

**Default:** `max_deviation_bps = 2000` (20%).

**Empirical justification:** rejected YieldBlox ($10.2M, ~1,050,000 bps), Mango
($114M, ~229,000 bps), and BonqDAO ($120M, conservative 5,000× → far over
threshold). See [Attack Anatomy](./02-attack-anatomy.md).

**Tuning:** tighten for stablecoin pairs (100–500 bps); loosen for volatile
small-caps (3000–5000 bps). See [Threshold Calibration](./05-threshold-calibration.md).

---

## Staleness Check — Layer 1

**Catches:** outdated feeds used to value collateral (Vector 2).

**Implementation:** `crates/safe-oracle/src/lib.rs:885`

```rust
fn check_staleness(
    env: &Env,
    current: &PriceData,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation>
```

Rejects when `now - current.timestamp > max_staleness_seconds`, and rejects
future-dated prices (`timestamp > now`) as clock-skew/manipulation. A **separate**
gate, `check_previous_staleness` (`crates/safe-oracle/src/lib.rs:1075`), bounds
the *previous* price used by the deviation check against
`previous_max_staleness_seconds`, so a post-gap recovery surfaces as `StaleData`
rather than a false `ExcessiveDeviation`.

**Defaults:** `max_staleness_seconds = 300` (current), `previous_max_staleness_seconds = 900`.

**Tuning:** match to the asset's update cadence and your tolerance for a paused
feed — never above 86,400s (validated upper bound).

---

## Cross-Source Check — Layer 1 (opt-in)

**Catches:** an attack that shifts only one feed while an independent feed
disagrees (Vector 3).

**Implementation:** `crates/safe-oracle/src/lib.rs:948`

```rust
fn check_cross_source(
    env: &Env,
    primary: &Address,
    asset: &Asset,
    current: &PriceData,
    config: &SafeOracleConfig,
    primary_decimals: u32,
) -> Result<(), OracleSafetyViolation>
```

Active only when `config.secondary_oracle = Some(addr)`. Fetches the secondary
price and rejects disagreement beyond `max_cross_source_bps`. Skips silently when
the secondary returns `None`, is stale, or traps (a broken secondary must not
freeze an otherwise-healthy primary). Decimals are reconciled first
(`DecimalsMismatch` on disagreement) so mismatched precision is a distinct,
recoverable error rather than an always-fires mismatch.

**Default:** `max_cross_source_bps = 500` (5%), opt-in.

---

## Liquidity Check — Layer 2 (opt-in)

**Catches:** a drained order book even when the price looks clean (Vector 4).

**Implementation:** `crates/safe-oracle/src/lib.rs:1173`

```rust
fn check_liquidity(
    snapshot: &LiquiditySnapshot,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation>
```

Rejects when the attested 30-minute SDEX volume `snapshot.volume_30m_usd` is below
`config.min_liquidity_usd`. The snapshot is fetched and freshness-validated once
by `get_validated_snapshot` (`crates/safe-oracle/src/lib.rs:1115`), which also
fail-safes a missing snapshot to `InsufficientLiquidity` (no evidence of
liquidity = evidence of absence).

**Default:** `min_liquidity_usd = 100_000_000_000` (= $10,000 at 7-decimal USD).

**Trust note:** Layer 2 reads attested snapshots, introducing the off-chain
attester pipeline as a second trust vector — hence opt-in. When `layer2_enabled =
false`, the registry is never queried.

---

## Thin-Sampling Check — Layer 2 (opt-in)

**Catches:** "VWAP-of-one" — a single trader dominating the window even at normal
volume (Vector 5).

**Implementation:** `crates/safe-oracle/src/lib.rs:1201`

```rust
fn check_thin_sampling(
    snapshot: &LiquiditySnapshot,
    config: &SafeOracleConfig,
) -> Result<(), OracleSafetyViolation>
```

Rejects when `snapshot.unique_trades_1h < config.min_trade_count_1h`. Independent
of the dollar-volume check — a market can clear volume and still be single-trader
dominated.

**Default:** `min_trade_count_1h = 5`.

---

## Wrapper: Circuit Breaker (opt-in)

**Implementation:** `crates/safe-oracle/src/circuit_breaker.rs`

```rust
pub fn check_circuit_breaker(env: &Env, asset: &Asset) -> Result<(), OracleSafetyViolation>  // :116
pub fn open_circuit_breaker(env: &Env, asset: &Asset, halt_duration_ledgers: u32)            // :177
pub fn close_circuit_breaker(env: &Env, asset: &Asset)                                       // :227
```

When `circuit_breaker_enabled = true`, any guardrail violation auto-halts the
asset for `circuit_breaker_halt_ledgers` (default 720 ≈ 1 hour at 5s close time);
a pre-flight check short-circuits a halted asset before any cross-contract call.
Governance can force-close via `close_circuit_breaker`. The halt write commits
because `lastprice` returns `PriceResult::Err` *inside* an `Ok` at the Soroban
boundary (`crates/safe-oracle/src/lib.rs:211`) — a `Result::Err` would roll back
the halt write.

---

## Composition order

`lastprice_inner` (`crates/safe-oracle/src/lib.rs:692`) runs the chain in this
order: fetch 2 prices → primary decimals → previous-staleness → deviation →
staleness → cross-source → (Layer 2) liquidity → thin sampling. The first
violation short-circuits, so the returned `OracleSafetyViolation` tells the
integrator exactly which check tripped.

→ Next: [Integration Guide](./04-integration-guide.md)
