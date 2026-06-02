# 5. Threshold Calibration

This chapter separates two things that are easy to conflate:

- **Empirical** — claims backed by the replay corpus. These are proven.
- **Engineering guidance** — reasoned starting points for tuning. These are
  *not* backtested per asset class, and they are labelled as such.

Honesty about which is which is the point of a standard.

---

## Empirical: why the default deviation is 2000 bps (20%)

The default `max_deviation_bps = 2000`
(`crates/safe-oracle/src/lib.rs:360`) was chosen to sit well above ordinary
inter-tick volatility for major pairs while remaining far below the moves real
oracle-manipulation attacks produce. The replay corpus
(`crates/safe-oracle/tests/historical_replay/`) confirms the second half of that
claim directly:

| Attack | Observed move | Deviation | vs. 2000 bps default |
|--------|---------------|-----------|----------------------|
| YieldBlox | $1.00 → $106 (100×) | ~1,050,000 bps | rejected by ~525× margin |
| Mango | $0.038 → $0.91 (>13×) | ~229,000 bps | rejected by ~114× margin |
| BonqDAO | ~$0.05 → ~$5,000 (5,000× conservative) | ~10⁹ bps | rejected by a vast margin |

**Empirical result: 3 / 3 oracle-manipulation attacks in the corpus are rejected
by the default**, each by a large margin. Even spread across several oracle
ticks, each per-tick step in these attacks still exceeds 20%, so the first step
trips the guardrail.

**Scope-honest caveat:** the corpus is 3 in-scope attacks. "Caught 3/3" is a
statement about *this corpus*, not a universal capture rate. A *sub-threshold*
attack (a deliberate <20% nudge against a thin book) is **not** caught by Layer 1
— that is exactly why Layer 2 exists, and the
`replay_yieldblox_subthreshold_needs_layer2` test documents the gap rather than
hiding it.

---

## Engineering guidance: per-asset-class thresholds

> **Not backtested.** The ranges below are reasoned starting points based on
> typical inter-tick volatility, not empirical capture rates. Calibrate against
> your own asset's history and document your choice.

| Asset class | Example | `max_deviation_bps` (start) | Reasoning |
|-------------|---------|-----------------------------|-----------|
| Stablecoin pairs | USDC/USDT | 100–500 (1–5%) | A real depeg is itself a risk signal; tight caps catch manipulation early. |
| Major pairs | XLM/USDC | 1000–2000 (10–20%) | The default range; absorbs normal volatility, rejects spikes. |
| Volatile small-caps | new SDEX tokens | 3000–5000 (30–50%) | Wider genuine swings; pair with Layer 2 rather than loosening blindly. |

For the other thresholds, start from the validated defaults and adjust to the
asset's reality:

| Field | Default | When to change |
|-------|---------|----------------|
| `max_staleness_seconds` | 300 | Match the feed's update cadence; never > 86,400. |
| `previous_max_staleness_seconds` | 900 | ~2–3× the current-price threshold. |
| `max_cross_source_bps` | 500 | Tighten if your two feeds track closely. |
| `min_liquidity_usd` | $10,000 (7-dec) | Scale to pool size and typical position. |
| `min_trade_count_1h` | 5 | Raise for markets you expect to be active. |

---

## The core trade-off: false positive vs. false negative

- **Too tight** → false positives: legitimate volatility halts borrowing
  (availability cost).
- **Too loose** → false negatives: a manipulation slips under the cap
  (security cost).

The layered design lets you avoid choosing globally: keep Layer 1 deviation at a
*loose* enough cap to avoid false positives on normal volatility, and use **Layer
2** to catch the sub-threshold manipulation a loose Layer 1 would miss. Loosening
Layer 1 is only safe when Layer 2 (or cross-source) covers the gap.

---

## Backtest your own config

The replay corpus is the regression harness for thresholds. Run it against your
chosen config before mainnet:

```bash
cargo test -p safe-oracle --test historical_replay
```

If you loosen `max_deviation_bps`, confirm the corpus still rejects what you
intend it to — a 5,000-bps cap, for instance, would let a hypothetical 40% spike
through, which may or may not be acceptable for your asset. Make that a conscious,
documented decision.

→ Next: [Audit Checklist](./06-checklist.md)
