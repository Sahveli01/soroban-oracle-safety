# safe-oracle

[![crates.io](https://img.shields.io/crates/v/safe-oracle.svg)](https://crates.io/crates/safe-oracle)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Tests](https://img.shields.io/badge/tests-310%20passing-brightgreen)](https://github.com/Sahveli01/soroban-oracle-safety)

**Drop-in oracle protection for Stellar Soroban.**

`safe-oracle` wraps your existing Reflector oracle calls with five mathematically-verified guardrails against oracle manipulation attacks. Adversarially reviewed (0 critical, 0 high), validated end-to-end on Stellar testnet.

## Quick Start

```toml
[dependencies]
safe-oracle = "0.2"
soroban-sdk = "25.3"
```

```rust
use safe_oracle::{lastprice, Asset, SafeOracleConfig};
use soroban_sdk::{Address, Env};

let result = lastprice(
    &env,
    &asset,
    &reflector_address,
    &registry_address,
    &SafeOracleConfig::default(),
);

let price = result.into_result()?;
// `price` has passed all 5 guardrails. Use it.
```

## Five Guardrails

| Layer | Guardrail | Catches |
|-------|-----------|---------|
| 1 | Deviation | Sudden price spikes (default 2000 BPS) |
| 1 | Staleness | Outdated feeds (default 300s / 900s previous) |
| 1 | Cross-Source | Disagreement between primary and secondary oracles |
| 2 | Liquidity | Thin SDEX 30-minute volume (default $10k USD) |
| 2 | Thin Sampling | Low trader diversity (default 5 unique traders / 1h) |

Plus: a per-asset circuit breaker that auto-halts on first violation.

## Live on Stellar Testnet

- LiquidityRegistry: `CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND`
- 17 successful oracle-watch attestations
- 1 successful borrow validated end-to-end
- Adversarial replay (10× price spike) rejected (`ExcessiveDeviation`)
- Stale oracle scenario (48h-old timestamp) rejected (`StaleData`)

## Documentation

Full documentation, architecture, deployment guide, adversarial review summary:

- **Project site:** <https://soroban-oracle-safety.vercel.app>
- **Repository:** <https://github.com/Sahveli01/soroban-oracle-safety>
- **DEPLOYMENT.md:** integrator + operator guide
- **deployment/testnet.json:** complete deployment artifact with all on-chain tx hashes

## License

Apache License 2.0.
