# PROJECT NOTES

## Phase Structure — Closed at Phase 8

Phase 1 → Phase 8 represented the from-scratch construction of `safe-oracle`.
With the completion of Phase 8.3 (web site production polish, OG image,
crates.io publish path, closure tag), the **formal phase structure is now closed**.

### Phase Summary

- **Phase 1–5:** `safe-oracle` library construction (5 guards, circuit breaker, validated config)
- **Phase 5.5 + Hardening:** Independent adversarial review (AR.H — 0 critical, 0 high, 3 medium closed)
- **Phase 6:** `oracle-watch` off-chain service (signed snapshots, pluggable webhook sinks)
- **Phase 7:** Stellar testnet deployment + e2e validation (3 contracts, 25+ on-chain tx hashes, 6 live bug fixes)
- **Phase 8:** Public-facing web site + crates.io metadata + closure

### Final Tags

| Tag | Meaning |
|-----|---------|
| `phase-2-complete` | Layer 1 guardrails closed |
| `phase-3-complete` | LiquidityRegistry contract closed |
| `phase-4-complete` | Layer 2 guardrails + e2e attack scenarios closed |
| `phase-5-complete` | Circuit breaker closed |
| `hardening-complete` | AR.H independent adversarial review closed |
| `phase-6-complete` | oracle-watch off-chain service closed |
| `phase-7-complete` (v0.1.0-testnet) | Testnet deployment + e2e validation closed |
| `phase-8-complete` (v0.1.0) | Web site + crates.io metadata + formal closure |

## Current State (Phase 8 Closure)

| Item | Value |
|------|-------|
| Workspace tests | **290 PASS** |
| Adversarial review | AR.H complete (0 critical, 0 high, 3 medium closed, 5 low, 10 info) |
| Live testnet contracts | 3 (LiquidityRegistry, mock-lending, mock-reflector) |
| Live testnet validation tx hashes | 4 (successful borrow, attack, rejection, recovery) — all verifiable on stellar.expert |
| Public web site | <https://soroban-oracle-safety.vercel.app> |
| Crate metadata | Ready for `cargo publish` (description, keywords, categories, README, include) — `cargo publish --dry-run` succeeded; actual publish pending operator credentials |
| License | Apache-2.0 |
| Repository | <https://github.com/Sahveli01/soroban-oracle-safety> |
| CI | green on Rust 1.95 stable |

## Going Forward — Incremental Mode

The project now enters **incremental development mode**. No more numbered
phases. No more end-of-phase tags. Standard software-engineering practice
applies:

- `fix: …` — bug fixes
- `feat: …` — new features
- `refactor: …` — non-behavior code changes
- `docs: …` — documentation updates
- `chore: …` — build/tooling/dependency updates

Examples of likely future work, none of them mandatory:

- Slack / PagerDuty `WebhookSink` implementations alongside the existing
  Discord and Telegram sinks (the `monitor.rs` trait pattern makes this trivial)
- Per-asset counter parameterization in `oracle-watch` (currently hard-defaults
  to USDC; supports per-deployment override via env)
- Real-time USD price feed for the counter asset (currently a static
  `usdc_price_usd: 1.0` placeholder — see Phase 9 prep notes in DEPLOYMENT.md)
- HSM/KMS integration for the attester signing key (currently env-var loaded)
- Multi-attester quorum in the registry contract
- Real Reflector mainnet integration when network conditions warrant
- Mainnet deployment after final mainnet audit

Anything not listed above is also fair game — the phase structure was a
construction scaffold, not a permanent contract.

## Mottomuz — Maintained Beyond the Phase Structure

The development principles that guided Phase 1–8 continue:

- **Empirical first.** No assumed APIs, no assumed versions, no assumed behavior.
- **No deviation, no shortcuts, no simplification.** Every workaround is
  explicit and explained.
- **Errors are not hidden.** Every regression, every CI failure, every live
  testnet bug — documented with the fix and the lesson.
- **"BİLMİYORUM" allowed**, fabrication forbidden. When the project hits a
  question it cannot answer empirically, that gets recorded too.
- **Audit-friendly.** Every commit message explains the why, not just the what.

These principles remain operative. Quality > speed.

## Closing Note

Phase 1 began with a workspace scaffold and a YieldBlox post-mortem. Phase 8
ends with a published artifact, a public web site, and a transparent audit
trail of every decision in between. The library does what it claimed to do
on a public ledger.

Going forward, treat this file as the "you are here" pointer. New
contributors read this first, then `README.md`, then `DEPLOYMENT.md`.
