"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const REPO = "https://github.com/Sahveli01/soroban-oracle-safety";

/**
 * Proof-at-a-glance. User feedback on the pass-2 dashboard: 'çok teknik
 * bulguların olmadığı gösterilmeli yok kapandı yok şöyle böyle şeklinde
 * olmamalı.' Strip everything that reads as audit-report (M1/M2/M3,
 * commit hashes, AR.H sub-labels, hardness grade) and leave a clean
 * three-card statement: scenarios attacked, critical, high.
 *
 * Numbers verified against README.md#adversarial-review:
 *   20 scenarios — AR.H executive summary
 *    0 critical  — AR.H Findings section
 *    0 high      — AR.H Findings section
 */
const PROOF = [
  { label: "Scenarios Attacked", value: "20" },
  { label: "Critical Findings", value: "0" },
  { label: "High Findings", value: "0" },
];

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
    label: "Tests",
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
        className="t-h1 max-w-3xl"
      >
        Self-audited.{" "}
        <span className="text-accent">No external badge.</span>
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.12, duration: 0.6 }}
        className="mt-5 max-w-2xl text-lg leading-relaxed text-text-muted"
      >
        Internal adversarial replay review. Public, verifiable, honest.
      </motion.p>

      {/* Three proof cards — just the numbers, no audit-report detail */}
      <div className="mt-10 grid gap-5 md:grid-cols-3">
        {PROOF.map((p, i) => (
          <motion.div
            key={p.label}
            initial={{ opacity: 0, y: 10 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{ delay: 0.1 + i * 0.06, duration: 0.5, ease: EASE }}
            className="surface-card p-7 text-center"
          >
            <div className="font-mono text-[11px] uppercase tracking-[0.2em] text-text-muted">
              {p.label}
            </div>
            <div className="mt-4 font-mono text-5xl font-medium text-accent tabular-nums">
              {p.value}
            </div>
          </motion.div>
        ))}
      </div>

      {/* Verifiable metadata — five compact pills */}
      <div className="mt-6 grid gap-2 sm:grid-cols-2 md:grid-cols-5">
        {METADATA.map((m, i) => (
          <motion.a
            key={m.label}
            href={m.href}
            target="_blank"
            rel="noopener noreferrer"
            initial={{ opacity: 0, y: 8 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{ delay: 0.3 + i * 0.04, duration: 0.4, ease: EASE }}
            className="surface-card block p-3.5"
          >
            <div className="font-mono text-[10px] uppercase tracking-[0.16em] text-text-dim">
              {m.label}
            </div>
            <div className="mt-1 font-mono text-sm text-text">{m.value}</div>
          </motion.a>
        ))}
      </div>

      {/* Honesty footer — non-negotiable, mottomuza uyum.
          Prevents the dashboard from being misread as third-party
          attestation. Methodology link points to the public README
          section, not the un-tracked AR_H_REVIEW.md. */}
      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-50px" }}
        transition={{ delay: 0.55, duration: 0.5 }}
        className="mt-7 max-w-3xl text-sm leading-relaxed text-text-dim"
      >
        Self-conducted adversarial review — not a third-party audit. Findings
        and methodology are public in the{" "}
        <a
          href={`${REPO}#adversarial-review`}
          target="_blank"
          rel="noopener noreferrer"
          className="text-text-muted underline-offset-2 hover:text-accent hover:underline"
        >
          repository
        </a>
        . External audit recommended before mainnet deployment with material
        funds.
      </motion.p>
    </SectionShell>
  );
}
