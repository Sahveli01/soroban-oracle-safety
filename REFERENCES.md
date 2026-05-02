# Canonical References

This document lists authoritative external sources used during the development of soroban-oracle-safety. When investigating a Soroban API, version compatibility, or design pattern, consult these sources rather than relying on memory or training data.

## Stellar / Soroban Core

### SDK
- **soroban-sdk 25.3.1** (current production target)
  - Docs: https://docs.rs/soroban-sdk/25.3.1/soroban_sdk/
  - Release notes: https://github.com/stellar/rs-soroban-sdk/releases
  - Migration guides: https://docs.rs/soroban-sdk/25.3.1/soroban_sdk/_migrating/

### Network
- **Stellar Protocol 25** (mainnet, Jan 22, 2026)
  - Software versions: https://developers.stellar.org/docs/networks/software-versions
  - Network passphrase: `Public Global Stellar Network ; September 2015` (mainnet), `Test SDF Network ; September 2015` (testnet)

### Tooling
- **Stellar CLI 26.0.0** (used in CI)
  - Repo: https://github.com/stellar/stellar-cli
- **WASM target**: `wasm32v1-none` (Soroban-specific, not legacy `wasm32-unknown-unknown`)
- **Rust edition**: 2024

### Documentation
- Build smart contracts: https://developers.stellar.org/docs/build/smart-contracts
- Best practices: https://developers.stellar.org/docs/build/smart-contracts/best-practices
- Soroban examples: https://github.com/stellar/soroban-examples

### AI Skill
- **Stellar Dev Skill**: https://github.com/stellar/stellar-dev-skill
- Installed at: `C:\Users\sahve\.claude\skills\stellar-dev\`
- Building with AI: https://developers.stellar.org/docs/tools/developer-tools/building-with-ai

## Reflector Oracle

The price feed this library protects.

- **Repo**: https://github.com/reflector-network/reflector-contract
- **Website**: https://reflector.network/
- **Mainnet decimals**: 14
- **Mainnet resolution**: 300 seconds (5-minute ticks)
- **Interface used by safe_oracle**: `lastprice(asset)`, `lastprices(asset, records)`, `decimals()`, `resolution()`

## Standards Referenced

### CAPs (Core Advancement Proposals)
- **CAP-0058** — Contract constructors (Protocol 22+, opt-in for now)
- **CAP-0053** — TTL extension behavior
- (Full list: https://github.com/stellar/stellar-protocol/tree/master/core)

### SEPs (Stellar Ecosystem Proposals)
- **SEP-0040** — Price oracle interface (what Reflector implements)
- **SEP-0048** — Contract interface specification
- **SEP-0049** — Upgradeable contracts (Phase 5+ relevance)
- (Full list: https://github.com/stellar/stellar-protocol/tree/master/ecosystem)

## Threat Model References

### YieldBlox Incident (Feb 22, 2026)
The attack this library defends against. $5 trade in thin SDEX liquidity manipulated Reflector's price feed → $10.2M stolen from YieldBlox/Blend lending pool. Root cause: integrator-side guardrails were absent; Reflector itself functioned correctly.

- (Add post-mortem links here when published)

## Project Files (Outside Repo)

These files are private and intentionally not in version control:

- **Specification**: `C:\SCF41\soroban-oracle-safety-spec.md`
  - Authoritative design document
  - Contains: 5 guardrail definitions, SafeOracleConfig schema, threat model
- **Implementation Roadmap**: `C:\SCF41\IMPLEMENTATION_ROADMAP.md`
  - 80-prompt phased implementation plan
  - 8 phases, with checkpoint and adversarial review prompts

## Stellar Community Fund

This project targets SCF Build Award funding.

- **SCF**: https://stellar.org/community/fund
- **Award range**: up to $150,000 in XLM
- **Round cadence**: every 4 weeks

## License

This project is **Apache 2.0** licensed. See `LICENSE` file in repo root.

---

*This file is updated when canonical sources change (e.g., soroban-sdk version bump, new SEP referenced). Maintained as part of the project documentation, not auto-generated.*
