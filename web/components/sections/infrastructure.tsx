"use client";

import { motion } from "framer-motion";
import { SectionShell } from "./section-shell";

const CARDS = [
  {
    title: "Layer 1 Guardrails",
    desc: "Deviation, staleness, cross-source. Validates oracle output before it reaches your logic.",
  },
  {
    title: "Layer 2 Guardrails",
    desc: "Liquidity volume + thin sampling. Validates market microstructure on-chain.",
  },
  {
    title: "Circuit Breaker",
    desc: "Auto-halt on first violation. Per-asset isolation. Manual governance override.",
  },
  {
    title: "Liquidity Registry",
    desc: "Signed snapshots from off-chain attesters. Authoritative source for Layer 2 checks.",
  },
  {
    title: "oracle-watch",
    desc: "Off-chain Rust service. Monitors SDEX, signs snapshots, dispatches Discord/Telegram alerts.",
  },
  {
    title: "Soroban-Native",
    desc: "Built for Stellar Soroban 25. WASM contract + reqwest off-chain. No bridges.",
  },
];

export function Infrastructure() {
  return (
    <SectionShell id="infrastructure" eyebrow="Infrastructure">
      <motion.h2
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, margin: "-100px" }}
        transition={{ duration: 0.7 }}
        className="text-5xl font-medium leading-[1.1] tracking-tight md:text-6xl"
      >
        Modular by design.
      </motion.h2>

      <div className="mt-16 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {CARDS.map((card, i) => (
          <motion.div
            key={card.title}
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-50px" }}
            transition={{
              delay: i * 0.06,
              duration: 0.5,
              ease: [0.19, 1, 0.22, 1],
            }}
            className="group relative rounded-xl border border-border bg-surface p-6 transition-all hover:border-accent/40"
          >
            <h3 className="font-mono text-xs uppercase tracking-wider text-text-muted">
              {card.title}
            </h3>
            <p className="mt-4 text-text">{card.desc}</p>
          </motion.div>
        ))}
      </div>
    </SectionShell>
  );
}
