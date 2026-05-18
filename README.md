# safe-oracle

[![Crates.io](https://img.shields.io/crates/v/safe-oracle.svg)](https://crates.io/crates/safe-oracle)
[![Tests](https://img.shields.io/badge/tests-310%20passing-brightgreen)](https://github.com/Sahveli01/soroban-oracle-safety)
[![Testnet](https://img.shields.io/badge/testnet-live-blue)](https://stellar.expert/explorer/testnet/contract/CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

**Drop-in oracle protection for Stellar Soroban.**

On 22 February 2026, an attacker drained ~$10.2M from YieldBlox with a single $5 SDEX trade. Reflector worked. Stellar worked. Blend V2 worked. The gap was integrator-side: no deviation guard, no liquidity threshold, no thin-sampling check. `safe-oracle` closes that gap.

---

## Quick Start

Add to `Cargo.toml`:

```toml
[dependencies]
safe-oracle = "0.2"
soroban-sdk = "25.3"
```

Or: `cargo add safe-oracle`

In your contract:

```rust
use safe_oracle::{lastprice, SafeOracleConfig};

let result = lastprice(
    &env,
    &asset,
    &reflector_address,
    &registry_address,
    &SafeOracleConfig::default(),
);

let price = result.into_result()?;  // PriceResult -> Result for ergonomic ?
// Now safe to use this price.
```

That's it. Five guardrails active. 310 tests passing.

---

## What It Does

`safe-oracle` wraps your existing Reflector oracle calls with five mathematically-validated guardrails. Each one closes a specific attack vector observed in real DeFi exploits:

### Layer 1 — Oracle-Side Checks

| Guardrail | What It Catches | Default Threshold |
|-----------|-----------------|-------------------|
| **Deviation** | Sudden price spikes between consecutive updates | 2000 BPS (20%) |
| **Staleness** | Outdated feeds (current + previous price) | 300s / 900s |
| **Cross-Source** | Disagreement between primary and secondary oracles | 500 BPS (opt-in) |

### Layer 2 — Market Microstructure Checks

| Guardrail | What It Catches | Default Threshold |
|-----------|-----------------|-------------------|
| **Liquidity** | Thin SDEX 30-minute volume | $10,000 USD |
| **Thin Sampling** | Low trader diversity (1-hour unique trader count) | 5 traders |

### Plus: Circuit Breaker (Phase 5)

After any guardrail violation, the affected asset can auto-halt for a configurable window (default ~1 hour at 720 ledgers). Manual governance override available via `close_circuit_breaker`. Opt-in per `config.circuit_breaker_enabled`.

---

## Live on Stellar Testnet

| Contract | Address |
|----------|---------|
| LiquidityRegistry | [`CCDWMKL5...WGND`](https://stellar.expert/explorer/testnet/contract/CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND) |
| mock-lending | [`CA6TBUXT...MXZV`](https://stellar.expert/explorer/testnet/contract/CA6TBUXTIQKHD4VZ3MMQTJTTREMHHYQD4G6R3OTOOVGHOGQNXUYSMXZV) |
| mock-reflector | [`CBUPTLPD...PHO7`](https://stellar.expert/explorer/testnet/contract/CBUPTLPDDNCB2OHTGTHD3DKHLGSZUDUMINU5OKU4CG5ZRHW5T7ATPHO7) |

**End-to-end validation evidence (testnet, public on-chain):**

- 17 consecutive `oracle-watch` attestation submissions, polling SDEX trade flow → signed `LiquiditySnapshot` → `LiquidityRegistry`
- Successful borrow at ledger 2,450,314 — all 5 guardrails passed: [`ce481203...`](https://stellar.expert/explorer/testnet/tx/ce4812031daa61ecb987c45123fbaba52eb83fe0b27f623dd3fa3fa0ec8a5c45)
- **Adversarial replay (10× price spike) rejected** by Layer 1 deviation guardrail:
  - Attack ([`b99d6134...`](https://stellar.expert/explorer/testnet/tx/b99d61340c63748394f27a589ac228bbc6a02aba7d74c5b50b67a416ee6acfb6)): 10× XLM price spike via mock-reflector ($0.10 → $1.00, 90000 BPS deviation)
  - Rejection ([`a1cfdec1...`](https://stellar.expert/explorer/testnet/tx/a1cfdec1fe8f6c778c0f6f48f481c0b7dfd31ea7322834d84944459ca80a7653)): borrow returned `BorrowOutcome::Failed(1) = ExcessiveDeviation`
- **Stale oracle scenario rejected** by Layer 1 staleness guardrail:
  - Inject ([`522e2ab4...`](https://stellar.expert/explorer/testnet/tx/522e2ab4d8ee951447cb6f28132d22a0750d86026599b5bf04f2bdd642f88774)): mock-reflector price timestamp set 48 hours old (value unchanged at $0.10)
  - Rejection ([`7b799e02...`](https://stellar.expert/explorer/testnet/tx/7b799e02c54d90334e2c45a2acdf2c43f4652d1fb125073896ebce1dc72a21f9)): borrow returned `BorrowOutcome::Failed(2) = StaleData`

See [`deployment/testnet.json`](deployment/testnet.json) for the complete deployment artifact (all contract IDs, deploy/init tx hashes, validation evidence).

---

## Architecture

```
        Integrator                Library                 External
        ──────────                ────────                ────────

    your_contract
        │
        ▼
    lastprice() ───→ safe_oracle ──┬──→ Reflector
                                   │     (price + decimals)
                                   │
                                   ├──→ LiquidityRegistry
                                   │     (volume + traders)
                                   │
                                   ▼
                              5 guardrails
                                   │
                                   ▼
                    PriceResult::Ok | PriceResult::Err
        │
        ▼
    use price
```

The library is purely defensive — it doesn't replace Reflector or Stellar's built-in price feeds. It validates them and gates downstream contract logic.

`oracle-watch` is the off-chain companion service that monitors SDEX trade flow via Horizon, aggregates volume + unique-trader counts, and submits signed liquidity snapshots to `LiquidityRegistry`. Operator-run; supports five pluggable webhook sinks for alert dispatch — Discord, Telegram, Slack, PagerDuty (Events API v2 with dedup-key), and a Generic sink for arbitrary HTTPS endpoints — all via the `WebhookSink` trait. See [`DEPLOYMENT.md`](DEPLOYMENT.md) for setup.

### Crate Layout

| Crate | Purpose |
|-------|---------|
| `safe-oracle` | The 5-guardrail library (rlib). Stateless — storage lives in the calling contract. |
| `liquidity-registry` | On-chain attestation contract. Stores SDEX volume snapshots written by whitelisted attesters. |
| `oracle-watch` | Off-chain Rust service. Polls Horizon, aggregates, signs, submits snapshots. |
| `mock-reflector` | Test/dev SEP-40 Reflector mock with `set_price` for adversarial scenarios. |
| `mock-lending` | Reference integrator showing the 8-line `safe-oracle` integration pattern. |
| `test-utils` | Shared `TestEnv` harness used across the workspace. |

---

## Adversarial Review

`safe-oracle` underwent independent adversarial review (AR.H) attempting 20+ distinct attack vectors across all five guardrails. The Hardening Phase closed 18/19 tracked debts before Phase 6 began. Key results:

| Severity | Count | Status |
|----------|-------|--------|
| Critical | **0** | — |
| High | **0** | — |
| Medium | 3 | All closed |
| Low | 5 | All closed (cap, validation, doc, test annotation, integrator warning) |
| Info | 10 | Documented |

**L4 (post-Phase 8 closure):** the bare `#[should_panic]` in the liquidity-registry unauthorized-signer test was replaced with an explicit `expected` message using the stable Soroban error code (`HostError: Error(Context, InvalidAction)`) — precise failure-mode verification, resilient to SDK message-format changes.

Notable closures:
- **M1**: `min_liquidity_usd == 0` silently disabled Layer 2 → `validate()` rejects 0
- **L1**: `circuit_breaker_halt_ledgers` unbounded → capped at `MAX_CIRCUIT_BREAKER_HALT_LEDGERS` (~1 week)
- **L2**: secondary oracle decimals mismatch → `DecimalsMismatch` enforced at library level
- **Debt #14**: redundant Reflector RPC round-trip → folded `records=1` into `records=2` fetch

All findings are documented in module-level doc-comments referencing the AR.H ID (`AR.H M1 fix:`, etc.) for audit traceability.

---

## Project Status

| Phase / Milestone | Status | Test Count |
|-------------------|--------|------------|
| Phase 1: Workspace + CI | ✅ Complete | — |
| Phase 2: Layer 1 guardrails | ✅ Complete | 30 |
| Phase 3: LiquidityRegistry contract | ✅ Complete | 60 |
| Phase 4: Layer 2 + e2e attack scenarios | ✅ Complete | 95 |
| Phase 5: Circuit breaker | ✅ Complete | 122 |
| Phase 5.5: Hardening + AR.H closure | ✅ Complete | 168 |
| Phase 6: `oracle-watch` off-chain service | ✅ Complete | 268 |
| Phase 7: Testnet deployment + e2e validation | ✅ Complete | 290 |
| Phase 8: Public web site | ✅ Complete | 290 |
| Post-Phase 8: Sinks + scenarios + AR.H L4 closure | ✅ Complete | 310 |
| **v0.2.0 — crates.io release** | ✅ **Published** | **310** |
| Mainnet deployment | ⏳ Planned | — |

---

## Building & Testing

Requires stable Rust (MSRV 1.85; developed/tested on 1.95) and the `wasm32v1-none` target.

```bash
cargo build --workspace
cargo test --workspace
```

Targeted demos:

```bash
# Phase 4 — adversarial e2e attack scenarios
cargo test --test e2e_attack_scenarios

# Phase 5.4 v2 — auto-halt regression test
cargo test --test integration -p mock-lending test_borrow_circuit_breaker_opens
```

---

## Documentation

- [`DEPLOYMENT.md`](DEPLOYMENT.md) — Operator + integrator guide. Includes adversarial replay + stale oracle reproduction steps.
- [`deployment/testnet.json`](deployment/testnet.json) — Complete deployment artifact with all tx hashes.
- [`CHANGELOG.md`](CHANGELOG.md) — Version history (Keep a Changelog format).

---

## License

Apache License 2.0. See [LICENSE](./LICENSE).

---

## Author

[@Sahveli01](https://github.com/Sahveli01)

Built for [Stellar Soroban](https://stellar.org/soroban). Oracle integration via [Reflector Network](https://reflector.network/).
