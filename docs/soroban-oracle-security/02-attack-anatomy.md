# 2. Attack Anatomy

Each attack below is replayed against the **default** `safe-oracle` config in
[`crates/safe-oracle/tests/historical_replay/`](../../crates/safe-oracle/tests/historical_replay/).
Prices are sourced to public post-mortems (links in each replay JSON's `sources`
array and in [`docs/historical_replay/`](../historical_replay/)).

---

## YieldBlox / Blend V2 — Stellar, 22 Feb 2026 (~$10.2M)

**Vector:** single-trade price spike (Vector 1).

**Price series:** USTRY ~$1.00 → ~$106.00 (100×), one SDEX trade.

**Why the integrator's logic failed:** YieldBlox priced USTRY collateral with a
Reflector VWAP feed and borrowed against it directly. USTRY/USDC was a dead
market (no trades in the prior 15 minutes), so a single trade *became* the VWAP
window. The oracle reported $106 correctly; YieldBlox never asked whether a 100×
jump was plausible or whether the market had any depth.

**How safe-oracle catches it:** the deviation guardrail
`check_deviation_from_pair` (`crates/safe-oracle/src/lib.rs:847`) computes
`|106 − 1| / 1 = 1,050,000 bps`, far past the 2000 bps default, and returns
`ExcessiveDeviation` before any borrow proceeds — Layer 1, no opt-in.

**Replay:** `replay_yieldblox_default_config_rejects`
(`tests/historical_replay/yieldblox_feb_2026.rs`). A companion test
(`replay_yieldblox_subthreshold_needs_layer2`) shows the *sophisticated* variant
(a 5% nudge against the same drained book) passes Layer 1 but is caught by the
opt-in Layer 2 liquidity guardrail — the honest gap that justifies Layer 2.

---

## Mango Markets — Solana, 11 Oct 2022 (~$114M) · adapted

**Vector:** single-trade price spike (Vector 1).

**Price series:** MNGO ~$0.038 → ~$0.91 (>13×) within ~30 minutes.

**Why the integrator's logic failed:** Mango marked open perpetual positions to
an oracle fed by three exchanges. The attacker bought ~$4M of MNGO across those
exchanges, inflating the mark; his long position showed enormous unrealized PnL,
against which he borrowed and withdrew ~$114M. Mango acted on a mark that moved
2,300% in half an hour without a deviation ceiling.

**How safe-oracle catches it:** `check_deviation_from_pair`
(`crates/safe-oracle/src/lib.rs:847`) sees ~229,000 bps → `ExcessiveDeviation`.

**Adaptation:** Solana attack; real MNGO prices replayed through Soroban
`PriceData`/`Asset`. The guardrail is chain-agnostic (it compares two ticks).
**Replay:** `replay_mango_default_config_rejects`.

---

## BonqDAO — Polygon, 1–2 Feb 2023 (~$120M) · adapted

**Vector:** single-trade price spike via writable oracle (Vector 1).

**Price series:** WALBT ~$0.05 → inflated (conservative 5,000× stand-in in the
replay; the real on-chain `submitValue` was far larger).

**Why the integrator's logic failed:** BonqDAO used Tellor with *instantaneous*
updates. The attacker staked the minimum 10 TRB to become a reporter, submitted a
massively inflated WALBT price, minted ~100M BEUR against 0.1 WALBT, then reported
a near-zero price to liquidate others. BonqDAO consumed each reported value with
no deviation or liquidity sanity check.

**How safe-oracle catches it:** the inflated submit is a gigantic step move →
`check_deviation_from_pair` returns `ExcessiveDeviation`. The thin market (price
moved with 0.1 WALBT) is additionally backstopped by the opt-in Layer 2 liquidity
guardrail.

**Adaptation:** Polygon/Tellor → Soroban/Reflector transport; pre-attack price
real, manipulated price a conservative lower bound. **Replay:**
`replay_bonqdao_default_config_rejects` + `replay_bonqdao_thin_market_layer2_backstop`.

---

## Euler Finance — Ethereum, 13 Mar 2023 ($197M) · OUT OF SCOPE

**Vector:** none of the five — **not an oracle attack.**

**Price series:** DAI ~$1.00 → ~$1.0005 (≈5 bps). The feed never moved.

**Why it is here:** Euler was the largest hack of 2023; omitting it would look
like cherry-picking. It exploited `donateToReserves()`, which lacked a
post-donation solvency check — burning eTokens but not dTokens created an
artificial under-collateralized position the attacker self-liquidated at a
discount. The oracle reported honest prices throughout.

**How safe-oracle responds:** correctly returns **Ok** — there is no price
anomaly to act on. A price validator cannot prevent a collateral-accounting bug,
and we do not pretend it can. `loss_usd_in_scope = 0`.

**Replay:** `replay_euler_out_of_scope_no_price_anomaly` asserts the honest feed
passes — proving `safe-oracle` is *selective* (it fires on manipulation and stays
quiet otherwise), which is what makes its other verdicts trustworthy.

→ Next: [Defense Patterns](./03-defense-patterns.md)
