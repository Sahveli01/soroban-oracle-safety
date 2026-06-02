# 1. Threat Model

## The trust boundary

```
   ┌────────────┐   price    ┌──────────────┐   action   ┌──────────┐
   │   Oracle   │ ─────────► │  Integrator  │ ─────────► │  Funds   │
   │ (Reflector)│            │  (lending)   │            │ at risk  │
   └────────────┘            └──────────────┘            └──────────┘
        trusted                    YOU                     borrow /
       to report               must validate              liquidate /
     a price honestly          before acting                 mint
```

The oracle is trusted to do one job: report the price its data sources produced.
It is **not** responsible for whether that price is *safe to act on*. A VWAP
oracle in a market with one trade reports that trade faithfully — the price is
"correct" and "manipulated" at the same time.

**The threat lives entirely on the integrator side of the boundary.** Every
attack in scope here is an integrator that read a technically-correct oracle
value and took an irreversible action without asking *"is this value plausible?"*

`safe-oracle` sits exactly on that boundary: it is the validation step between
`oracle.lastprice()` and the action. See
[`lastprice()`](../../crates/safe-oracle/src/lib.rs) at `crates/safe-oracle/src/lib.rs:651`.

## The five oracle-manipulation vectors

Each vector is a real, observed shape — not a hypothetical. Each maps to exactly
one guardrail and one `OracleSafetyViolation` discriminant
(`crates/safe-oracle/src/lib.rs:57`).

| # | Vector | Real-world example | Guardrail | Violation | Layer |
|---|--------|--------------------|-----------|-----------|-------|
| 1 | **Single-trade price spike** | YieldBlox ($1→$106), Mango ($0.038→$0.91), BonqDAO | Deviation | `ExcessiveDeviation` | 1 |
| 2 | **Stale price exploitation** | Feed paused / network partition; old price used to value collateral | Staleness | `StaleData` | 1 |
| 3 | **Single-source over-trust** | One feed shifted while an independent feed disagrees | Cross-source | `CrossSourceMismatch` | 1 (opt-in) |
| 4 | **Thin-liquidity manipulation** | Drained order book — the *next* trade moves price arbitrarily | Liquidity | `InsufficientLiquidity` | 2 (opt-in) |
| 5 | **Low trader-count manipulation** | "VWAP-of-one" — a single trader dominates the pricing window | Thin sampling | `ThinSampling` | 2 (opt-in) |

### Vector 1 — Single-trade price spike

A manipulator moves the spot price with one trade in an illiquid market; the
oracle's next reading reflects it. **Caught by** the deviation guardrail, which
compares consecutive oracle reads and rejects moves beyond `max_deviation_bps`
(default 2000 = 20%). Empirically caught YieldBlox, Mango, and BonqDAO — see
[Attack Anatomy](./02-attack-anatomy.md).

### Vector 2 — Stale price exploitation

The off-chain feed stalls (paused upstream, RPC outage, deliberate hold) and the
on-chain price no longer reflects reality. An integrator that values collateral
against a stale price can be drained as the real price moves. **Caught by** the
staleness guardrail (`max_staleness_seconds`, default 300s) plus a separate
freshness gate on the *previous* price used for the deviation comparison
(`previous_max_staleness_seconds`, default 900s) so a post-gap recovery is not
misclassified as a violent move.

### Vector 3 — Single-source over-trust

An attack that shifts only one feed is invisible to a single-source integrator.
**Caught by** the cross-source guardrail when a secondary oracle is configured:
disagreement beyond `max_cross_source_bps` (default 500 = 5%) is rejected. Opt-in
(`secondary_oracle = Some(addr)`) because it adds a second data dependency.

### Vector 4 — Thin-liquidity manipulation

Even when the *price* looks clean to Layer 1, an order book drained to near-zero
is structurally unsafe: the next trade (including the borrower's own liquidation)
can move price by an unbounded amount. This is the precondition that *made*
YieldBlox possible. **Caught by** the Layer 2 liquidity guardrail against an
attested 30-minute SDEX volume (`min_liquidity_usd`, default $10,000).

### Vector 5 — Low trader-count manipulation

A market can clear the volume threshold while still being dominated by a single
trader ("VWAP-of-one"). **Caught by** the Layer 2 thin-sampling guardrail
(`min_trade_count_1h`, default 5 unique trades/hour), which is independent of the
dollar-volume check.

## What we do **not** defend against

Scope honesty is a security property. `safe-oracle` validates oracle *prices*. It
does **not** defend against:

- **Collateral-accounting / liquidation-logic bugs** — e.g. Euler ($197M,
  `donateToReserves()` missing a solvency check). The price feed was honest; no
  oracle guardrail applies. The right defense is a solvency invariant in the
  lending contract. See [Attack Anatomy → Euler Finance](./02-attack-anatomy.md).
- **Smart-contract bugs** in the integrator (reentrancy, math errors, access
  control) — outside the oracle boundary entirely.
- **Governance / key-compromise attacks** — if the attester signing keys or the
  protocol admin are compromised, the trust model is already broken upstream of
  validation.
- **Oracle-internal compromise** — if Reflector itself is fully subverted at the
  source level, cross-source (Vector 3) is the only partial mitigation, and only
  if the secondary is independent.

A guardrail that claims to stop everything stops nothing credibly. Knowing the
edge of the defended region is what lets an integrator add the *other* defenses
(solvency checks, audits, key management) deliberately.

→ Next: [Attack Anatomy](./02-attack-anatomy.md)
