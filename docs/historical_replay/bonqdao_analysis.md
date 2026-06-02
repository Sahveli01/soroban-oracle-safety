# BonqDAO — Polygon, 1–2 February 2023

**Reported loss:** ~$120M at-risk (~$1.8M realized) · **Chain:** Polygon
(**adapted** to Soroban) · **Oracle attack:** yes · **Default outcome:**
✓ REJECTED (`ExcessiveDeviation`)

## Attack mechanism

BonqDAO sourced its WALBT (Wrapped AllianceBlock Token) price from the Tellor
oracle. Tellor lets anyone become a reporter by staking the minimum 10 TRB. The
attacker staked, then called `submitValue` to set the WALBT price to a massively
inflated value. BonqDAO used **instantaneous** Tellor updates, so the attacker
immediately deposited 0.1 WALBT into a trove and minted ~100M BEUR against the
inflated collateral. They then reported a near-zero WALBT price to liquidate
other users' troves. PeckShield put the protocol loss at ~$120M; realized profit
was ~$1.8M after laundering, bounded by on-chain liquidity (the same thin-market
reality that bounds every such attack).

## Replay (adapted)

| Point | Price (USD) | Reflector i128 (14-dec) |
|-------|-------------|--------------------------|
| pre-attack | ~$0.05 (real ALBT) | `5000000000000` |
| manipulated | ~$5,000 (conservative stand-in) | `500000000000000000` |

Deviation at this conservative 5,000× figure: **~10,000,000,000 BPS** — and the
real on-chain inflation (derived from minting 100M BEUR against 0.1 WALBT, ≈ $1B
per token) was far larger. Any move >20% trips the guardrail, so the verdict
holds regardless of the exact figure.

## Adaptation + honesty note

- **Adapted:** Polygon/Tellor → Soroban/Reflector transport. The deviation
  guardrail compares consecutive ticks and is chain-agnostic.
- **Conservative manipulated price:** the pre-attack WALBT price (~$0.05) is real;
  the exact `submitValue` figure is not cleanly published, so we use a
  conservative 5,000× lower bound for readable arithmetic. This **understates**
  the real manipulation — the guardrail would fire even harder on the true value.

## safe-oracle response

`replay_bonqdao_default_config_rejects` trips `ExcessiveDeviation` at Layer 1
under `SafeOracleConfig::default()`. A second test
(`replay_bonqdao_thin_market_layer2_backstop`) shows that even a hypothetical
sub-threshold Tellor nudge against BonqDAO's thin market (the attacker moved
price with 0.1 WALBT) is backstopped by the opt-in Layer 2 liquidity guardrail —
the same structural defense as the YieldBlox sophisticated variant.

## Sources

- Halborn — *Explained: The BonqDAO Hack (February 2023)*:
  https://www.halborn.com/blog/post/explained-the-bonqdao-hack-february-2023
- Hacken — *The BonqDAO Price Oracle Hack Explained*:
  https://hacken.io/insights/bonqdao-hack/
- Immunefi — *Hack Analysis: BonqDAO, February 2023*:
  https://medium.com/immunefi/hack-analysis-bonqdao-february-2023-ef6aab0086d6
