"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const REPO = "https://github.com/Sahveli01/soroban-oracle-safety";

/**
 * Trust strip — engineering metadata, not a marketing band.
 *
 * Surfaces the live, verifiable facts a developer needs before
 * depending on the crate: version, licence, test count, author, repo.
 * Reuses the `.surface-card` language; introduces no new visual system.
 */
const TRUST_ITEMS = [
  { label: "Version", value: "v0.2.0", href: "https://crates.io/crates/safe-oracle" },
  { label: "License", value: "Apache-2.0", href: `${REPO}/blob/main/LICENSE` },
  { label: "Tests", value: "310 passing", href: `${REPO}/actions` },
  { label: "Author", value: "@Sahveli01", href: "https://github.com/Sahveli01" },
  { label: "Repository", value: "soroban-oracle-safety", href: REPO },
];

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

export function Trust() {
  return (
    <SectionShell id="trust" eyebrow="Trust">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.6, ease: EASE }}
        className="t-h2 max-w-3xl"
      >
        Engineering metadata.
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.15, duration: 0.6 }}
        className="mt-6 max-w-2xl text-lg leading-relaxed text-text-muted"
      >
        Live, verifiable, and current. No badges to take on faith — every
        value links to its source.
      </motion.p>

      <div className="mt-12 grid gap-3 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5">
        {TRUST_ITEMS.map((item, i) => (
          <motion.a
            key={item.label}
            href={item.href}
            target="_blank"
            rel="noopener noreferrer"
            initial={{ opacity: 0, y: 10 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{ delay: i * 0.05, duration: 0.4, ease: EASE }}
            className="surface-card block p-5"
          >
            <div className="font-mono text-xs uppercase tracking-wider text-text-dim">
              {item.label}
            </div>
            <div className="mt-2 font-mono text-sm text-text">{item.value}</div>
          </motion.a>
        ))}
      </div>
    </SectionShell>
  );
}
