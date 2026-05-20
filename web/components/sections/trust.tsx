"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const REPO = "https://github.com/Sahveli01/soroban-oracle-safety";

/**
 * Self-audit dashboard — replaces the minimal pill grid that read as
 * "barren". Surfaces the AR.H adversarial-review data the project has
 * actually accumulated, while never claiming third-party attestation
 * (we have none).
 *
 * All numbers below are EMPIRICALLY sourced and verifiable in
 * README.md#adversarial-review:
 * - Scenarios tested  — AR.H executive summary ("20+ distinct attack
 *                       vectors across all five guardrails").
 * - Critical / High   — README Adversarial Review table: 0 / 0.
 * - MEDIUM / LOW      — README Adversarial Review: 3 medium and 5 low,
 *                       all closed. Notable closures referenced
 *                       in-code with `AR.H {id} fix:` doc-comments.
 * - Last reviewed     — AR.H written against commit 0dda2fd
 *                       (2026-05-06); closures landed through v0.2.0.
 * - Tests passing     — Workspace cargo test --workspace == 310.
 */
const AUDIT = {
  scenariosTested: 20,
  criticalFindings: 0,
  highFindings: 0,
  mediumTotal: 3,
  mediumClosed: 3,
  lowTotal: 5,
  lowClosed: 5,
  lastReviewDate: "2026-05-06",
  reviewCommit: "0dda2fd",
};

const METADATA = {
  version: "v0.2.0",
  license: "Apache-2.0",
  testCount: 310,
  author: "@Sahveli01",
  repo: "soroban-oracle-safety",
};

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
        Internal adversarial replay review: every guardrail attacked, every
        finding catalogued, every patch named in git. Empirical evidence over
        external attestation.
      </motion.p>

      {/* Primary audit metrics — three load-bearing cards */}
      <div className="mt-8 grid gap-4 md:grid-cols-3">
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-50px" }}
          transition={{ delay: 0.08, duration: 0.5, ease: EASE }}
          className="surface-card p-5"
        >
          <div className="font-mono text-[11px] uppercase tracking-[0.18em] text-text-dim">
            Scenarios attacked
          </div>
          <div className="mt-3 font-mono text-4xl font-medium text-accent tabular-nums">
            {AUDIT.scenariosTested}
          </div>
          <div className="mt-2 text-sm text-text-muted">
            Per-guardrail adversarial sweep (AR.H)
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 10 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-50px" }}
          transition={{ delay: 0.14, duration: 0.5, ease: EASE }}
          className="surface-card p-5"
        >
          <div className="font-mono text-[11px] uppercase tracking-[0.18em] text-text-dim">
            Critical / High
          </div>
          <div className="mt-3 font-mono text-4xl font-medium text-accent tabular-nums">
            {AUDIT.criticalFindings} / {AUDIT.highFindings}
          </div>
          <div className="mt-2 text-sm text-text-muted">
            Last reviewed {AUDIT.lastReviewDate} · {AUDIT.reviewCommit}
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 10 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-50px" }}
          transition={{ delay: 0.2, duration: 0.5, ease: EASE }}
          className="surface-card p-5"
        >
          <div className="font-mono text-[11px] uppercase tracking-[0.18em] text-text-dim">
            Medium / Low closed
          </div>
          <div className="mt-3 font-mono text-4xl font-medium text-accent tabular-nums">
            {AUDIT.mediumClosed + AUDIT.lowClosed}/
            {AUDIT.mediumTotal + AUDIT.lowTotal}
          </div>
          <div className="mt-2 text-sm text-text-muted">
            All MEDIUMs and LOWs closed in code (M1, M2, M3, L1–L5)
          </div>
        </motion.div>
      </div>

      {/* Metadata strip — five compact verifiable pills */}
      <div className="mt-6 grid gap-2 sm:grid-cols-2 md:grid-cols-5">
        {[
          {
            label: "Version",
            value: METADATA.version,
            href: "https://crates.io/crates/safe-oracle",
          },
          {
            label: "License",
            value: METADATA.license,
            href: `${REPO}/blob/main/LICENSE`,
          },
          {
            label: "Tests",
            value: `${METADATA.testCount} passing`,
            href: `${REPO}/actions`,
          },
          {
            label: "Author",
            value: METADATA.author,
            href: "https://github.com/Sahveli01",
          },
          {
            label: "Repository",
            value: METADATA.repo,
            href: REPO,
          },
        ].map((item, i) => (
          <motion.a
            key={item.label}
            href={item.href}
            target="_blank"
            rel="noopener noreferrer"
            initial={{ opacity: 0, y: 8 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{ delay: 0.26 + i * 0.04, duration: 0.4, ease: EASE }}
            className="surface-card block p-3.5"
          >
            <div className="font-mono text-[10px] uppercase tracking-[0.16em] text-text-dim">
              {item.label}
            </div>
            <div className="mt-1 font-mono text-sm text-text">{item.value}</div>
          </motion.a>
        ))}
      </div>

      {/* Honesty footer — non-negotiable: prevents the dashboard from
          being misread as third-party attestation. */}
      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-50px" }}
        transition={{ delay: 0.5, duration: 0.5 }}
        className="mt-7 max-w-3xl text-sm leading-relaxed text-text-dim"
      >
        This is a self-conducted adversarial review — not a third-party audit.
        Findings, severity table, and closures are public in the repository
        (
        <a
          href={`${REPO}#adversarial-review`}
          target="_blank"
          rel="noopener noreferrer"
          className="text-text-muted underline-offset-2 hover:text-accent hover:underline"
        >
          README · Adversarial Review
        </a>
        ); each patch is annotated in-code with its AR.H finding ID. External
        audit is recommended before mainnet deployment with material funds.
      </motion.p>
    </SectionShell>
  );
}
