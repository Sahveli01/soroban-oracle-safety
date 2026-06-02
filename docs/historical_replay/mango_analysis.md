# Mango Markets — Solana, 11 October 2022

**Reported loss:** ~$114M (estimates $110M–$117M) · **Chain:** Solana
(**adapted** to Soroban) · **Oracle attack:** yes · **Default outcome:**
✓ REJECTED (`ExcessiveDeviation`)

## Attack mechanism

Avraham Eisenberg funded two Mango accounts with ~$10M USDC and used them to take
opposite sides of a large MNGO perpetual position (one short ~488M MNGO, the
other long the same size). He then bought ~$4M of MNGO across the three exchanges
that fed Mango's price oracle. The oracle-reported MNGO price jumped over 13× —
from ~$0.038 to a peak of ~$0.91 (~2,300%) — within a ~30-minute span. The
inflated mark gave his long position enormous unrealized PnL, against which he
borrowed and withdrew ~$114M, draining the protocol. The CFTC later charged him
with a manipulative scheme; the figure cited was "over $110 million."

## Replay (adapted)

| Point | Price (USD) | Reflector i128 (14-dec) |
|-------|-------------|--------------------------|
| pre-attack | ~$0.038 | `3800000000000` |
| manipulated | ~$0.91 | `91000000000000` |

Deviation: `|0.91 − 0.038| / 0.038 ≈ 2,294%` → **~229,000 BPS**, far over the
default 2000 BPS threshold.

## Adaptation note

Mango ran on Solana, not Soroban. We replay the **real MNGO oracle prices**
(sourced below) through Soroban's `PriceData` / `Asset` types. safe-oracle's
deviation guardrail compares two consecutive oracle ticks and is chain-agnostic —
the adaptation changes the transport, not the arithmetic. JSON marked
`adapted: true`.

## safe-oracle response

`replay_mango_default_config_rejects` runs the series through `lastprice()` with
`SafeOracleConfig::default()`. The >13× oracle pump trips
`OracleSafetyViolation::ExcessiveDeviation` at Layer 1. Even if the manipulation
had been spread across several oracle ticks, each step from $0.038 toward $0.91
exceeds 20%, so the guardrail fires on the first one.

## Sources

- CFTC — *Charges against Avraham Eisenberg* (press release 8647-23):
  https://www.cftc.gov/PressRoom/PressReleases/8647-23
- Chainalysis — *Oracle Manipulation Attacks Rising*:
  https://www.chainalysis.com/blog/oracle-manipulation-attacks-rising/
- CoinDesk — *How Market Manipulation Led to a $100M Exploit on Mango*:
  https://www.coindesk.com/markets/2022/10/12/how-market-manipulation-led-to-a-100m-exploit-on-solana-defi-exchange-mango
