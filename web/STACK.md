# Web Site Stack — Phase 8

**Date:** 2026-05-08
**Phase:** 8.0 Pre-Phase Discovery
**Hero tagline (locked):** `Trust the oracle. Verify the integrator.`

---

## Tech Stack — Empirically Verified

| Tool | Version Selected | Latest Available | Rationale |
|------|------------------|------------------|-----------|
| Next.js | **16.2.6** | 16.2.6 | App Router, RSC, stable; `latest` dist-tag |
| React | **19.2.6** | 19.2.6 | Required by Next 16; stable |
| Tailwind CSS | **4.2.4** | 4.2.4 | v4 is now `latest` (v3 demoted to `v3-lts`); CSS-first config |
| Framer Motion | **12.38.0** | 12.38.0 | Premium reveals, scroll triggers, useMotionValue counters |
| Lenis | **1.3.23** | 1.3.23 | Apple-grade smooth scroll; previously `@studio-freight/lenis` |
| Geist Sans + Mono | **1.7.0** | 1.7.0 | Vercel official font, Next.js native integration |
| shadcn/ui | **4.7.0** (optional) | 4.7.0 | Copy-paste pattern only; no full install |
| Hosting | Vercel | — | Next.js native; deploy via vercel.com/new GitHub integration |

### Key empirical findings

- **Tailwind v4 is now stable** (`latest = 4.2.4`, v3 demoted to `v3-lts`). v4's CSS-first config (`@theme` in CSS, no `tailwind.config.js`) is production-ready. **No fallback to v3 needed.**
- **Vercel CLI not installed** locally. Deployment will go through the Vercel dashboard (`vercel.com/new` → GitHub repo → auto-detect Next.js). No `vercel deploy` from terminal needed — keeps the deploy path low-friction.
- **Next.js 16** is the latest major (knowledge cutoff was Aug 2025; current date is May 2026). React 19 baseline.

---

## Design Tokens

### Colors

```
Background:    #050507  (deepest, almost black)
Surface:       #0D0D12  (cards, slightly lifted)
Border:        #1A1A24  (subtle separation)

Accent:        #00FF94  (signature — "blocked / safe / passed")
Accent muted:  rgba(0, 255, 148, 0.10)  (glow halos)
Accent dim:    rgba(0, 255, 148, 0.50)  (active state)
Danger:        #FF2D55  (rare use — only "ATTACK" labels)

Text:          #F5F5F7  (primary)
Text muted:    #8E8E93  (secondary)
Text dim:      #525258  (footnotes, metadata, tx hashes background)
```

### Typography

- **Display:** Geist Sans, 6xl–7xl (60–84px desktop), tight tracking (`-0.04em`), `font-medium` (weight 500)
- **Body:** Geist Sans, base (16px), `leading-relaxed` (1.625)
- **Mono:** Geist Mono — code, tx hashes, contract addresses, stats numerals
- **Numerals:** `font-variant-numeric: tabular-nums` (counter alignment)

### Spacing

- **Section padding:** 120–200px vertical
- **Container max-w:** 1200px (`max-w-screen-xl` ≈ 1280px also acceptable)
- **Grid gap:** 24–48px

### Animations

- **Smooth scroll:** Lenis (~1.2s ease-out-expo, lerp 0.1)
- **Reveal:** Framer Motion `whileInView`, opacity 0→1 + y 20→0, stagger 100ms children
- **Counter-up:** Framer `useMotionValue` + spring (stiffness 100, damping 30)
- **Marquee:** Pure CSS infinite linear scroll (40s loop)
- **Hover:** Border glow + 1px lift, ~200ms ease-out — **no scale transforms, no rotate**

---

## Page Structure (Single Page, Anchored Sections)

```
/                                                    (single page)
├── #hero          "Trust the oracle. Verify the integrator."
├── #attack        "The Attack" — YieldBlox $10.2M post-mortem
├── #solution      "The Solution" — 5 guards summary
├── #how-it-works  "How It Works" — Yiling-style numbered steps
├── #architecture  "Architecture" — text-based diagram
├── #live          "Live on Stellar" — real tx hashes + contract addresses
├── #audit         "Adversarial Review" — counter-up severity table
└── #footer        Minimal — GitHub, docs, license
```

---

## Hero Spec

```
[Top small slogan]   Eight lines. Five guards. Zero exploits.

[Big headline]       Trust the oracle.
                     Verify the integrator.

[Subline]            Five mathematically-verified guardrails between
                     your protocol and the next oracle manipulation attack.

[Code snippet]       cargo add safe-oracle      [copy button]

[CTAs]               [GitHub] [Read the docs]

[Stats row]          $10.2M    │ 290     │ 0           │ 5
                     Drained   │ Tests   │ Critical    │ Guards
                     YieldBlox │ Passing │ Findings    │ Active
```

---

## Animation Map

| Element | Animation |
|---------|-----------|
| Hero slogan | Word-by-word fade-in stagger (100ms) |
| Hero CTA | Subtle border glow on mount (~600ms ease-out) |
| Stats counter | `useMotionValue` spring 0→target on `whileInView` |
| Marquee guardrails | Pure CSS infinite scroll (40s linear loop) |
| Section reveal | `whileInView` opacity 0→1 + y 20→0, stagger children 100ms |
| Code typewriter | Per-character reveal, 30ms each |
| Hover (cards) | Border glow + 1px lift, 200ms ease-out |
| Adversarial replay | Custom 4-stage scrubber (Phase 8.3) |

---

## Live Data Sources

All numbers and addresses pulled from `deployment/testnet.json` at Phase 7 closure (commit 4215fb9):

### Contracts

```
LiquidityRegistry:  CCDWMKL54WC3525IJA2UNRCRLTIROHWVVPK3MBU2YO4EMASLRB6WWGND
mock-lending:       CA6TBUXTIQKHD4VZ3MMQTJTTREMHHYQD4G6R3OTOOVGHOGQNXUYSMXZV
mock-reflector:     CBUPTLPDDNCB2OHTGTHD3DKHLGSZUDUMINU5OKU4CG5ZRHW5T7ATPHO7
```

### E2E Validation Tx Hashes (5 stages)

```
Phase 7.7 first submission:    cf4ecc2805c4355cd319b61dfc09aed719229c5b2c1ad5b804bc2f2099553c36  (ledger 2,448,975)
Phase 7.8 successful borrow:   ce4812031daa61ecb987c45123fbaba52eb83fe0b27f623dd3fa3fa0ec8a5c45  (ledger 2,450,314)
Phase 7.9 attack set_price:    b99d61340c63748394f27a589ac228bbc6a02aba7d74c5b50b67a416ee6acfb6
Phase 7.9 adversarial borrow:  a1cfdec1fe8f6c778c0f6f48f481c0b7dfd31ea7322834d84944459ca80a7653  (rejected: Failed:1)
Phase 7.9 recovery set_price:  9cae263874ab308ccba3871bc00aeec95dbff0199e2e7187c71c1ecf1bba378f
Phase 7.9 post-recovery borrow:5f5d06e4822e3a1b061513acfd159958105e2a7f7f1ab15a5337fa8ab10aec55
```

### Site Stats

- **17** consecutive `oracle-watch` submissions (Phase 7.7)
- **290** tests passing
- **0** critical findings (AR.H)
- **0** high findings
- **3** medium findings (all closed)
- **5** Layer 1 + Layer 2 guardrails active
- **6** live testnet bug fixes during Phase 7

---

## What This Site WILL NOT Have

- Generic gradients (purple→pink, orange→red)
- Stock 3D illustrations
- Emoji headers
- "Powered by" badges
- Carousel/slider components
- Modal popups (except optional cookie banner if legally required)
- Excessive hover transformations (no `scale 1.05`, no rotate)
- Loading skeletons (use Suspense fallback or skip entirely)
- Confetti, particle systems (a defensive radar sweep is the one exception, subtle)

## What This Site WILL Have

- Real tx hashes (not mocked, copy-able)
- Real contract addresses (linked to stellar.expert)
- Honest threshold note (testnet config relaxed for demo)
- Numbered, audited claims (290 tests, 0 critical, 17 submissions)
- Public links to GitHub, audit references, deployment artifact
- Minimal footer (GitHub, docs, license, that's it)

---

## Phase 8 Sub-Phase Plan

| Sub-phase | Scope | Estimated time |
|-----------|-------|----------------|
| **8.0** | Pre-phase discovery (THIS) | 30–45 min |
| **8.1** | Bootstrap + Hero (Next.js init, layout, hero, marquee, stats, first Vercel deploy) | 3–4 h |
| **8.2** | Content sections (Attack, Solution, How It Works, Architecture, Live, Audit, Footer) | 5–6 h |
| **8.3** | Polish + production (mobile, performance, attack replay animation, prod deploy, tag) | 3–4 h |

**Total:** ~12–15 hours.

---

## References

- **noether.exchange** — premium dark, yellow accent (we use green), screenshot frames, marquee chains
- **yiling-protocol-landing.vercel.app** — academic tone, numbered steps, math equations, infrastructure grid

---

## Source-of-Truth Pointer

This document is referenced by all subsequent Phase 8 implementation prompts. When a visual / tech decision is in doubt, this file wins. Updates require a doc-only commit and a brief rationale in the message.
