"use client";

import { motion } from "framer-motion";

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

/**
 * Closing panel — the final page of the same stacked page-turn.
 *
 * It is a `sticky top-0` sibling like every section, so it slides up
 * and covers the last content panel exactly like every other turn (the
 * end no longer reads as "two separate pages"). `.page-foot` gives it a
 * flush, seamless leading edge (no rounded lip) so the document resolves
 * into one continuous closing surface instead of an orphaned strip.
 */
export function Footer() {
  return (
    <footer className="page-foot sticky top-0 bg-[var(--color-background)]">
      <div className="screen-min mx-auto flex w-full max-w-5xl flex-col justify-between px-6 py-[clamp(4rem,9vh,7rem)]">
        {/* Big closing wordmark */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.7, ease: EASE }}
          className="flex flex-1 flex-col justify-center"
        >
          <div className="t-h1 font-mono tracking-tight">
            safe-oracle
          </div>
          <p className="mt-5 max-w-md text-lg leading-relaxed text-text-muted">
            Drop-in oracle protection for Stellar Soroban.
            <br />
            Open source. Apache-2.0.
          </p>

          <div className="mt-10 flex flex-wrap gap-x-8 gap-y-3 font-mono text-sm">
            <a
              href="https://github.com/Sahveli01/soroban-oracle-safety"
              target="_blank"
              rel="noopener noreferrer"
              className="link-sweep text-text transition-colors hover:text-accent"
            >
              GitHub ↗
            </a>
            <a
              href="https://github.com/Sahveli01/soroban-oracle-safety/blob/main/DEPLOYMENT.md"
              target="_blank"
              rel="noopener noreferrer"
              className="link-sweep text-text transition-colors hover:text-accent"
            >
              Docs ↗
            </a>
            <a
              href="https://stellar.org/soroban"
              target="_blank"
              rel="noopener noreferrer"
              className="link-sweep text-text transition-colors hover:text-accent"
            >
              Stellar Soroban ↗
            </a>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ delay: 0.2, duration: 0.6 }}
          className="mt-16 flex flex-col gap-3 border-t border-border pt-6 font-mono text-xs text-text-dim sm:flex-row sm:items-center sm:justify-between"
        >
          <span>Built for Stellar Soroban · © 2026</span>
          <span className="text-text-muted">
            Trust the oracle. Verify the integrator.
          </span>
        </motion.div>
      </div>
    </footer>
  );
}
