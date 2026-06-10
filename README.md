# safe-oracle

> One-line oracle safety for Soroban DeFi. Validate the price **before** your protocol trusts it.

[![Crates.io](https://img.shields.io/crates/v/safe-oracle.svg)](https://crates.io/crates/safe-oracle)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Tests](https://img.shields.io/badge/tests-321%20passing-brightgreen)](https://github.com/Sahveli01/soroban-oracle-safety)
[![Testnet](https://img.shields.io/badge/testnet-live-blue)](https://stellar.expert/explorer/testnet/contract/CBMDP4NLNLI2T5XJPMGT23QUXZKLD67YX6YGVNUKPLEFRCJG3ICUJX4K)

**We don't replace oracles. We validate them.**

---

## The Problem

On **22 February 2026**, an attacker drained **~$10.2M** from YieldBlox with a single ~$5 SDEX trade. Reflector worked. Stellar worked. Blend V2 worked. The oracle reported an *honest* price for a market that was, for one ledger, real and almost empty.

The gap was integrator-side: the lending contract trusted the raw `lastprice` with no deviation guard, no liquidity threshold, no staleness window between the feed and the action it gated. `safe-oracle` closes exactly that gap — the boundary between an oracle telling you a number and your protocol acting on it.

Think of it as `SafeERC20` for price feeds: a thin, defensive wrapper you put *between* the oracle and the risky action.

---

## What safe-oracle Does

`safe-oracle` wraps an existing Reflector (SEP-40) oracle call with mathematically-checked guardrails. Each one closes a specific attack vector seen in real DeFi exploits. The result is a `PriceResult` that is either a validated price or a typed rejection — your contract `?`-propagates the error and never acts on a manipulated feed.

It is **purely defensive**: it does not produce prices, replace Reflector, or hold collateral. It validates a price someone else produced and gates your downstream logic on the verdict.

---

## Quick Start

```toml
[dependencies]
safe-oracle = "0.4"
soroban-sdk = "25.3"
```

Or: `cargo add safe-oracle`

In your contract — the full integration is ~8 lines:

```rust
use safe_oracle::{lastprice, SafeOracleConfig};

let result = lastprice(
    &env,
    &asset,
    &reflector_address,
    &registry_address,
    &SafeOracleConfig::default(),
);

let price = result.into_result()?;  // PriceResult -> Result, ergonomic `?`
// `price` is now safe to use: it survived every default guardrail.
```

With `SafeOracleConfig::default()`, the **three Layer 1 guardrails** are active — pure on-chain Reflector math, no off-chain trust. **321 tests passing.**

---

## How It Works

### Layer 1 — Trustless, on by default

Pure on-chain Reflector math. No external trust vector.

| Guardrail | What it catches | Default threshold |
|-----------|-----------------|-------------------|
| **Deviation** | Sudden price spikes between consecutive updates | 2000 BPS (20%) |
| **Staleness** | Outdated feeds (current + previous price) | 300s / 900s |
| **Cross-Source** | Disagreement between a primary and secondary oracle | 500 BPS (opt-in 2nd feed) |

### Layer 2 — Opt-in

Reads attested `LiquidityRegistry` snapshots, which introduces a second trust vector — the off-chain attesters that sign those snapshots. **Off by default** (`config.layer2_enabled`, default `false`):

```rust
let config = SafeOracleConfig { layer2_enabled: true, ..SafeOracleConfig::default() };
```

| Guardrail | What it catches | Default threshold |
|-----------|-----------------|-------------------|
| **Liquidity** | Thin SDEX 30-minute volume | $10,000 USD |
| **Thin Sampling** | Low trader diversity (1-hour unique-trader count) | 5 traders |

When `layer2_enabled` is `false`, the `LiquidityRegistry` is never queried — not even for `Asset::Stellar`. The default posture (Layer 1 only) is fully trustless.

> **Plus — Circuit Breaker (opt-in).** After any violation, the affected asset can auto-halt for a configurable window (default ~1h at 720 ledgers), with a manual governance override via `close_circuit_breaker`. Enabled per `config.circuit_breaker_enabled`.

---

## Empirical Validation — Historical Exploit Replay

The default thresholds are not intuition. They are replayed against real DeFi exploits, every price loaded from a cited public source, through the live `safe_oracle::lastprice()` path.

| Attack | Date | Reported loss | Chain | Oracle attack? | Default (Layer 1) outcome |
|--------|------|---------------|-------|----------------|---------------------------|
| YieldBlox | 2026-02-22 | ~$10.2M | Stellar (native) | Yes | ✓ REJECTED — `ExcessiveDeviation` |
| Mango Markets | 2022-10-11 | ~$114M | Solana (adapted) | Yes | ✓ REJECTED — `ExcessiveDeviation` |
| BonqDAO | 2023-02-02 | ~$120M | Polygon (adapted) | Yes | ✓ REJECTED — `ExcessiveDeviation` |
| Euler Finance | 2023-03-13 | ~$197M | Ethereum (adapted) | **No** | — out of scope (negative control) |

**Oracle-manipulation attacks in the corpus: 3 / 3 rejected by the default config (~$244.2M in-scope), with zero opt-in** — pure on-chain Reflector math, Layer 2 disabled.

Euler's $197M is **deliberately excluded** from any "prevented" total: it was a `donateToReserves()` accounting bug, not oracle manipulation, and its feed was honest throughout. safe-oracle correctly returns `Ok` on it (no false positive) and would **not** have stopped it. We do **not** claim "$441M prevented" — only the ~$244M that was actually oracle manipulation is in scope, and the defaults catch all of it.

Non-Stellar attacks are marked `adapted` (real foreign-chain prices replayed through Soroban semantics). Per-attack analysis and source links: [`docs/historical_replay/`](./docs/historical_replay/). Reproduce with:

```bash
cargo test -p safe-oracle --test historical_replay
```

---

## Multi-Oracle Support

safe-oracle is **oracle-agnostic** — it validates any SEP-40-shaped feed, not just Reflector. The oracle address is a parameter to `oracle-validator.validate(oracle, asset)`, so the same guardrail logic applies unchanged to any feed exposing the Reflector interface:

- **Reflector** (production target) — validated directly via the SEP-40 `prices(asset, records)` history. *(As of v0.4.0, the deviation guardrail reads `prices()`, not the singular `lastprice`, so it has real consecutive samples to compare.)*
- **Other SEP-40 feeds** — any source exposing `lastprice`/`prices`/`decimals` plugs into the same validator with no code change; a feed with a different native shape can be wrapped by a thin Reflector-shaped adapter.

**Honest scope:** Reflector is the only third-party feed live on testnet here. Band and DIA are *not* deployed — the live demo shows them honestly as unavailable rather than fabricating a price.

---

## Deployed Contracts (Stellar Testnet)

All are live and callable read-only via `simulateTransaction`.

| Contract | Role | Address |
|----------|------|---------|
| **oracle-validator** | Oracle-agnostic `validate(oracle, asset)` entry point | [`CBMDP4NL…JX4K`](https://stellar.expert/explorer/testnet/contract/CBMDP4NLNLI2T5XJPMGT23QUXZKLD67YX6YGVNUKPLEFRCJG3ICUJX4K) |
| **liquidity-registry** | Attested Layer-2 liquidity snapshots | [`CCDWMKL5…WGND`](https://stellar.expert/explorer/testnet/contract/CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND) |
| mock-lending | Demo integrator (borrow guarded by safe-oracle) | [`CA6TBUXT…MXZV`](https://stellar.expert/explorer/testnet/contract/CA6TBUXTIQKHD4VZ3MMQTJTTREMHHYQD4G6R3OTOOVGHOGQNXUYSMXZV) |
| mock-reflector | Demo price feed (Reflector interface, `set_price`) | [`CBUPTLPD…PHO7`](https://stellar.expert/explorer/testnet/contract/CBUPTLPDDNCB2OHTGTHD3DKHLGSZUDUMINU5OKU4CG5ZRHW5T7ATPHO7) |

**Verified live (read-only simulation):**

- `oracle-validator.validate(Reflector, BTC)` runs the full Layer 1 guardrail chain against the real Reflector testnet feed with no signature and no fee — try it on the [live demo](https://soroban-oracle-safety.vercel.app/demo), which calls it through `simulateTransaction` and renders the verdict, price, and live data-age from the on-chain timestamp.

**End-to-end on-chain evidence (testnet, public):**

- Successful borrow at ledger 2,450,314 — all guardrails passed: [`ce481203…`](https://stellar.expert/explorer/testnet/tx/ce4812031daa61ecb987c45123fbaba52eb83fe0b27f623dd3fa3fa0ec8a5c45)
- Adversarial replay (10× XLM spike) rejected by Layer 1 deviation: attack [`b99d6134…`](https://stellar.expert/explorer/testnet/tx/b99d61340c63748394f27a589ac228bbc6a02aba7d74c5b50b67a416ee6acfb6) → rejection [`a1cfdec1…`](https://stellar.expert/explorer/testnet/tx/a1cfdec1fe8f6c778c0f6f48f481c0b7dfd31ea7322834d84944459ca80a7653) (`Failed(1) = ExcessiveDeviation`)
- Stale-oracle scenario rejected by Layer 1 staleness: inject [`522e2ab4…`](https://stellar.expert/explorer/testnet/tx/522e2ab4d8ee951447cb6f28132d22a0750d86026599b5bf04f2bdd642f88774) → rejection [`7b799e02…`](https://stellar.expert/explorer/testnet/tx/7b799e02c54d90334e2c45a2acdf2c43f4652d1fb125073896ebce1dc72a21f9) (`Failed(2) = StaleData`)

Full deployment artifact (all contract IDs + tx hashes): [`deployment/testnet.json`](deployment/testnet.json).

---

## Live Demo

**[soroban-oracle-safety.vercel.app/demo](https://soroban-oracle-safety.vercel.app/demo)** — a real-data-age indicator over the live testnet feed, plus a cinematic split-screen attack replay (DRAINED vs PROTECTED).

---

## Security Standard

Beyond the implementation, this repo documents a reusable **[Soroban Oracle Security standard](./docs/soroban-oracle-security/)** — threat model, attack anatomy, the defense patterns (each linked to its real `file:line`), an integration guide, empirical threshold calibration, and a pre-mainnet audit checklist. `safe-oracle` is the working reference implementation of that standard.

---

## Architecture

```
        Integrator                Library                 External
        ──────────                ────────                ────────

    your_contract
        │
        ▼
    lastprice() ───→ safe_oracle ──┬──→ Reflector / adapter
                                   │     (price + decimals)
                                   │
                                   ├──→ LiquidityRegistry      (Layer 2,
                                   │     (volume + traders)     opt-in)
                                   ▼
                              guardrails
                                   │
                                   ▼
                    PriceResult::Ok | PriceResult::Err
        │
        ▼
    use price
```

`oracle-watch` is the off-chain companion service: it monitors SDEX trade flow via Horizon, aggregates volume + unique-trader counts, signs `LiquiditySnapshot`s, and submits them to `LiquidityRegistry`. Operator-run; supports five pluggable webhook sinks (Discord, Telegram, Slack, PagerDuty Events API v2, and a Generic HTTPS sink) via the `WebhookSink` trait. See [`DEPLOYMENT.md`](DEPLOYMENT.md).

### Crate Layout

| Crate | Purpose |
|-------|---------|
| `safe-oracle` | The guardrail library (rlib). Stateless — storage lives in the calling contract. |
| `oracle-validator` | On-chain `validate(oracle, asset)` contract — oracle-agnostic entry point. |
| `liquidity-registry` | On-chain attestation contract for SDEX volume snapshots. |
| `oracle-watch` | Off-chain Rust service. Polls Horizon, aggregates, signs, submits. |
| `mock-reflector` | SEP-40 Reflector mock with `set_price` for adversarial scenarios. |
| `mock-lending` | Reference integrator showing the ~8-line integration pattern. |
| `test-utils` | Shared `TestEnv` harness used across the workspace. |

---

## Security & Honesty

| Severity | Count | Status |
|----------|-------|--------|
| Critical | **0** | — |
| High | **0** | — |
| Medium | 3 | All closed |
| Low | 5 | All closed |
| Info | 10 | Documented |

- **Adversarial review (AR.H):** an internal review attempted 20+ distinct attack vectors across the guardrails; 0 critical and 0 high findings, all medium/low closed. Findings are traceable in module doc-comments (`AR.H M1 fix:`, etc.).
- **No third-party audit yet.** The review above is internal/adversarial, not an independent professional audit. Treat accordingly before any mainnet use.
- **Layer 2 adds a trust vector.** It depends on off-chain attesters signing liquidity snapshots, which is why it is opt-in and off by default. The trustless default is Layer 1 only.
- **Testnet demo configs are relaxed** (24h staleness windows, $1 liquidity floor) to accommodate thin testnet liquidity — production defaults are stricter (see `mock-lending` config note in `deployment/testnet.json`).

---

## Project Status

- **Published:** crates.io [`safe-oracle` v0.4.0](https://crates.io/crates/safe-oracle)
- **Tests:** 321 passing (`cargo test --workspace`)
- **License:** Apache-2.0
- **Mainnet:** planned

### Building & Testing

Requires stable Rust (MSRV 1.85; developed on 1.95) and the `wasm32v1-none` target.

```bash
cargo build --workspace
cargo test --workspace
```

---

## Links

- **Crate:** [crates.io/crates/safe-oracle](https://crates.io/crates/safe-oracle)
- **Source:** [github.com/Sahveli01/soroban-oracle-safety](https://github.com/Sahveli01/soroban-oracle-safety)
- **Live demo:** [soroban-oracle-safety.vercel.app/demo](https://soroban-oracle-safety.vercel.app/demo)
- **Security standard:** [`docs/soroban-oracle-security/`](./docs/soroban-oracle-security/)
- **Operator/integrator guide:** [`DEPLOYMENT.md`](DEPLOYMENT.md)
- **Changelog:** [`CHANGELOG.md`](CHANGELOG.md)

---

## License

Apache License 2.0. See [LICENSE](./LICENSE).

## Author

[@Sahveli01](https://github.com/Sahveli01) · Built for [Stellar Soroban](https://stellar.org/soroban). Oracle integration via [Reflector Network](https://reflector.network/).
