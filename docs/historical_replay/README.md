# Historical Exploit Replay — safe-oracle Empirical Validation

`safe-oracle`'s **default** thresholds, replayed against real DeFi exploits.

This corpus exists to answer one critique directly: *"the defaults are just
intuition."* They are not. Each test below loads a real attack's price series
from a cited public source and runs it through the live `safe_oracle::lastprice()`
path (`crates/safe-oracle/tests/historical_replay/`), then records whether the
**default config** (trustless Layer 1, zero opt-in) would have rejected the feed.

## Results

| Attack | Date | Reported loss | Chain | Oracle attack? | Default (Layer 1) outcome | Guardrail |
|--------|------|---------------|-------|----------------|---------------------------|-----------|
| **YieldBlox** | 2026-02-22 | ~$10.2M | Stellar (native) | Yes | ✓ **REJECTED** | `ExcessiveDeviation` |
| **Mango Markets** | 2022-10-11 | ~$114M | Solana (adapted) | Yes | ✓ **REJECTED** | `ExcessiveDeviation` |
| **BonqDAO** | 2023-02-02 | ~$120M | Polygon (adapted) | Yes | ✓ **REJECTED** | `ExcessiveDeviation` |
| **Euler Finance** | 2023-03-13 | ~$197M | Ethereum (adapted) | **No** | — **OUT OF SCOPE** | None — not an oracle attack |

### Honest scorecard

- **Oracle-manipulation attacks in this corpus: 3 / 3 rejected by the default config.**
  YieldBlox + Mango + BonqDAO = **~$244.2M** in oracle-manipulation losses, all
  caught by Layer 1 deviation with **no opt-in** (Layer 2 disabled by default).
- **Euler ($197M) is deliberately NOT counted as prevented.** It was the largest
  hack of 2023, but it was *not* an oracle-price manipulation — it exploited a
  missing solvency check in `donateToReserves()`. We include it as a **negative
  control**: safe-oracle correctly does *not* false-positive on Euler's honest
  feed, and it could not have prevented that attack. A price validator does not
  defend against collateral-accounting bugs, and we will not pretend otherwise.

> We do **not** claim "$441M prevented." The corpus spans ~$441M of headline
> losses; only the **~$244M that was actually oracle manipulation** is in scope,
> and the default thresholds reject all of it. That distinction is the point.

## What "default config" means here

`SafeOracleConfig::default()` is the trustless mode:

- **Active:** deviation (2000 BPS / 20%), staleness (300s), previous-price
  staleness (900s), Reflector decimals check. Cross-source is opt-in (no
  secondary configured by default).
- **Disabled:** Layer 2 (liquidity threshold + thin sampling) — `layer2_enabled
  = false`. The off-chain attester pipeline is never queried.

So every ✓ REJECTED above is achieved with **pure on-chain Reflector math** — no
attester, no secondary oracle, no circuit breaker. That is the strongest form of
the claim.

## Layer 2 is the answer to the *sophisticated* variant

The single-shot historical attacks all produced enormous single-step price moves
(100x, 13x, orders of magnitude) because their victims had *no* deviation guard —
the attackers never needed to be subtle. An attacker facing safe-oracle's 20%
deviation guardrail would instead go **sub-threshold**: nudge the price a few
percent against a drained order book. Layer 1 alone misses that; the opt-in
Layer 2 liquidity guardrail catches it. Two tests demonstrate this explicitly
(`replay_yieldblox_subthreshold_needs_layer2`,
`replay_bonqdao_thin_market_layer2_backstop`) — and they are honest that the
*default* would let the sub-threshold move through.

## Honest disclosures

1. **Non-Stellar attacks are adapted.** Mango (Solana), BonqDAO (Polygon), and
   Euler (Ethereum) did not happen on Soroban. We replay their **real, sourced
   price magnitudes** through Soroban's `PriceData` / `Asset` types. The
   `ExcessiveDeviation` guardrail compares consecutive oracle ticks and is
   chain-agnostic, so the adaptation does not change the math — only the
   transport. Each JSON file carries `adapted: true`.
2. **Timestamps are synthetic.** The test harness runs on a synthetic ledger
   clock (`now = 100_000`), so real-world unix timestamps (which would read as
   decades-stale) are not used. The deviation guardrail is time-invariant, so the
   harness assigns in-window ledger timestamps while preserving the real price
   values and ordering. Real attack dates live in each JSON `date` field.
3. **BonqDAO's manipulated price is a conservative stand-in.** The pre-attack
   WALBT price (~$0.05) is real; the exact on-chain `submitValue` figure is not
   cleanly published, so we use a conservative 5,000x lower bound (the real
   inflation was far larger). Any move >20% trips the guardrail, so the verdict
   is robust to the exact number.
4. **This is replay analysis, not live prevention.** Real-world conditions,
   ledger timing, and integrator wiring differ. These tests prove the *guardrail
   math* rejects the *observed price shapes* — they do not simulate the full
   on-chain attack transaction.

## Per-attack analysis

- [YieldBlox (2026) — Stellar native](./yieldblox_analysis.md)
- [Mango Markets (2022) — adapted](./mango_analysis.md)
- [BonqDAO (2023) — adapted](./bonqdao_analysis.md)
- [Euler Finance (2023) — out of scope](./euler_analysis.md)

## Reproduce

```bash
cargo test -p safe-oracle --test historical_replay
```

Evidence files (real prices + source URLs) live in
[`crates/safe-oracle/tests/historical_replay/data/`](../../crates/safe-oracle/tests/historical_replay/data/).
