"use client";

import { motion } from "framer-motion";

export function Footer() {
  return (
    <footer className="border-t border-border px-6 py-16">
      <div className="mx-auto max-w-5xl">
        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="flex flex-col items-start justify-between gap-8 md:flex-row md:items-end"
        >
          <div>
            <div className="font-mono text-2xl font-medium tracking-tight">
              safe-oracle
            </div>
            <p className="mt-2 max-w-md text-sm text-text-muted">
              Drop-in oracle protection for Stellar Soroban. Open source.
              Apache-2.0.
            </p>
          </div>

          <div className="flex flex-wrap gap-6 font-mono text-sm text-text-muted">
            <a
              href="https://github.com/Sahveli01/soroban-oracle-safety"
              target="_blank"
              rel="noopener noreferrer"
              className="transition-colors hover:text-text"
            >
              GitHub ↗
            </a>
            <a
              href="https://github.com/Sahveli01/soroban-oracle-safety/blob/main/DEPLOYMENT.md"
              target="_blank"
              rel="noopener noreferrer"
              className="transition-colors hover:text-text"
            >
              Docs ↗
            </a>
            <a
              href="https://stellar.org/soroban"
              target="_blank"
              rel="noopener noreferrer"
              className="transition-colors hover:text-text"
            >
              Stellar Soroban ↗
            </a>
          </div>
        </motion.div>

        <div className="mt-12 border-t border-border pt-6 font-mono text-xs text-text-dim">
          Built for Stellar Soroban · © 2026
        </div>
      </div>
    </footer>
  );
}
