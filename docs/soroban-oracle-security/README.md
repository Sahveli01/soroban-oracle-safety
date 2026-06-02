# Soroban Oracle Security

A reference standard for defending Soroban DeFi protocols against oracle
manipulation attacks — with [`safe-oracle`](https://crates.io/crates/safe-oracle)
as the working reference implementation.

## Why this exists

Oracle manipulation has drained **~$244M** from DeFi protocols that this
standard is in scope to defend:

| Attack | Date | Loss | Chain |
|--------|------|------|-------|
| YieldBlox / Blend V2 | 2026-02-22 | ~$10.2M | Stellar |
| Mango Markets | 2022-10-11 | ~$114M | Solana |
| BonqDAO | 2023-02-02 | ~$120M | Polygon |

In every case the *oracle reported correctly*. The integrator acted on the
reported price **without validating it** — no deviation guard, no liquidity
threshold, no staleness ceiling. This document is the missing layer between the
oracle read and the risky action (borrow / liquidate / mint).

Every claim here is backed by two things: **real code** (`file:line` into the
reference implementation) and **empirical replay** (the historical exploits above
are replayed against the default thresholds in
[`crates/safe-oracle/tests/historical_replay/`](../../crates/safe-oracle/tests/historical_replay/)).

## Contents

1. [Threat Model](./01-threat-model.md) — the trust boundary and the five
   oracle-manipulation vectors we defend against (and what we deliberately do not).
2. [Attack Anatomy](./02-attack-anatomy.md) — how YieldBlox, Mango, BonqDAO, and
   Euler actually worked, and which guardrail catches each.
3. [Defense Patterns](./03-defense-patterns.md) — the five guardrails, each with
   its real signature, `file:line`, default threshold, and empirical justification.
4. [Integration Guide](./04-integration-guide.md) — Layer 1 quick start, Layer 2
   opt-in, circuit breaker, per-asset configuration, testing.
5. [Threshold Calibration](./05-threshold-calibration.md) — why the defaults are
   what they are (empirical), and how to tune per asset class (engineering
   guidance, clearly labelled).
6. [Audit Checklist](./06-checklist.md) — what to verify before mainnet.

## Reference implementation

`safe-oracle` implements every pattern in this document. It is published on
[crates.io](https://crates.io/crates/safe-oracle) and live on Stellar testnet.
Each best practice below links to the actual line of code that implements it, so
this standard cannot drift from a working, tested implementation.

## Empirical validation (scope-honest)

Default thresholds replayed against the corpus above:

- YieldBlox (2026): ✓ caught — `ExcessiveDeviation`
- Mango (2022): ✓ caught — `ExcessiveDeviation`
- BonqDAO (2023): ✓ caught — `ExcessiveDeviation`
- **Euler (2023, $197M): out of scope** — it was a `donateToReserves()`
  accounting bug, *not* oracle manipulation. `safe-oracle` correctly does **not**
  false-positive on its honest feed and could not have prevented it. We include
  it as a negative control and do **not** count its $197M as "prevented."

> We claim the defaults catch **3 / 3 oracle-manipulation attacks** in the
> corpus (~$244M), not "$441M prevented." The distinction is the whole point of
> a security standard: know exactly what you defend against.

Reproduce: `cargo test -p safe-oracle --test historical_replay`
