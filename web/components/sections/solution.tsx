"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const GUARDS = [
  { layer: "Layer 1", name: "Deviation", catch: "Sudden price spikes", mode: "default" },
  { layer: "Layer 1", name: "Staleness", catch: "Outdated feeds", mode: "default" },
  { layer: "Layer 1", name: "Cross-Source", catch: "Oracle disagreement", mode: "default" },
  { layer: "Layer 2", name: "Liquidity", catch: "Thin SDEX volume", mode: "opt-in" },
  { layer: "Layer 2", name: "Thin Sampling", catch: "Low trader count", mode: "opt-in" },
];

export function Solution() {
  return (
    <SectionShell id="solution" eyebrow="The Solution" density="dense">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7, ease: [0.19, 1, 0.22, 1] }}
        className="t-h1"
      >
        Three trustless by default.
        <br />
        <span className="text-accent">Two more, opt-in.</span>
      </motion.h2>

      <motion.p
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.2, duration: 0.7 }}
        className="mt-7 max-w-2xl text-lg leading-relaxed text-text-muted"
      >
        Layer 1 runs on pure on-chain Reflector math — no off-chain trust.
        Layer 2 adds market-microstructure defense, opt-in because it relies
        on attested liquidity snapshots. Each guard closes a specific attack
        vector seen in real DeFi exploits.
      </motion.p>

      <div className="mt-9 divide-y divide-border border-y border-border">
        {GUARDS.map((guard, i) => (
          <motion.div
            key={guard.name}
            initial={{ opacity: 0, x: -10 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{
              delay: i * 0.08,
              duration: 0.5,
              ease: [0.19, 1, 0.22, 1],
            }}
            className="grid grid-cols-12 gap-4 py-4"
          >
            <div className="col-span-3 font-mono text-xs uppercase tracking-wider text-text-dim md:col-span-2">
              {guard.layer}
            </div>
            <div className="col-span-9 flex items-center gap-3 md:col-span-4">
              <span className="text-2xl font-medium">{guard.name}</span>
              <span
                className={`shrink-0 rounded-full border px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider ${
                  guard.mode === "default"
                    ? "border-accent/40 text-accent"
                    : "border-border text-text-dim"
                }`}
              >
                {guard.mode}
              </span>
            </div>
            <div className="col-span-12 text-text-muted md:col-span-6">
              {guard.catch}
            </div>
          </motion.div>
        ))}
      </div>

      <motion.div
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ delay: 0.6, duration: 0.5 }}
        className="mt-6 flex items-center gap-3 text-sm text-text-muted"
      >
        <span className="inline-block h-px w-12 bg-accent" />
        <span className="font-mono">
          Plus: Circuit Breaker — auto-halt on first violation
        </span>
      </motion.div>
    </SectionShell>
  );
}
