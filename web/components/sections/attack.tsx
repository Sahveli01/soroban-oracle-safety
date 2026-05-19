"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const PILLS = ["Thin Liquidity", "No Deviation Guard", "No Volume Check"];

const EASE: [number, number, number, number] = [0.19, 1, 0.22, 1];

export function Attack() {
  return (
    <SectionShell id="attack" eyebrow="The Attack">
      {/* Dramatic stat-led composition — the number is the hero */}
      <motion.div
        initial={{ opacity: 0, y: 24 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.8, ease: EASE }}
      >
        <div className="font-mono text-sm uppercase tracking-[0.25em] text-text-muted">
          One $5 trade
        </div>
        <h2 className="mt-4 flex items-end gap-4">
          <span className="text-danger text-[clamp(3rem,11vw,7rem)] font-semibold leading-[0.85] tracking-tight">
            $10.2M
          </span>
          <span className="mb-1.5 text-xl font-medium text-text md:mb-2.5 md:text-3xl">
            drained.
          </span>
        </h2>
      </motion.div>

      <div className="mt-14 grid gap-10 md:grid-cols-12">
        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true, margin: "-100px" }}
          transition={{ delay: 0.2, duration: 0.7 }}
          className="space-y-5 text-lg leading-relaxed text-text-muted md:col-span-7"
        >
          <p>
            On February 22, 2026, an attacker manipulated a thin SDEX market
            with a single $5 trade to inflate collateral valuation on a Stellar
            lending protocol. They walked away with $10.2 million.
          </p>
          <p>
            Reflector worked. Stellar worked. Blend V2 worked. The oracle
            reported the price it observed. The protocol trusted it.
          </p>
          <p className="text-text">
            The gap was integrator-side.{" "}
            <span className="font-mono text-accent">safe-oracle</span> closes
            that gap.
          </p>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, x: 10 }}
          whileInView={{ opacity: 1, x: 0 }}
          viewport={{ once: true, margin: "-100px" }}
          transition={{ delay: 0.35, duration: 0.7 }}
          className="md:col-span-5 md:border-l md:border-border md:pl-10"
        >
          <div className="font-mono text-xs uppercase tracking-[0.2em] text-text-dim">
            What was missing
          </div>
          <div className="mt-5 flex flex-col gap-2.5">
            {PILLS.map((pill) => (
              <span
                key={pill}
                className="rounded-lg border border-border bg-surface px-4 py-3 font-mono text-sm text-text-muted"
              >
                {pill}
              </span>
            ))}
          </div>
        </motion.div>
      </div>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-50px" }}
        transition={{ delay: 0.5, duration: 0.6 }}
        className="mt-10 font-mono text-xs text-text-dim"
      >
        Sources:{" "}
        <a
          href="https://rekt.news/yieldblox-rekt"
          target="_blank"
          rel="noopener noreferrer"
          className="underline decoration-dotted underline-offset-4 transition-colors hover:text-accent"
        >
          Rekt News
        </a>
        {" · "}
        <a
          href="https://www.halborn.com/blog/post/explained-the-yieldblox-hack-february-2026"
          target="_blank"
          rel="noopener noreferrer"
          className="underline decoration-dotted underline-offset-4 transition-colors hover:text-accent"
        >
          Halborn analysis
        </a>
        {" · "}
        <a
          href="https://x.com/script3official/status/2025403423840141450"
          target="_blank"
          rel="noopener noreferrer"
          className="underline decoration-dotted underline-offset-4 transition-colors hover:text-accent"
        >
          Script3 official statement
        </a>
      </motion.p>
    </SectionShell>
  );
}
