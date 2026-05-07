# Phase 7 Pre-Phase Discovery — Empirical Findings

**Date:** 2026-05-07
**Branch:** main
**HEAD:** 9942c8059b5037f549a7d3527c3e6b29106e4a94
**Tag:** phase-6-complete

Read-only audit before Phase 7 implementation. No code changes. All facts in
this document were captured by direct command output, not memory or assumption.
"BİLMİYORUM" appears where empirical answer was unavailable.

---

## 1. Repo State

- **HEAD:** `9942c80` ✓ (matches expected)
- **Last tag:** `phase-6-complete` ✓
- **Working tree:** clean ✓
- **Test count:** **268 PASS / 0 FAIL / 5 ignored** (sum across 22 test binaries)
- **Clippy:** clean (`cargo clippy --workspace --all-targets -- -D warnings` exits 0
  with no warnings; final line: `Finished dev profile`)

No deviation from Phase 6 closure state.

---

## 2. Stellar CLI + Network

- **Version:** `stellar 25.1.0` (commit `a048a57a75762458b487052e0021ea704a926bee`)
- **stellar-xdr CLI bundle:** `25.0.0`
- **xdr curr:** `0a621ec7` (Protocol 26 era)
- **Networks configured:** `local, futurenet, mainnet, testnet` ✓ (testnet pre-existing)

**Divergence note:** Phase 1 target was `26.0.0`, current is `25.1.0`. CLI 25.1.0
talks Protocol 26 (xdr curr matches), so it is functionally correct for current
testnet. Whether to upgrade to 26.x before Phase 7.5 (build/deploy) is a chat-side
decision. Decisions to be made empirically when WASM build is attempted.

---

## 3. Endpoint Reachability

All three endpoints reachable with HTTP 405 (Method Not Allowed) for HEAD requests
— normal for POST-only RPC endpoints:

| Endpoint                          | Status |
|-----------------------------------|--------|
| https://friendbot.stellar.org     | 405    |
| https://horizon-testnet.stellar.org | 405  |
| https://soroban-testnet.stellar.org | 405  |

405 confirms the host serves but rejects HEAD; sufficient for reachability.

---

## 4. Soroban RPC API Shape

### `getNetwork`

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "friendbotUrl": "https://friendbot.stellar.org/",
    "passphrase": "Test SDF Network ; September 2015",
    "protocolVersion": 26
  }
}
```

**Implication for Phase 7.1:** envelope construction must use passphrase
exactly `"Test SDF Network ; September 2015"` for signing. Protocol = 26.

### `getLatestLedger`

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "id": "d7527387505451abea7eb61deff53e303c693ceeb0497f2d2872cadada01defe",
    "protocolVersion": 26,
    "sequence": 2437910,
    "closeTime": "1778197474",
    "headerXdr": "<base64 ledger header>",
    "metadataXdr": "<base64 ledger metadata>"
  }
}
```

**Implication:** ledger sequence at probe time = 2437910. This will move; what
matters is the response shape — `result.sequence` (u32 as JSON number) is what
oracle-watch will use to bind transaction validity windows.

---

## 5. stellar-rpc-client Crate Status

Empirical `cargo search` results (as of 2026-05-07):

| Crate                  | Latest version    | Status       |
|------------------------|-------------------|--------------|
| stellar-rpc-client     | `26.0.0-rc.2`     | **STILL RC** |
| stellar-xdr            | `26.0.1`          | stable       |
| soroban-client         | `0.5.6`           | third-party (not Stellar Foundation) |
| rs-soroban-client      | `0.1.0`           | third-party |
| soroban-rpc            | `20.3.3`          | outdated (Protocol 20) |
| soroban-sdk            | `26.0.0`          | stable (in use) |
| ed25519-dalek          | `3.0.0-pre.7`     | pre-release |

**Decision:** Phase 6.0 drop of stellar-rpc-client still stands — only an RC is
available, no stable 26.x. Phase 7.1 envelope construction will use `stellar-xdr 26.0.1`
(stable) directly + manual JSON-RPC POST (consistent with Phase 6 approach).

**Open question for chat:** is the `soroban-client = 0.5.6` (third-party) worth
evaluating as a higher-level wrapper over raw RPC, or do we stay with hand-rolled
to avoid third-party dep risk?

---

## 6. Reflector Testnet Address

Empirical scan results:
- **REFERENCES.md:** mainnet info only — `decimals=14`, `resolution=300s`, repo URL,
  website. **No testnet address.**
- **CLAUDE.md:** mentions Reflector in two lines (project description, free function
  pattern). **No testnet address.**
- **No grep hit for `C[A-Z0-9]{55}` Reflector address constants in source.**

**Status:** **BİLİNMİYOR** — Reflector testnet adresi kodda referans yok.
Chat-side empirical web research required before Phase 7.5.

**Fallback option:** deploy `mock-reflector` on testnet, point safe-oracle at it.
This is sufficient for E2E borrow + adversarial replay tests since the mock
implements the same interface (`lastprice`, `lastprices`, `decimals`, `resolution`).
Real Reflector integration could be Phase 7.7 stretch goal.

---

## 7. WASM Target

```
$ rustup target list --installed
x86_64-pc-windows-msvc
```

**Empirical finding: NO wasm targets installed.** `wasm32v1-none` is available
in the toolchain (`rustup target list` shows it among the unselected) but **not
yet added** to the local toolchain.

```
$ rustc --version
rustc 1.91.0 (f8297e351 2025-10-28)
```

`cargo check --target wasm32v1-none` was **not run** because target install
is required first. Phase 7.5 must begin with:

```
rustup target add wasm32v1-none
```

**No project-local pin file** (`rust-toolchain.toml`, `.cargo/config.toml`) exists,
so target add will be system-wide for this rustc.

---

## 8. Borç Envanteri

### 8.1 TODO/FIXME (full inventory)

```
mocks/mock-lending/src/lib.rs:232:        // TODO: extend_ttl in production
mocks/mock-lending/src/lib.rs:242:        // TODO: extend_ttl in production
mocks/mock-reflector/src/lib.rs:69:        // TODO: extend_ttl in production
mocks/mock-reflector/src/lib.rs:94:        // TODO: extend_ttl in production
mocks/mock-reflector/src/lib.rs:115:       // TODO: extend_ttl in production
crates/liquidity-registry/src/lib.rs:295:  // TODO: extend_ttl in production (Phase 8 deployment).
crates/liquidity-registry/src/lib.rs:331:  // TODO: extend_ttl on read in production (Phase 8 deployment).
crates/oracle-watch/src/main.rs:236:       "base_account": "GACCT1XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"  (placeholder in test JSON)
crates/oracle-watch/src/monitor.rs:166:    // TODO Phase 6.7: dispatch_alerts(&anomalies, &alert_config).await
```

**Note:** `monitor.rs:166` mentions "Phase 6.7" — but Phase 6.7 IS the alert dispatch
work (per Phase 6 commit history: `729407a feat: oracle-watch alert dispatch with
WebhookSink trait (Phase 6.7)`). This TODO appears to be **stale** — likely already
implemented by Phase 6.7 commit, just not cleaned up in line 166. Needs verification
in Phase 7 cleanup.

### 8.2 Phase 7/8/deferred mentions (full inventory)

```
mocks/mock-lending/src/lib.rs:367:    /// Phase 7 may relocate these primitives to a separate sibling crate
crates/oracle-watch/src/config.rs:43:    /// Phase 8 will replace this with a real-time…
crates/safe-oracle/src/lib.rs:110:   /// (Hardening 6C finding, deferred).** Independent of the
crates/safe-oracle/src/lib.rs:127:   /// remains deferred for future SDK releases that resolve constraint (2).
crates/safe-oracle/src/lib.rs:262:   /// Phase 7 reconciliation plan.
crates/safe-oracle/src/lib.rs:581:   // (debt #13, deferred to Phase 8).
crates/safe-oracle/src/lib.rs:713:   /// Phase 7 will add a configurable `previous_max_staleness_seconds`
crates/safe-oracle/src/lib.rs:820:   /// Phase 7 will add a one-time `decimals()` call at first read…
crates/oracle-watch/src/main.rs:126: /// submission Phase 8)
crates/oracle-watch/src/main.rs:145: /// Phase 8 will pre-filter…
crates/oracle-watch/src/main.rs:162: /// Phase 8 will pre-filter…
crates/oracle-watch/src/main.rs:185: /// Phase 6.5 stub — Phase 8 wires real submission
crates/oracle-watch/src/main.rs:190: /// Phase 8 will add: registry_writer.submit_transaction_stub(envelope_xdr)
crates/oracle-watch/src/registry_writer.rs:19:  Real testnet/mainnet connectivity is **Phase 8 work**
crates/oracle-watch/src/registry_writer.rs:45:  `Sign` and `Rpc` variants are reserved for Phase 8
crates/oracle-watch/src/registry_writer.rs:91:  Held for Phase 8 transaction-envelope signing
crates/oracle-watch/src/registry_writer.rs:179: integration is Phase 8 — testnet account funding
crates/oracle-watch/src/registry_writer.rs:182: until Phase 8 wiring
crates/oracle-watch/src/registry_writer.rs:246: Phase 8 may refine to precise enum-discriminant XDR
crates/oracle-watch/src/registry_writer.rs:278: Phase 8 may extend with explicit
crates/oracle-watch/src/monitor.rs:170:    /// volume-trend or trade-count-delta detection in Phase 7+
crates/oracle-watch/src/monitor.rs:213:    /// Phase 7+ may use this to refine thresholds
crates/oracle-watch/src/monitor.rs:240:    /// Phase 8 if real…
crates/oracle-watch/src/monitor.rs:297:    /// concurrent dispatch is Phase 8 if real
crates/oracle-watch/src/signer.rs:51:        /// Phase 8 deployment must handle key provisioning
crates/oracle-watch/src/signer.rs:120:       /// Phase 8 cross-verification
crates/oracle-watch/src/types.rs:30:          // downstream Phase 8 work (dedup by id…)
crates/oracle-watch/src/types.rs:65:          #[allow(dead_code)] // consumed by Phase 8 precision-sensitive paths
crates/liquidity-registry/src/lib.rs:243:     /// production deployments must call `extend_ttl` here (Phase 8 deployment work)
crates/liquidity-registry/src/lib.rs:295:     // Phase 8 deployment.
crates/liquidity-registry/src/lib.rs:325:     /// (Phase 8 deployment work)
crates/liquidity-registry/src/lib.rs:331:     // Phase 8 deployment.
```

**Critical terminology note:** the codebase uses "Phase 8" to mean what the
**new** roadmap calls Phase 7 (testnet deployment). When Phase 7 (Adversarial
Test Suite) was retroactively considered "covered" by existing tests, the old
Phase 8 became the new Phase 7. Therefore:
- Code says "Phase 8 deployment work" → **THIS IS NOW PHASE 7**.
- Code says "Phase 7 will add X" → these are independent feature additions
  scheduled for the testnet phase (still relevant for current Phase 7).

This dual usage will likely confuse future readers; the cleanup of these
references is a candidate for Phase 7.10 (docs) or a small hygiene commit.

### 8.3 "Phase 7 Reconciliation Plan" (lib.rs:262)

**Location:** `crates/safe-oracle/src/lib.rs:252-262` — doc-comment on
`SafeOracleConfig.secondary_oracle` field.

**Content (paste):**
```
    /// Optional secondary oracle for cross-source price verification.
    /// `None` skips the cross-source guardrail entirely (single-source mode);
    /// `Some(addr)` activates `check_cross_source` against the configured
    /// `max_cross_source_bps` threshold.
    ///
    /// **Integrator warning (AR.H M2):** the secondary must report prices in
    /// the same decimal precision as the primary (Reflector mainnet = 14
    /// decimals). A decimals-mismatched secondary produces always-fires
    /// `CrossSourceMismatch` because BPS arithmetic is unscaled `i128` math.
    /// See `check_cross_source` doc-comment for full rationale and the
    /// Phase 7 reconciliation plan.
```

**Cross-reference (lib.rs:815-823):**
```
    /// **Integrator responsibility:** verify that the secondary oracle reports
    /// in the same decimal precision as the primary. The cross-source
    /// guardrail is currently safe to use only with same-precision pairs.
    ///
    /// Phase 7 will add a one-time `decimals()` call at first read to verify
    /// primary/secondary precision agreement, with `CrossSourceMismatch` (or
    /// a new error variant) returned if they disagree. Until then, this is
    /// an integrator-side configuration concern.
```

**Yorum (Terminal Claude):** "Phase 7 reconciliation plan" = a one-time
`decimals()` call at first cross-source read, comparing primary vs secondary
oracle decimals. If mismatch → return `CrossSourceMismatch` (or new variant).
Currently this is left as integrator responsibility with only a doc warning.

This is **Phase 7.3** territory per the roadmap. Implementation needs:
1. New storage key for "decimals already verified" (per-secondary-oracle).
2. First-read fetch of `primary.decimals()` and `secondary.decimals()`.
3. Mismatch error variant or reuse of `CrossSourceMismatch`.
4. Tests covering same/diff precision pairs.

### 8.4 Debt #13 (lib.rs:581)

**Location:** `crates/safe-oracle/src/lib.rs:573-587` — comment in
`fetch_with_validation` (or similar pre-validation helper).

**Content (paste):**
```
    // 1. Fetch newest + previous prices in a single cross-contract call.
    //
    // Hardening Phase debt #14: pre-6A this path issued two reads —
    // `records=1` here for `current`, then `records=2` again inside
    // `check_deviation` for the previous price. The records=2 fetch
    // already returns both, so the records=1 call was redundant; folding
    // it eliminates one Reflector round-trip per Layer 1 evaluation.
    // Actual gas savings will be measured during testnet deployment
    // (debt #13, deferred to Phase 8).
```

**Yorum (Terminal Claude):** debt #13 = **measure actual gas savings** from
the records=1 → records=2 fold (debt #14 fix during Hardening 6A). The
measurement requires testnet deployment because in-mock simulation doesn't
produce realistic gas figures.

This is **trivial Phase 7.7 work** — a measurement, not a code change.
Just call the existing path on testnet, measure cost, document. Does **not**
gate Phase 7 progress.

### 8.5 extend_ttl Eksikleri (Empirical Inventory)

**Total: 7 missing locations across 3 files:**

| File | Line | Context |
|------|------|---------|
| `mocks/mock-reflector/src/lib.rs` | 69 | after `lastprice` persistent get |
| `mocks/mock-reflector/src/lib.rs` | 94 | inside `lastprices` history loop |
| `mocks/mock-reflector/src/lib.rs` | 115 | after `set_price` history push |
| `mocks/mock-lending/src/lib.rs` | 232 | after `initialize` set |
| `mocks/mock-lending/src/lib.rs` | 242 | after balance update set |
| `crates/liquidity-registry/src/lib.rs` | 295 | after `write_snapshot` set |
| `crates/liquidity-registry/src/lib.rs` | 331 | after read `get` |

**Phase 7.2 (Soroban persistence hardening)** target: add `extend_ttl` calls at
all 7 sites with appropriate threshold/extend-to values. Spec values to be
confirmed empirically against Soroban docs (TTL constants vary by storage type
and use case).

**Implementation note:** mocks may not strictly need extend_ttl since they only
run in tests, but consistency + future re-use as production-style references
favor adding them. This is a chat-side decision — minimal scope says "real
contract only" (just the 2 in liquidity-registry), thorough scope says all 7.

### 8.6 previous_max_staleness_seconds + decimals() Notes

**previous_max_staleness_seconds (lib.rs:713):**
```
/// Phase 7 will add a configurable `previous_max_staleness_seconds`
/// (or reuse `max_staleness_seconds * K` for some K=2..5) so post-gap
/// movement is correctly classified as `StaleData` rather than
/// `ExcessiveDeviation`.
```

**decimals() (lib.rs:820):** see §8.3 above (same item from a different angle).

**Phase 7.3 verdict:** both are scheduled for Phase 7.3 in the roadmap. They
are **proper code changes**, not just measurements. Test coverage will need
extension. Estimated complexity: small to medium.

**Open question for chat:** is `previous_max_staleness_seconds` a separate
config field (clearer, more memory in storage) or a derived `K * max_staleness_seconds`
(less storage, less flexible)? Leaning toward separate field for explicitness,
but defer to chat-side architectural call.

---

## 9. Web/ Klasörü

- **`web/`:** does not exist (expected — Phase 8 territory).
- **`.gitignore`:** present, already references "Phase 8'de kullanılacak"
  for `deployment/secrets/` and `*.key`. Will need `web/node_modules` added
  in Phase 8.
- **No action required** in Phase 7.

---

## 10. Phase 7 Önerilen Sıralama (Terminal Claude'un Bakışı)

The roadmap order (7.1 → 7.10) is reasonable, but I'd suggest one re-ordering:

| New order | Original | Rationale |
|-----------|----------|-----------|
| **7.1** Soroban persistence hardening (extend_ttl) | was 7.2 | Pure code; do BEFORE deploy so no re-deploy needed |
| **7.2** safe-oracle design debts (decimals + staleness) | was 7.3 | Pure code; same reasoning |
| **7.3** Real submission path (envelope + RPC submit) | was 7.1 | Code change in oracle-watch; can validate locally before testnet |
| **7.4** Testnet identities + network | unchanged | Sets up funded accounts |
| **7.5** WASM build + contract deploy | unchanged | First step needs `rustup target add wasm32v1-none` |
| **7.6** Contract initialization | unchanged | Calls deployed contracts |
| **7.7** oracle-watch testnet run | unchanged | Includes debt #13 gas measurement |
| **7.8** E2E successful borrow | unchanged | |
| **7.9** E2E adversarial replay | unchanged | |
| **7.10** README + DEPLOYMENT.md (+ stale Phase 7/8 doc cleanup) | unchanged + scope add | |

**Rationale for code-first:** any extend_ttl or decimals-check change to
liquidity-registry / safe-oracle requires re-deploying contracts. Deploying
once at 7.5 with all code changes already in is more efficient.

**Counterargument:** if 7.5 deploys go badly and we discover we need to redo
contract layout anyway, the order is wash. But empirically, fewer iterations
of WASM build + deploy is better.

This is a **chat-side decision** — both orderings work.

---

## 11. Bilinmeyenler

1. **Reflector testnet contract address** — not in REFERENCES.md, not in source.
   Chat-side web research needed (or fall back to mock-reflector deploy).
2. **Stellar CLI 25.1.0 vs 26.0.0** — whether 25.1.0 is sufficient for Phase 7.5
   `stellar contract deploy` against testnet, or 26.x upgrade is required.
   To be tested empirically when WASM build attempted.
3. **wasm target choice** — is `wasm32v1-none` the canonical Soroban 26 target,
   or `wasm32-unknown-unknown`? Soroban SDK 26.0.0 docs to be checked at 7.5.
4. **Friendbot funding amount** — 10000 XLM default per request, but multiple
   identities (oracle-watch attester, deployer, lender, borrower) needed.
5. **soroban-client 0.5.6 (third-party)** — whether to evaluate as RPC wrapper
   for envelope submission, or stay with hand-rolled JSON-RPC. Phase 6.0 favored
   hand-rolled; same reasoning may stand.
6. **TTL extend values** — what's the recommended threshold/extend-to for
   `liquidity-registry` snapshots given they update every 5 minutes?
7. **`monitor.rs:166` stale TODO** — is this actually stale (Phase 6.7 done) or
   does it refer to a different capability not yet implemented? Needs chat
   review.

---

## 12. Empirik Endişeler (Concerns)

1. **Phase 7/Phase 8 terminology drift.** The codebase has 30+ "Phase 8"
   references for what the new roadmap calls Phase 7. This is not a blocker
   but creates ongoing reader confusion. Mass rename in 7.10 is recommended
   but adds a sizeable diff late in the phase.

2. **WASM target not installed.** Phase 1 was supposed to set this up. Either
   the install was never persisted, or rustc 1.91.0 (October 2025 nightly?)
   reset the target list. Easy fix (`rustup target add`) but a sign that
   environment isn't fully reproducible without a `rust-toolchain.toml`.

3. **Stellar CLI 25.1.0** lags expected `26.0.0`. Functionally compatible with
   Protocol 26 (xdr current is at 26 spec), but version skew may cause subtle
   issues with newer `stellar contract deploy` flags.

4. **stellar-rpc-client still RC.** Phase 6.0 dropped this; nothing has changed
   in 2+ months. Suggests Stellar Foundation is comfortable shipping RC
   indefinitely. Hand-rolled JSON-RPC remains the right call.

5. **Stale `Phase 6.7` TODO at `monitor.rs:166`.** Likely a forgotten cleanup;
   indicates that prior phases left small debris. Phase 7 should include a
   pass-by cleanup as part of 7.10 hygiene.

6. **`base_account` placeholder** in oracle-watch test JSON
   (`crates/oracle-watch/src/main.rs:236`) — `"GACCT1XXXX..."`. Not a real
   address, just a test fixture string, but worth confirming during Phase 7
   that no production path consumes this string.

7. **`monitor.rs:170` and `monitor.rs:213` mention "Phase 7+" expansions**
   (volume-trend / trade-count-delta detection). These are FUTURE features,
   NOT current Phase 7 commitments. Should not be implemented unless explicitly
   re-scoped by chat.

---

## 13. Test Suite Decomposition (sanity)

For audit reproducibility, the 268 passing tests breakdown (across crates):

| Crate (rough)     | Tests |
|-------------------|-------|
| largest binary    | 100   |
| sub-binaries      | 26, 22, 21, 20, 19, 12, 7, 7, 5, 5, 5, 5, 5, 5, 2, 2 |
| **Total PASS**    | **268** |
| ignored           | 5     |

Final test count matches Phase 6 closure expectation.

---

## END OF DISCOVERY

This document is reference-only. Phase 7.1+ implementation prompts will cite
specific sections (e.g., "see §8.5 extend_ttl inventory" or "per §6 Reflector
testnet still unknown — falling back to mock-reflector"). Do not edit findings
above retroactively; create new sections in subsequent phase docs if state
changes.
