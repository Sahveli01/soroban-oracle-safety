# Phase 8 Pre-Phase Discovery — Empirical Findings

**Date:** 2026-05-08
**HEAD:** 4215fb9 (Phase 7.10)
**Tag:** phase-7-complete (annotated, pushed to origin)
**Working tree:** clean

---

## 1. Repo State

- Test count: **290/290 PASS** (unchanged from Phase 7 closure)
- Working tree: clean (`nothing to commit, working tree clean`)
- Tag `phase-7-complete`: present, points to 4215fb9
- All Phase 7 commits pushed to `origin/main`

---

## 2. Web Stack Empirical (npm view, 2026-05-08)

### Next.js
- **Latest stable:** `16.2.6`
- **Beta:** `16.0.0-beta.0`
- **Decision:** use Next.js **16.2.6** — App Router, RSC, stable

### React
- **Latest:** `19.2.6`
- **Decision:** **19.2.6** — required by Next 16

### Tailwind CSS — *the question*
- **Latest stable:** `4.2.4` (dist-tag `latest`)
- **v3 status:** `3.4.19` demoted to dist-tag `v3-lts`
- **v4 status:** **production-stable as of `latest`**, no longer alpha/beta
- **Decision:** use Tailwind **v4.2.4** — CSS-first config (`@theme`), no `tailwind.config.js`
- *Rationale:* v4 has graduated; prompt template's "v3.4 fallback" path is unnecessary

### Framer Motion
- **Latest:** `12.38.0`
- **Decision:** **12.38.0**

### Lenis
- **Latest:** `1.3.23` (package name `lenis`, formerly `@studio-freight/lenis`)
- **Decision:** **1.3.23**

### Geist Font
- **Latest:** `1.7.0`
- **Decision:** **1.7.0** — Vercel's official font, native Next.js integration via `next/font`

### shadcn/ui
- **Latest:** `4.7.0`
- **Decision:** copy-paste pattern, no full install (minimal component surface)

---

## 3. Vercel CLI

- **Installed:** **NO** (`vercel: command not found`)
- **Required action:** none — deployment will go through `vercel.com/new` GitHub integration
- *Rationale:* keeps deploy path low-friction (no local CLI auth, no shell-side secrets); the dashboard auto-detects Next.js, sets up build/preview environments, and connects to the GitHub repo for CI-driven deploys

---

## 4. Phase 7 Live Data

### Contract Addresses (verified against `deployment/testnet.json`)

| Contract | Address |
|----------|---------|
| LiquidityRegistry | `CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND` |
| mock-lending (Phase 7.8 re-deploy) | `CA6TBUXTIQKHD4VZ3MMQTJTTREMHHYQD4G6R3OTOOVGHOGQNXUYSMXZV` |
| mock-reflector | `CBUPTLPDDNCB2OHTGTHD3DKHLGSZUDUMINU5OKU4CG5ZRHW5T7ATPHO7` |
| mock-lending (Phase 7.5 deprecated) | `CBXSYSZKW5K66PBVZ5QFH73XT4DWX6JLPDZWHIZUZDHHH454NBSRNSAY` |

### E2E Validation Tx Hashes (Phase 7 closure)

| Phase | Action | Tx Hash | Ledger |
|-------|--------|---------|--------|
| 7.7 | First oracle-watch submission | `cf4ecc2805c4355cd319b61dfc09aed719229c5b2c1ad5b804bc2f2099553c36` | 2,448,975 |
| 7.8 | Successful borrow | `ce4812031daa61ecb987c45123fbaba52eb83fe0b27f623dd3fa3fa0ec8a5c45` | 2,450,314 |
| 7.9 | Attack set_price (10× spike) | `b99d61340c63748394f27a589ac228bbc6a02aba7d74c5b50b67a416ee6acfb6` | — |
| 7.9 | Adversarial borrow (rejected) | `a1cfdec1fe8f6c778c0f6f48f481c0b7dfd31ea7322834d84944459ca80a7653` | — |
| 7.9 | Recovery set_price | `9cae263874ab308ccba3871bc00aeec95dbff0199e2e7187c71c1ecf1bba378f` | — |
| 7.9 | Post-recovery borrow | `5f5d06e4822e3a1b061513acfd159958105e2a7f7f1ab15a5337fa8ab10aec55` | — |

### Site Stats

- Tests: **290** passing
- Submissions: **17** (Phase 7.7 continuous run)
- Critical findings: **0**
- High findings: **0**
- Medium findings: **3** (all closed during Hardening Phase + AR.H closure)
- Low findings: **5**
- Info findings: **10**
- Live testnet bug fixes during Phase 7: **6**
- Public on-chain tx hashes (deploy + init + e2e): **25+**

---

## 5. Bilinmeyenler (Open Questions)

- Domain: starts on Vercel subdomain (`safe-oracle.vercel.app` or similar). Custom domain decision deferred to Phase 8.3.
- Demo replay strategy: deferred to Phase 8.3 — pre-recorded with the real Phase 7.9 tx data, scrubbed step-by-step animation. No live RPC calls from the static site.
- Hero copy variants: tagline "Trust the oracle. Verify the integrator." is locked. Subline + stats wording can iterate during 8.1.

---

## 6. Empirical Concerns

- **Tailwind v4 just graduated to stable.** Edge-case bugs may exist; if a v4-specific issue blocks 8.1 work, fall back to v3.4.19 (`v3-lts`). This is a contingency, not a default.
- **Next.js 16** is recent (8 months from May 2026 tagging). No immediate concerns; App Router is well-trodden territory.
- **No CI for the web project** — Vercel's build is the only verification. That is fine for a marketing site; we do not need a Rust-style 290-test workspace for a static page.

---

## 7. Phase 8 Approved Sub-Phase Sequence

| # | Title | Estimated time |
|---|-------|----------------|
| 8.0 | Pre-Phase Discovery (THIS) | 30–45 min |
| 8.1 | Bootstrap + Hero | 3–4 h |
| 8.2 | Content Sections | 5–6 h |
| 8.3 | Polish + Production Deploy | 3–4 h |

**Total estimated:** 12–15 hours.

---

## 8. Pointer to STACK.md

`web/STACK.md` is the source-of-truth artifact for visual decisions and tech selections. All subsequent Phase 8 prompts reference it. This `PHASE_8_PRE.md` is the empirical audit trail; STACK.md is the implementation specification.
