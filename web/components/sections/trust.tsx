"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const REPO = "https://github.com/Sahveli01/soroban-oracle-safety";

/**
 * Trust — text-driven prose, not a big-number dashboard.
 *
 * Pass 3 simplified Trust to three large green numbers (20 / 0 / 0).
 * User feedback: 'çok çocukça gözüküyor' — it read as a video-game
 * score board, not a premium engineering trust page.
 *
 * Pass 4A: rebuilt following Stripe / Linear / Resend / Anthropic
 * trust-page patterns. The shift is from "look at our score" to "here
 * is what we did, in our own words, with caveats". Numbers are still
 * present (and unchanged: 20 scenarios, 0 critical, 0 high, 310
 * tests) but integrated into sentences instead of standing alone as
 * card-sized digits — that is the difference between sophistication
 * and maximalism.
 *
 * Numerical accuracy verified against README.md#adversarial-review.
 */

const METADATA = [
  {
    label: "Version",
    value: "v0.2.0",
    href: "https://crates.io/crates/safe-oracle",
  },
  {
    label: "License",
    value: "Apache-2.0",
    href: `${REPO}/blob/main/LICENSE`,
  },
  {
    label: "Test Coverage",
    value: "310 passing",
    href: `${REPO}/actions`,
  },
  {
    label: "Author",
    value: "@Sahveli01",
    href: "https://github.com/Sahveli01",
  },
  {
    label: "Repository",
    value: "soroban-oracle-safety",
    href: REPO,
  },
  {
    label: "Network",
    value: "Stellar Soroban",
    // Static — no link, balances the 3-col grid as the 6th cell.
  },
];

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

export function Trust() {
  return (
    <SectionShell id="trust" eyebrow="Trust" density="dense">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.6, ease: EASE }}
        className="t-h1 max-w-4xl"
      >
        Honest about what we are.
        <br />
        <span className="text-accent">And what we are not.</span>
      </motion.h2>

      <motion.p
        initial={{ opacity: 0, y: 10 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-80px" }}
        transition={{ delay: 0.12, duration: 0.6, ease: EASE }}
        className="mt-7 max-w-3xl text-lg leading-relaxed text-text-muted"
      >
        safe-oracle has been subjected to an internal adversarial replay
        review across{" "}
        <span className="font-medium text-text">20 attack scenarios</span> —
        covering deviation manipulation, staleness, cross-source disagreement,
        liquidity-floor evasion, and circuit-breaker bypass. No critical or
        high findings remain open.
      </motion.p>

      <motion.p
        initial={{ opacity: 0, y: 10 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-80px" }}
        transition={{ delay: 0.2, duration: 0.6, ease: EASE }}
        className="mt-5 max-w-3xl text-lg leading-relaxed text-text-muted"
      >
        This is not a third-party audit. The methodology, the findings, and
        the patches are{" "}
        <a
          href={`${REPO}#adversarial-review`}
          target="_blank"
          rel="noopener noreferrer"
          className="text-accent underline decoration-dotted underline-offset-4 transition hover:decoration-solid"
        >
          public in the repository
        </a>
        . External audit is recommended before mainnet deployment with
        material funds.
      </motion.p>

      {/* Metadata — inline key/value pairs, border-separated. NOT a
          surface-card grid: cards add weight and read as dashboard
          tiles. The hairline separator above + pure-typography pairs
          read as document chrome, which is the premium tell. */}
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-50px" }}
        transition={{ delay: 0.3, duration: 0.5, ease: EASE }}
        className="mt-12 grid max-w-4xl grid-cols-1 gap-x-12 gap-y-5 border-t border-border pt-8 sm:grid-cols-2 md:grid-cols-3"
      >
        {METADATA.map((m) => (
          <div key={m.label}>
            <div className="font-mono text-[11px] uppercase tracking-[0.2em] text-text-dim">
              {m.label}
            </div>
            {m.href ? (
              <a
                href={m.href}
                target="_blank"
                rel="noopener noreferrer"
                className="mt-1.5 block font-mono text-sm text-text transition-colors hover:text-accent"
              >
                {m.value}
              </a>
            ) : (
              <div className="mt-1.5 font-mono text-sm text-text">
                {m.value}
              </div>
            )}
          </div>
        ))}
      </motion.div>
    </SectionShell>
  );
}
