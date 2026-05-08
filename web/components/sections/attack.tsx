"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const PILLS = ["Thin Liquidity", "No Deviation Guard", "No Volume Check"];

export function Attack() {
  return (
    <SectionShell id="attack" eyebrow="The Attack">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7, ease: [0.19, 1, 0.22, 1] }}
        className="text-4xl font-medium leading-[1.1] tracking-tight sm:text-5xl md:text-6xl lg:text-7xl"
      >
        $5 trade.
        <br />
        <span className="text-danger">$10.2M drained.</span>
      </motion.h2>

      <motion.div
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.2, duration: 0.7 }}
        className="mt-12 max-w-2xl space-y-6 text-lg leading-relaxed text-text-muted"
      >
        <p>
          On February 22, 2026, an attacker manipulated a thin SDEX market with
          a single $5 trade to inflate collateral valuation on a Stellar lending
          protocol. They walked away with $10.2 million.
        </p>
        <p>
          Reflector worked. Stellar worked. Blend V2 worked. The oracle reported
          the price it observed. The protocol trusted it.
        </p>
        <p className="text-text">
          The gap was integrator-side.{" "}
          <span className="font-mono text-accent">safe-oracle</span> closes that
          gap.
        </p>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 10 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.4, duration: 0.6 }}
        className="mt-10 flex flex-wrap gap-3"
      >
        {PILLS.map((pill) => (
          <span
            key={pill}
            className="rounded-full border border-border bg-surface px-4 py-2 font-mono text-xs uppercase tracking-wider text-text-muted"
          >
            {pill}
          </span>
        ))}
      </motion.div>
    </SectionShell>
  );
}
