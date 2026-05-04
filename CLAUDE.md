# Project Context for Claude Code

This file is automatically read by Claude Code at the start of every session for this project. It contains project-specific rules, conventions, and current status.

## Project: soroban-oracle-safety

Open-source guardrails library for Stellar Soroban. Protects lending protocols against YieldBlox-class oracle manipulation attacks (Feb 2026: $5 trade → $10.2M stolen via Reflector price manipulation in thin SDEX liquidity).

Three components:
1. `safe_oracle` Rust library (5 guardrails)
2. `oracle-watch` off-chain service (SDEX trade attestation)
3. `LiquidityRegistry` on-chain contract (attestation storage)

## Current Phase Status

- ✅ **Phase 1** Complete: Workspace, CI, mock infrastructure (`01f589c`)
- ✅ **Phase 2** Complete: Layer 1 guardrails (deviation, staleness, cross-source) — tag `phase-2-complete`
- ✅ **Phase 3** Complete: LiquidityRegistry contract (auth, whitelist, snapshot, replay protection) — tag `phase-3-complete`
- ✅ **Phase 4** Complete: Layer 2 guardrails (liquidity, thin sampling) + e2e attack scenarios — tag `phase-4-complete`
- ✅ **Phase 5** Complete: Circuit breaker (auto-halt + governance override + Ok-API types) — tag `phase-5-complete`
- 🟡 **Phase 6** Starting: Audit + improvements (tracked debts cleanup)
- ⏳ Phase 7: SEP-Oracle-Safety standard draft
- ⏳ Phase 8: Testnet deployment

## Critical Rules — ALWAYS Apply

### Architecture
- `safe_oracle` is a **library** (rlib), not a contract. Calling contracts use it via path dependency, function call (not cross-contract call).
- Free function pattern: `safe_oracle::lastprice(env, asset, reflector, registry, config)` — no struct, no `&self`.
- Library is stateless; storage lives in calling contracts.

### Initialization Pattern
- **ALL** `initialize()` functions MUST include reinitialization protection:
```rust
  if env.storage().instance().has(&DataKey::Admin) {
      panic!("AlreadyInitialized");
      // OR return Err(YourError::AlreadyInitialized)
  }
```
- All current contracts (LiquidityRegistry from Phase 3.1, MockLending from Phase 2 carryover) include this. New contracts MUST include it from day one.

### Defensive Programming
- BPS arithmetic: ALWAYS use `checked_mul` to prevent overflow.
- Before division: validate divisor is positive (`previous.price > 0`).
- Before subtraction: validate non-negativity guarantees (e.g., timestamp ordering).
- Sanity check inputs: `current.price <= 0` → fail fast, treat as manipulation signal.

### Error Handling
- Each guardrail returns specific `OracleSafetyViolation` variant (1-7).
- Calling contracts (mock-lending) use transparent passthrough — same discriminants in their own error enum.
- This preserves granular error reporting for audit/debugging.

### Storage Types
- `instance` — config, admin, oracle addresses (long-lived, automatic TTL extension on contract use).
- `persistent` — user data, trade history (manual `extend_ttl` required in production).
- `temporary` — short-lived cache (rarely used).
- Production deployment: ALL persistent writes need `extend_ttl(MIN_TTL, EXTEND_TO)` (currently TODO in mocks).

### Testing
- Use `TestEnv` from `test-utils` crate. Never write tests with raw `Env::default()` if guardrails are involved.
- Integration tests in `tests/*.rs` directory (not `mod test` inline) — avoids circular dev-dependency issues.
- Soroban auto-generated `test_snapshots/` are committed to repo (deterministic regression detection).
- Test ledger timestamp baseline: `100_000` (set by `TestEnv::new()`).

### Versioning
- soroban-sdk: **25.3.1** (production stable, NOT 26.x rc)
- WASM target: `wasm32v1-none` (NOT legacy `wasm32-unknown-unknown`)
- Rust edition: 2024
- Stellar Protocol: 25 (mainnet, Jan 2026)
- Stellar CLI: 26.0.0 in CI (latest stable)

### Modern Soroban Patterns
- Contract registration: `env.register(C, ())` (NOT old `env.register_contract(None, C)`)
- Cross-contract: `#[contractclient(name = "FooClient")]` trait + `FooClient::new(env, &address).method(args)`
- Events: `#[contractevent]` struct with `#[topic]` field-level attribute
- Errors: `#[contracterror]` enum, contiguous u32 discriminants

## Conventions

### Commits
- Conventional commits: `feat:`, `fix:`, `refactor:`, `style:`, `test:`, `docs:`
- Each prompt's output gets one focused commit (granular history, audit-friendly).
- No pre-commit hooks (natural workflow signal).
- Tag major milestones: `phase-N-complete` annotated tags.

### Code Style
- `cargo fmt --all` before committing (CI enforces).
- `cargo clippy --workspace --all-targets -- -D warnings` clean (CI enforces).
- `#![no_std]` first line of every contract crate's lib.rs.
- Doc comments on public APIs explain WHY, not just WHAT.

### Project Files Location
- **Spec** lives OUTSIDE the repo: `C:\SCF41\soroban-oracle-safety-spec.md` (private, not committed)
- **Roadmap** lives OUTSIDE the repo: `C:\SCF41\IMPLEMENTATION_ROADMAP.md` (private)
- **Stellar Dev Skill** installed at: `C:\Users\sahve\.claude\skills\stellar-dev\` (active in Claude Code)

## Workflow

User (Sahveli01) drives implementation by:
1. Pasting roadmap prompt to chat Claude (review/edit)
2. Edited prompt given to terminal Claude Code
3. Output reviewed by chat Claude
4. Approved changes committed + pushed
5. CI verification before next prompt

User is the verifier. Decisions need explicit approval. No autonomous scope changes, no proactive feature additions.

## When in Doubt

- See `REFERENCES.md` for canonical sources (versions, docs, spec).
- Stellar Dev Skill (auto-loaded) covers general Soroban patterns; this file covers project-specific decisions.
- If skill recommendation conflicts with this file, this file wins (project decisions are explicit).
- If unsure about a Soroban API, web search docs.rs/soroban-sdk/25.3.1 before guessing.

## Reinitialization Protection — Mandatory for All initialize() Functions

When writing `initialize()` for any contract, always include reinitialization protection:
```rust
pub fn initialize(env: Env, admin: Address, /* ... */) -> Result<(), YourError> {
    if env.storage().instance().has(&DataKey::Admin) {
        return Err(YourError::AlreadyInitialized);
    }
    admin.require_auth();
    env.storage().instance().set(&DataKey::Admin, &admin);
    // ... rest of init
    Ok(())
}
```

**Status:**
- ✅ `LiquidityRegistry::initialize()` — implemented in Phase 3.1 (commit `d6c22cb`)
- ✅ `MockLending::initialize()` — implemented as Phase 2 carryover fix (Phase 4 prep)

This pattern was identified as missing in mock-lending during the post-Phase-2 audit (Medium severity). All new contracts MUST include it from day one.
