# Euler Finance — Ethereum, 13 March 2023 (OUT OF SCOPE)

**Reported loss:** ~$197M · **Chain:** Ethereum (adapted) · **Oracle attack:**
**NO** · **Default outcome:** — PASS (no price anomaly; not preventable by an
oracle validator)

> This is an **honest negative control**, not a prevention claim. Euler's $197M
> is deliberately excluded from any "prevented" total (`loss_usd_in_scope: 0`).

## Why Euler is here

Euler was the largest DeFi hack of 2023, so any "exploit replay" corpus that
quietly omitted it would look like cherry-picking. We include it precisely to be
honest about safe-oracle's boundary: **it stops oracle manipulation, and Euler
was not an oracle manipulation.**

## Attack mechanism

The exploit used six flash-loan-funded transactions to abuse
`donateToReserves()`, which lacked a post-donation solvency check. The function
burned eTokens but not the corresponding dTokens, producing an artificial
under-collateralized position. The attacker then self-liquidated that position at
a discount via Euler's soft-liquidation mechanism, pocketing the difference. As
security write-ups state plainly, the attack was **"notably not based on oracle
manipulation"** — the price feed reported legitimate prices throughout.

## Replay

| Point | Price (USD) | Reflector i128 (14-dec) |
|-------|-------------|--------------------------|
| pre-attack | ~$1.00 (honest DAI feed) | `100000000000000` |
| during attack | ~$1.0005 (still honest) | `100050000000000` |

Deviation: ~5 BPS — well within tolerance. The feed never moved.

## safe-oracle response

`replay_euler_out_of_scope_no_price_anomaly` feeds the honest series through
`lastprice()` with `SafeOracleConfig::default()` and asserts the result is **Ok**,
returning the unchanged price. This proves two things:

1. **No false positive.** safe-oracle does not flag a legitimate, stable feed
   just because the protocol around it is being attacked by other means.
2. **Honest scope.** A price validator cannot defend against a
   collateral-accounting / liquidation-logic bug. We report that openly rather
   than inflating a headline number.

## Takeaway

Including Euler strengthens, rather than weakens, the corpus: it demonstrates that
safe-oracle's "✓ REJECTED" verdicts for YieldBlox, Mango, and BonqDAO are
*selective* — the library fires on oracle manipulation and stays quiet on
everything else. The right defense for an Euler-class bug is a solvency
invariant in the lending contract, not an oracle guardrail.

## Sources

- Chainalysis — *Euler Finance Flash Loan Attack Explained*:
  https://www.chainalysis.com/blog/euler-finance-flash-loan-attack/
- Hacken — *The Euler Finance Hack Explained*:
  https://hacken.io/discover/euler-finance-hack/
- BlockSec — *Euler Finance Incident: The Largest Hack of 2023*:
  https://blocksec.com/blog/euler-finance-incident-the-largest-hack-of-2023
