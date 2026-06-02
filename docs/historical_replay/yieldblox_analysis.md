# YieldBlox / Blend V2 — Stellar, 22 February 2026

**Reported loss:** ~$10.2M · **Chain:** Stellar (native, 1:1 replay) ·
**Oracle attack:** yes · **Default outcome:** ✓ REJECTED (`ExcessiveDeviation`)

## Attack mechanism

YieldBlox's DAO-managed lending pool, built on Blend V2, accepted USTRY as
collateral priced by Reflector — a volume-weighted average price (VWAP) oracle.
The USTRY/USDC market on Stellar's SDEX was effectively dead: near-zero
liquidity and no trades in the 15 minutes before the attack. Because Reflector's
price for that window was VWAP-derived, a **single** abnormal SDEX trade became
the entire window, inflating USTRY from ~$1.00 to ~$106 (a 100× jump). The
attacker borrowed against the inflated collateral and drained ~$10.2M. Reflector,
Stellar consensus, and Blend V2 all functioned exactly as designed — the gap was
integrator-side: no deviation guard, no liquidity threshold, no thin-sampling
check.

## Replay

| Point | Price (USD) | Reflector i128 (14-dec) |
|-------|-------------|--------------------------|
| pre-attack | ~$1.00 | `100000000000000` |
| manipulated | ~$106.00 | `10600000000000000` |

Deviation: `|106 − 1| / 1 = 10500%` → **1,050,000 BPS**, vastly over the default
2000 BPS (20%) threshold.

## safe-oracle response

`replay_yieldblox_default_config_rejects` runs the series through
`lastprice()` with `SafeOracleConfig::default()` (Layer 2 **off**). Layer 1's
`check_deviation_from_pair` short-circuits with
`OracleSafetyViolation::ExcessiveDeviation` before any borrow can proceed — pure
on-chain Reflector math, no attester required.

### Sophisticated variant (defense-in-depth)

`replay_yieldblox_subthreshold_needs_layer2` is the honest companion. An attacker
who reads the post-mortem keeps the move at 5% (below the 20% threshold) but
exploits the *same* dead-market precondition. The test shows:

- **Default (Layer 1 only):** the 5% move passes — Layer 1 alone does **not**
  catch the sophisticated variant.
- **Layer 2 (opt-in):** the drained-book snapshot (smallest valid attestation)
  is rejected with `InsufficientLiquidity`.

This is the structural argument for the LiquidityRegistry: the thin order book
that *made YieldBlox possible* is the signal Layer 2 reads.

## Thresholds that decided the outcome

- `max_deviation_bps = 2000` (20%) → catches the 100× headline attack.
- `min_liquidity_usd = $10,000` (Layer 2, opt-in) → catches the sub-threshold
  variant against a drained book.

## Sources

- Halborn — *Explained: The YieldBlox Hack (February 2026)*:
  https://www.halborn.com/blog/post/explained-the-yieldblox-hack-february-2026
- protos — *YieldBlox lending pool hit by $10M hack on Stellar*:
  https://protos.com/yieldblox-lending-pool-hit-by-10m-hack-on-stellar/
- QuillAudits — *YieldBlox $10M Hack (Oracle Manipulation) Explained*:
  https://www.quillaudits.com/blog/hack-analysis/yeildblox-10m-hack-explained

*Loss reported as $10.2M (Halborn/protos/QuillAudits); some outlets report $10.8M.*
